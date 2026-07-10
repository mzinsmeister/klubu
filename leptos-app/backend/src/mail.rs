//! Local SMTP/IMAP relay. It binds to localhost by default because these
//! protocol handlers are plaintext; expose them only behind a TLS-capable mail
//! proxy or a private network. All message writes use the app's archive API.

use app::server::email::{self, ImapMessage};
use base64::Engine;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};

const MAX_MESSAGE_BYTES: usize = 50 * 1024 * 1024;

fn bind_address() -> String {
    std::env::var("KLUBU_MAIL_BIND").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn path_value(value: &str) -> String {
    value
        .split_once('<')
        .and_then(|(_, rest)| rest.split_once('>').map(|(path, _)| path.to_string()))
        .unwrap_or_else(|| value.trim().to_string())
}

fn auth_plain(value: &str) -> Option<(String, String)> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .ok()?;
    let mut parts = decoded.split(|byte| *byte == 0);
    let _authzid = parts.next()?;
    let username = String::from_utf8(parts.next()?.to_vec()).ok()?;
    let password = String::from_utf8(parts.next()?.to_vec()).ok()?;
    Some((username, password))
}

fn auth_login_value(value: &str) -> Option<String> {
    String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(value.trim())
            .ok()?,
    )
    .ok()
}

async fn reply(writer: &mut WriteHalf<TcpStream>, text: &str) -> Result<(), String> {
    writer
        .write_all(text.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

async fn smtp_data(reader: &mut BufReader<ReadHalf<TcpStream>>) -> Result<Vec<u8>, String> {
    let mut raw = Vec::new();
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("SMTP-Verbindung während DATA beendet".to_string());
        }
        if line == ".\r\n" || line == ".\n" {
            break;
        }
        let line = line.strip_prefix("..").unwrap_or(&line);
        raw.extend_from_slice(line.as_bytes());
        if raw.len() > MAX_MESSAGE_BYTES {
            return Err("E-Mail überschreitet 50 MB".to_string());
        }
    }
    Ok(raw)
}

async fn smtp_client(stream: TcpStream, repo: Arc<app::db::SqlRepository>) -> Result<(), String> {
    let (read_half, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    reply(&mut writer, "220 klubu ESMTP GoBD archive relay\r\n").await?;
    let mut authenticated: Option<String> = None;
    let mut login_user: Option<String> = None;
    let mut envelope_from: Option<String> = None;
    let mut recipients = Vec::<String>::new();

    loop {
        let mut line = String::new();
        if reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        let command = line.trim_end_matches(['\r', '\n']);
        if let Some(username) = login_user.take() {
            let password = auth_login_value(command)
                .ok_or_else(|| "Ungültiges AUTH LOGIN Passwort".to_string())?;
            if email::authenticate_mail_user(&repo, &username, &password)
                .await
                .map_err(|e| e.to_string())?
            {
                authenticated = Some(username);
                reply(&mut writer, "235 2.7.0 Authentifizierung erfolgreich\r\n").await?;
            } else {
                reply(
                    &mut writer,
                    "535 5.7.8 Authentifizierung fehlgeschlagen\r\n",
                )
                .await?;
            }
            continue;
        }
        let mut parts = command.splitn(2, ' ');
        let verb = parts.next().unwrap_or_default().to_ascii_uppercase();
        let argument = parts.next().unwrap_or_default().trim();
        match verb.as_str() {
            "EHLO" => {
                reply(
                    &mut writer,
                    "250-klubu\r\n250-SIZE 52428800\r\n250-AUTH PLAIN LOGIN\r\n250 HELP\r\n",
                )
                .await?
            }
            "HELO" => reply(&mut writer, "250 klubu\r\n").await?,
            "NOOP" => reply(&mut writer, "250 2.0.0 OK\r\n").await?,
            "RSET" => {
                envelope_from = None;
                recipients.clear();
                reply(&mut writer, "250 2.0.0 Reset\r\n").await?;
            }
            "QUIT" => {
                reply(&mut writer, "221 2.0.0 Bye\r\n").await?;
                break;
            }
            "AUTH" if argument.to_ascii_uppercase().starts_with("PLAIN") => {
                let credentials = argument.split_once(' ').map(|(_, value)| value.trim());
                let (username, password) = match credentials.and_then(auth_plain) {
                    Some(credentials) => credentials,
                    None => {
                        reply(&mut writer, "334 \r\n").await?;
                        let mut encoded = String::new();
                        reader
                            .read_line(&mut encoded)
                            .await
                            .map_err(|e| e.to_string())?;
                        auth_plain(encoded.trim())
                            .ok_or_else(|| "Ungültige AUTH PLAIN Zugangsdaten".to_string())?
                    }
                };
                if email::authenticate_mail_user(&repo, &username, &password)
                    .await
                    .map_err(|e| e.to_string())?
                {
                    authenticated = Some(username);
                    reply(&mut writer, "235 2.7.0 Authentifizierung erfolgreich\r\n").await?;
                } else {
                    reply(
                        &mut writer,
                        "535 5.7.8 Authentifizierung fehlgeschlagen\r\n",
                    )
                    .await?;
                }
            }
            "AUTH" if argument.eq_ignore_ascii_case("LOGIN") => {
                reply(&mut writer, "334 VXNlcm5hbWU6\r\n").await?;
                let mut encoded = String::new();
                reader
                    .read_line(&mut encoded)
                    .await
                    .map_err(|e| e.to_string())?;
                login_user = Some(
                    auth_login_value(encoded.trim())
                        .ok_or_else(|| "Ungültiger Benutzername".to_string())?,
                );
                reply(&mut writer, "334 UGFzc3dvcmQ6\r\n").await?;
            }
            "MAIL" if argument.to_ascii_uppercase().starts_with("FROM:") => {
                envelope_from = Some(path_value(argument[5..].trim()));
                recipients.clear();
                reply(&mut writer, "250 2.1.0 Sender OK\r\n").await?;
            }
            "RCPT" if argument.to_ascii_uppercase().starts_with("TO:") => {
                let recipient = path_value(argument[3..].trim());
                if authenticated.is_none() {
                    let domain = email::settings().address_domain;
                    let local = recipient
                        .rsplit_once('@')
                        .map(|(_, value)| value.eq_ignore_ascii_case(&domain))
                        .unwrap_or(false);
                    if !local {
                        reply(
                            &mut writer,
                            "530 5.7.1 Authentication required for relay\r\n",
                        )
                        .await?;
                        continue;
                    }
                }
                recipients.push(recipient);
                reply(&mut writer, "250 2.1.5 Recipient OK\r\n").await?;
            }
            "DATA" => {
                if envelope_from.is_none() || recipients.is_empty() {
                    reply(
                        &mut writer,
                        "503 5.5.1 Need MAIL FROM and RCPT TO first\r\n",
                    )
                    .await?;
                    continue;
                }
                reply(&mut writer, "354 End data with <CR><LF>.<CR><LF>\r\n").await?;
                let raw = match smtp_data(&mut reader).await {
                    Ok(raw) => raw,
                    Err(error) => {
                        reply(&mut writer, &format!("552 5.3.4 {error}\r\n")).await?;
                        continue;
                    }
                };
                match email::receive_smtp_message(
                    &repo,
                    authenticated.as_deref(),
                    envelope_from.as_deref(),
                    &recipients,
                    &raw,
                )
                .await
                {
                    Ok(()) => {
                        reply(&mut writer, "250 2.0.0 Message archived and accepted\r\n").await?
                    }
                    Err(error) => reply(&mut writer, &format!("451 4.3.0 {error}\r\n")).await?,
                }
                envelope_from = None;
                recipients.clear();
            }
            _ => reply(&mut writer, "502 5.5.2 Command not implemented\r\n").await?,
        }
    }
    Ok(())
}

fn imap_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' && quoted {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character.is_whitespace() && !quoted {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn imap_flags(raw: &str) -> Vec<String> {
    raw.trim_matches(|character| character == '(' || character == ')')
        .split_whitespace()
        .filter(|flag| flag.starts_with('\\'))
        .map(str::to_string)
        .collect()
}

fn sequence_indexes(spec: &str, messages: &[ImapMessage], uid: bool) -> Vec<usize> {
    let mut result = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (start, end) = if let Some((from, to)) = part.split_once(':') {
            let start = from.parse::<i64>().unwrap_or(1);
            let end = if to == "*" {
                messages.len() as i64
            } else {
                to.parse::<i64>().unwrap_or(start)
            };
            (start, end)
        } else if part == "*" {
            (messages.len() as i64, messages.len() as i64)
        } else {
            let value = part.parse::<i64>().unwrap_or(0);
            (value, value)
        };
        for value in start.min(end)..=start.max(end) {
            if value < 1 {
                continue;
            }
            if uid {
                if let Some(index) = messages.iter().position(|message| message.id == value) {
                    result.push(index);
                }
            } else if (value as usize) <= messages.len() {
                result.push(value as usize - 1);
            }
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}

async fn fetch_message(
    writer: &mut WriteHalf<TcpStream>,
    sequence: usize,
    message: &ImapMessage,
) -> Result<(), String> {
    let flags = message.flags.join(" ");
    reply(
        writer,
        &format!(
            "* {sequence} FETCH (UID {} FLAGS ({flags}) RFC822.SIZE {} BODY[] {{{}\r\n",
            message.id,
            message.raw.len(),
            message.raw.len()
        ),
    )
    .await?;
    writer
        .write_all(&message.raw)
        .await
        .map_err(|e| e.to_string())?;
    reply(writer, ")\r\n").await
}

async fn imap_client_legacy(
    stream: TcpStream,
    repo: Arc<app::db::SqlRepository>,
) -> Result<(), String> {
    let (read_half, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    reply(
        &mut writer,
        "* OK klubu IMAP4rev1 GoBD archive relay ready\r\n",
    )
    .await?;
    let mut user: Option<String> = None;
    let mut selected: Option<(String, bool)> = None;

    loop {
        let mut line = String::new();
        if reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        let mut pieces = line.splitn(3, ' ');
        let tag = pieces.next().unwrap_or("*");
        let command = pieces.next().unwrap_or_default().to_ascii_uppercase();
        let rest = pieces.next().unwrap_or_default();
        let args = imap_args(rest);
        match command.as_str() {
            "CAPABILITY" => {
                reply(
                    &mut writer,
                    "* CAPABILITY IMAP4rev1 UIDPLUS AUTH=PLAIN AUTH=LOGIN\r\n",
                )
                .await?;
                reply(&mut writer, &format!("{tag} OK CAPABILITY completed\r\n")).await?;
            }
            "NOOP" | "CHECK" => {
                reply(&mut writer, &format!("{tag} OK {command} completed\r\n")).await?
            }
            "ID" => {
                reply(&mut writer, "* ID NIL\r\n").await?;
                reply(&mut writer, &format!("{tag} OK ID completed\r\n")).await?;
            }
            "LOGIN" => {
                if args.len() < 2 {
                    reply(
                        &mut writer,
                        &format!("{tag} BAD LOGIN requires user and password\r\n"),
                    )
                    .await?;
                    continue;
                }
                if email::authenticate_mail_user(&repo, &args[0], &args[1])
                    .await
                    .map_err(|e| e.to_string())?
                {
                    user = Some(args[0].clone());
                    reply(&mut writer, &format!("{tag} OK LOGIN completed\r\n")).await?;
                } else {
                    reply(&mut writer, &format!("{tag} NO Authentication failed\r\n")).await?;
                }
            }
            "LIST" | "LSUB" => {
                reply(&mut writer, "* LIST (\\HasNoChildren) \"/\" \"INBOX\"\r\n* LIST (\\HasNoChildren) \"/\" \"Sent\"\r\n").await?;
                reply(&mut writer, &format!("{tag} OK {command} completed\r\n")).await?;
            }
            "SELECT" | "EXAMINE" => {
                let Some(owner) = user.as_deref() else {
                    reply(&mut writer, &format!("{tag} NO Authenticate first\r\n")).await?;
                    continue;
                };
                let requested = args.first().map(String::as_str).unwrap_or("INBOX");
                let mailbox = match requested.to_ascii_lowercase().as_str() {
                    "inbox" => "INBOX",
                    "sent" => "Sent",
                    _ => {
                        reply(&mut writer, &format!("{tag} NO Unknown mailbox\r\n")).await?;
                        continue;
                    }
                }
                .to_string();
                let messages = email::imap_messages(&repo, owner, &mailbox)
                    .await
                    .map_err(|e| e.to_string())?;
                reply(&mut writer, &format!(
                    "* {} EXISTS\r\n* 0 RECENT\r\n* FLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)\r\n* OK [UIDVALIDITY 1] klubu\r\n* OK [UIDNEXT {}] next\r\n",
                    messages.len(),
                    messages.last().map(|message| message.id + 1).unwrap_or(1)
                )).await?;
                let read_only = command == "EXAMINE";
                selected = Some((mailbox, read_only));
                reply(
                    &mut writer,
                    &format!(
                        "{tag} OK [{}] {command} completed\r\n",
                        if read_only { "READ-ONLY" } else { "READ-WRITE" }
                    ),
                )
                .await?;
            }
            "STATUS" => {
                let Some(owner) = user.as_deref() else {
                    reply(&mut writer, &format!("{tag} NO Authenticate first\r\n")).await?;
                    continue;
                };
                let requested = args.first().map(String::as_str).unwrap_or("INBOX");
                let mailbox = if requested.eq_ignore_ascii_case("sent") {
                    "Sent"
                } else {
                    "INBOX"
                };
                let messages = email::imap_messages(&repo, owner, mailbox)
                    .await
                    .map_err(|e| e.to_string())?;
                let unseen = messages
                    .iter()
                    .filter(|message| !message.flags.iter().any(|flag| flag == "\\Seen"))
                    .count();
                reply(&mut writer, &format!(
                    "* STATUS \"{mailbox}\" (MESSAGES {} UNSEEN {unseen} UIDNEXT {})\r\n{tag} OK STATUS completed\r\n",
                    messages.len(),
                    messages.last().map(|message| message.id + 1).unwrap_or(1)
                )).await?;
            }
            "LOGOUT" => {
                reply(&mut writer, "* BYE klubu logging out\r\n").await?;
                reply(&mut writer, &format!("{tag} OK LOGOUT completed\r\n")).await?;
                break;
            }
            "UID" | "FETCH" | "SEARCH" | "STORE" | "EXPUNGE" | "APPEND" => {
                let Some(owner) = user.as_deref() else {
                    reply(&mut writer, &format!("{tag} NO Authenticate first\r\n")).await?;
                    continue;
                };
                if command == "APPEND" {
                    let requested = args.first().map(String::as_str).unwrap_or("Sent");
                    let mailbox = if requested.eq_ignore_ascii_case("inbox") {
                        "INBOX"
                    } else {
                        "Sent"
                    };
                    let literal = rest
                        .rsplit_once('{')
                        .and_then(|(_, value)| value.strip_suffix('}'))
                        .and_then(|value| value.parse::<usize>().ok());
                    let Some(length) = literal else {
                        reply(
                            &mut writer,
                            &format!("{tag} BAD APPEND requires a literal\r\n"),
                        )
                        .await?;
                        continue;
                    };
                    reply(&mut writer, "+ Ready for literal data\r\n").await?;
                    let mut raw = vec![0u8; length];
                    reader
                        .read_exact(&mut raw)
                        .await
                        .map_err(|e| e.to_string())?;
                    let mut ending = [0u8; 2];
                    let _ = reader.read_exact(&mut ending).await;
                    email::archive_raw_message(
                        &repo,
                        owner,
                        mailbox,
                        &raw,
                        None,
                        &[],
                        "imap_append",
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    reply(
                        &mut writer,
                        &format!("{tag} OK [APPENDUID 1 1] APPEND completed\r\n"),
                    )
                    .await?;
                    continue;
                }

                let Some((mailbox, read_only)) = selected.clone() else {
                    reply(&mut writer, &format!("{tag} NO Select a mailbox first\r\n")).await?;
                    continue;
                };
                let messages = email::imap_messages(&repo, owner, &mailbox)
                    .await
                    .map_err(|e| e.to_string())?;
                if command == "EXPUNGE" {
                    if read_only {
                        reply(&mut writer, &format!("{tag} NO Mailbox is read-only\r\n")).await?;
                        continue;
                    }
                    let removed = email::expunge_deleted_messages(&repo, owner, &mailbox)
                        .await
                        .map_err(|e| e.to_string())?;
                    for id in removed {
                        reply(&mut writer, &format!("* {id} EXPUNGE\r\n")).await?;
                    }
                    reply(
                        &mut writer,
                        &format!("{tag} OK EXPUNGE completed; raw archive retained\r\n"),
                    )
                    .await?;
                    continue;
                }

                let uid = command == "UID";
                let subcommand = if uid {
                    args.first().map(String::as_str).unwrap_or_default()
                } else {
                    command.as_str()
                };
                let command_args: &[String] = if uid { &args[1..] } else { &args[..] };
                match subcommand.to_ascii_uppercase().as_str() {
                    "SEARCH" => {
                        let unseen = command_args
                            .iter()
                            .any(|arg| arg.eq_ignore_ascii_case("UNSEEN"));
                        let ids = messages
                            .iter()
                            .enumerate()
                            .filter(|(_, message)| {
                                !unseen || !message.flags.iter().any(|flag| flag == "\\Seen")
                            })
                            .map(
                                |(index, message)| {
                                    if uid {
                                        message.id
                                    } else {
                                        (index + 1) as i64
                                    }
                                },
                            )
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                            .join(" ");
                        reply(
                            &mut writer,
                            &format!("* SEARCH {ids}\r\n{tag} OK SEARCH completed\r\n"),
                        )
                        .await?;
                    }
                    "FETCH" => {
                        let indexes = sequence_indexes(
                            command_args.first().map(String::as_str).unwrap_or("1:*"),
                            &messages,
                            uid,
                        );
                        for index in indexes {
                            fetch_message(&mut writer, index + 1, &messages[index]).await?;
                        }
                        reply(&mut writer, &format!("{tag} OK FETCH completed\r\n")).await?;
                    }
                    "STORE" => {
                        if read_only {
                            reply(&mut writer, &format!("{tag} NO Mailbox is read-only\r\n"))
                                .await?;
                            continue;
                        }
                        let indexes = sequence_indexes(
                            command_args.first().map(String::as_str).unwrap_or("1:*"),
                            &messages,
                            uid,
                        );
                        let operation = command_args
                            .get(1)
                            .map(String::as_str)
                            .unwrap_or("+FLAGS")
                            .to_ascii_uppercase();
                        let requested = command_args
                            .get(2)
                            .map(|value| imap_flags(value))
                            .unwrap_or_default();
                        for index in indexes {
                            let mut flags = messages[index].flags.clone();
                            if operation.starts_with("+FLAGS") {
                                for flag in &requested {
                                    if !flags.contains(flag) {
                                        flags.push(flag.clone());
                                    }
                                }
                            } else if operation.starts_with("-FLAGS") {
                                flags.retain(|flag| !requested.contains(flag));
                            } else {
                                flags = requested.clone();
                            }
                            email::set_email_flags(&repo, owner, messages[index].id, flags)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        reply(&mut writer, &format!("{tag} OK STORE completed\r\n")).await?;
                    }
                    _ => {
                        reply(
                            &mut writer,
                            &format!("{tag} BAD Unsupported IMAP command\r\n"),
                        )
                        .await?
                    }
                }
            }
            _ => {
                reply(
                    &mut writer,
                    &format!("{tag} BAD Command not implemented\r\n"),
                )
                .await?
            }
        }
    }
    Ok(())
}

async fn smtp_listener_legacy(repo: Arc<app::db::SqlRepository>, address: String, port: u16) {
    let listener = match TcpListener::bind((address.as_str(), port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("[mail] SMTP relay disabled: cannot bind {address}:{port}: {error}");
            return;
        }
    };
    eprintln!("[mail] SMTP relay listening on {address}:{port}");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let repo = repo.clone();
                tokio::spawn(async move {
                    if let Err(error) = smtp_client(stream, repo).await {
                        eprintln!("[mail] SMTP: {error}");
                    }
                });
            }
            Err(error) => eprintln!("[mail] SMTP accept failed: {error}"),
        }
    }
}

async fn imap_listener_legacy(repo: Arc<app::db::SqlRepository>, address: String, port: u16) {
    let listener = match TcpListener::bind((address.as_str(), port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("[mail] IMAP relay disabled: cannot bind {address}:{port}: {error}");
            return;
        }
    };
    eprintln!("[mail] IMAP relay listening on {address}:{port}");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let repo = repo.clone();
                tokio::spawn(async move {
                    if let Err(error) = imap_client_legacy(stream, repo).await {
                        eprintln!("[mail] IMAP: {error}");
                    }
                });
            }
            Err(error) => eprintln!("[mail] IMAP accept failed: {error}"),
        }
    }
}

pub fn spawn(repo: Arc<app::db::SqlRepository>) -> Vec<tokio::task::JoinHandle<()>> {
    let settings = email::settings();
    if !settings.email_enabled {
        eprintln!("[mail] SMTP/IMAP relay disabled: email feature not enabled");
        return Vec::new();
    }
    if !settings.relay_enabled {
        eprintln!("[mail] SMTP/IMAP relay disabled by KLUBU_MAIL_RELAY_ENABLED");
        return Vec::new();
    }
    let address = bind_address();
    vec![
        tokio::spawn(smtp_listener_legacy(
            repo.clone(),
            address.clone(),
            settings.smtp_port,
        )),
        tokio::spawn(imap_listener_legacy(repo, address, settings.imap_port)),
    ]
}

#[cfg(any())]
async fn imap_client(stream: TcpStream, repo: Arc<app::db::SqlRepository>) -> Result<(), String> {
    let (read_half, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    reply(
        &mut writer,
        "* OK klubu IMAP4rev1 GoBD archive relay ready\r\n",
    )
    .await?;
    let mut user: Option<String> = None;
    let mut selected: Option<(String, bool)> = None;

    loop {
        let mut line = String::new();
        if reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?
            == 0
        {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        let mut pieces = line.splitn(3, ' ');
        let tag = pieces.next().unwrap_or("*");
        let command = pieces.next().unwrap_or_default().to_ascii_uppercase();
        let rest = pieces.next().unwrap_or_default();
        let args = imap_args(rest);
        match command.as_str() {
            "CAPABILITY" => {
                reply(
                    &mut writer,
                    "* CAPABILITY IMAP4rev1 UIDPLUS AUTH=PLAIN AUTH=LOGIN\r\n",
                )
                .await?;
                reply(&mut writer, &format!("{tag} OK CAPABILITY completed\r\n")).await?;
            }
            "NOOP" | "CHECK" => {
                reply(&mut writer, &format!("{tag} OK {command} completed\r\n")).await?
            }
            "ID" => {
                reply(&mut writer, "* ID NIL\r\n").await?;
                reply(&mut writer, &format!("{tag} OK ID completed\r\n")).await?;
            }
            "LOGIN" => {
                if args.len() < 2 {
                    reply(
                        &mut writer,
                        &format!("{tag} BAD LOGIN requires user and password\r\n"),
                    )
                    .await?;
                    continue;
                }
                if email::authenticate_mail_user(&repo, &args[0], &args[1])
                    .await
                    .map_err(|e| e.to_string())?
                {
                    user = Some(args[0].clone());
                    reply(&mut writer, &format!("{tag} OK LOGIN completed\r\n")).await?;
                } else {
                    reply(&mut writer, &format!("{tag} NO Authentication failed\r\n")).await?;
                }
            }
            "LIST" | "LSUB" => {
                reply(&mut writer, "* LIST (\\HasNoChildren) \"/\" \"INBOX\"\r\n* LIST (\\HasNoChildren) \"/\" \"Sent\"\r\n").await?;
                reply(&mut writer, &format!("{tag} OK {command} completed\r\n")).await?;
            }
            "SELECT" | "EXAMINE" => {
                let Some(owner) = user.as_deref() else {
                    reply(&mut writer, &format!("{tag} NO Authenticate first\r\n")).await?;
                    continue;
                };
                let requested = args.first().map(String::as_str).unwrap_or("INBOX");
                let mailbox = match requested.to_ascii_lowercase().as_str() {
                    "inbox" => "INBOX",
                    "sent" => "Sent",
                    _ => {
                        reply(&mut writer, &format!("{tag} NO Unknown mailbox\r\n")).await?;
                        continue;
                    }
                }
                .to_string();
                let messages = email::imap_messages(&repo, owner, &mailbox)
                    .await
                    .map_err(|e| e.to_string())?;
                reply(&mut writer, &format!(
                    "* {} EXISTS\r\n* 0 RECENT\r\n* FLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)\r\n* OK [UIDVALIDITY 1] klubu\r\n* OK [UIDNEXT {}] next\r\n",
                    messages.len(),
                    messages.last().map(|message| message.id + 1).unwrap_or(1)
                )).await?;
                let read_only = command == "EXAMINE";
                selected = Some((mailbox, read_only));
                reply(
                    &mut writer,
                    &format!(
                        "{tag} OK [{}] {command} completed\r\n",
                        if read_only { "READ-ONLY" } else { "READ-WRITE" }
                    ),
                )
                .await?;
            }
            "STATUS" => {
                let Some(owner) = user.as_deref() else {
                    reply(&mut writer, &format!("{tag} NO Authenticate first\r\n")).await?;
                    continue;
                };
                let requested = args.first().map(String::as_str).unwrap_or("INBOX");
                let mailbox = if requested.eq_ignore_ascii_case("sent") {
                    "Sent"
                } else {
                    "INBOX"
                };
                let messages = email::imap_messages(&repo, owner, mailbox)
                    .await
                    .map_err(|e| e.to_string())?;
                let unseen = messages
                    .iter()
                    .filter(|message| !message.flags.iter().any(|flag| flag == "\\Seen"))
                    .count();
                reply(&mut writer, &format!(
                    "* STATUS \"{mailbox}\" (MESSAGES {} UNSEEN {unseen} UIDNEXT {})\r\n{tag} OK STATUS completed\r\n",
                    messages.len(),
                    messages.last().map(|message| message.id + 1).unwrap_or(1)
                )).await?;
            }
            "LOGOUT" => {
                reply(&mut writer, "* BYE klubu logging out\r\n").await?;
                reply(&mut writer, &format!("{tag} OK LOGOUT completed\r\n")).await?;
                break;
            }
            "UID" | "FETCH" | "SEARCH" | "STORE" | "EXPUNGE" | "APPEND" => {
                let Some(owner) = user.as_deref() else {
                    reply(&mut writer, &format!("{tag} NO Authenticate first\r\n")).await?;
                    continue;
                };
                if command == "APPEND" {
                    let requested = args.first().map(String::as_str).unwrap_or("Sent");
                    let mailbox = if requested.eq_ignore_ascii_case("inbox") {
                        "INBOX"
                    } else {
                        "Sent"
                    };
                    let literal = rest
                        .rsplit_once('{')
                        .and_then(|(_, value)| value.strip_suffix('}'))
                        .and_then(|value| value.parse::<usize>().ok());
                    let Some(length) = literal else {
                        reply(
                            &mut writer,
                            &format!("{tag} BAD APPEND requires a literal\r\n"),
                        )
                        .await?;
                        continue;
                    };
                    reply(&mut writer, "+ Ready for literal data\r\n").await?;
                    let mut raw = vec![0u8; length];
                    reader
                        .read_exact(&mut raw)
                        .await
                        .map_err(|e| e.to_string())?;
                    let mut ending = [0u8; 2];
                    let _ = reader.read_exact(&mut ending).await;
                    email::archive_raw_message(
                        &repo,
                        owner,
                        mailbox,
                        &raw,
                        None,
                        &[],
                        "imap_append",
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    reply(
                        &mut writer,
                        &format!("{tag} OK [APPENDUID 1 1] APPEND completed\r\n"),
                    )
                    .await?;
                    continue;
                }

                let Some((mailbox, read_only)) = selected.clone() else {
                    reply(&mut writer, &format!("{tag} NO Select a mailbox first\r\n")).await?;
                    continue;
                };
                let messages = email::imap_messages(&repo, owner, &mailbox)
                    .await
                    .map_err(|e| e.to_string())?;
                if command == "EXPUNGE" {
                    if read_only {
                        reply(&mut writer, &format!("{tag} NO Mailbox is read-only\r\n")).await?;
                        continue;
                    }
                    let removed = email::expunge_deleted_messages(&repo, owner, &mailbox)
                        .await
                        .map_err(|e| e.to_string())?;
                    for id in removed {
                        reply(&mut writer, &format!("* {id} EXPUNGE\r\n")).await?;
                    }
                    reply(
                        &mut writer,
                        &format!("{tag} OK EXPUNGE completed; raw archive retained\r\n"),
                    )
                    .await?;
                    continue;
                }

                let uid = command == "UID";
                let subcommand = if uid {
                    args.first().map(String::as_str).unwrap_or_default()
                } else {
                    command.as_str()
                };
                let command_args: &[String] = if uid { &args[1..] } else { &args[..] };
                match subcommand.to_ascii_uppercase().as_str() {
                    "SEARCH" => {
                        let unseen = command_args
                            .iter()
                            .any(|arg| arg.eq_ignore_ascii_case("UNSEEN"));
                        let ids = messages
                            .iter()
                            .enumerate()
                            .filter(|(_, message)| {
                                !unseen || !message.flags.iter().any(|flag| flag == "\\Seen")
                            })
                            .map(
                                |(index, message)| {
                                    if uid {
                                        message.id
                                    } else {
                                        (index + 1) as i64
                                    }
                                },
                            )
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                            .join(" ");
                        reply(
                            &mut writer,
                            &format!("* SEARCH {ids}\r\n{tag} OK SEARCH completed\r\n"),
                        )
                        .await?;
                    }
                    "FETCH" => {
                        let indexes = sequence_indexes(
                            command_args.first().map(String::as_str).unwrap_or("1:*"),
                            &messages,
                            uid,
                        );
                        for index in indexes {
                            fetch_message(&mut writer, index + 1, &messages[index]).await?;
                        }
                        reply(&mut writer, &format!("{tag} OK FETCH completed\r\n")).await?;
                    }
                    "STORE" => {
                        if read_only {
                            reply(&mut writer, &format!("{tag} NO Mailbox is read-only\r\n"))
                                .await?;
                            continue;
                        }
                        let indexes = sequence_indexes(
                            command_args.first().map(String::as_str).unwrap_or("1:*"),
                            &messages,
                            uid,
                        );
                        let operation = command_args
                            .get(1)
                            .map(String::as_str)
                            .unwrap_or("+FLAGS")
                            .to_ascii_uppercase();
                        let requested = command_args
                            .get(2)
                            .map(|value| imap_flags(value))
                            .unwrap_or_default();
                        for index in indexes {
                            let mut flags = messages[index].flags.clone();
                            if operation.starts_with("+FLAGS") {
                                for flag in &requested {
                                    if !flags.contains(flag) {
                                        flags.push(flag.clone());
                                    }
                                }
                            } else if operation.starts_with("-FLAGS") {
                                flags.retain(|flag| !requested.contains(flag));
                            } else {
                                flags = requested.clone();
                            }
                            email::set_email_flags(&repo, owner, messages[index].id, flags)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        reply(&mut writer, &format!("{tag} OK STORE completed\r\n")).await?;
                    }
                    _ => {
                        reply(
                            &mut writer,
                            &format!("{tag} BAD Unsupported IMAP command\r\n"),
                        )
                        .await?
                    }
                }
            }
            _ => {
                reply(
                    &mut writer,
                    &format!("{tag} BAD Command not implemented\r\n"),
                )
                .await?
            }
        }
    }
    Ok(())
}

#[cfg(any())]
async fn smtp_listener(repo: Arc<app::db::SqlRepository>, address: String, port: u16) {
    let listener = match TcpListener::bind((address.as_str(), port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("[mail] SMTP relay disabled: cannot bind {address}:{port}: {error}");
            return;
        }
    };
    eprintln!("[mail] SMTP relay listening on {address}:{port}");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let repo = repo.clone();
                tokio::spawn(async move {
                    if let Err(error) = smtp_client(stream, repo).await {
                        eprintln!("[mail] SMTP: {error}");
                    }
                });
            }
            Err(error) => eprintln!("[mail] SMTP accept failed: {error}"),
        }
    }
}

#[cfg(any())]
async fn imap_listener(repo: Arc<app::db::SqlRepository>, address: String, port: u16) {
    let listener = match TcpListener::bind((address.as_str(), port)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("[mail] IMAP relay disabled: cannot bind {address}:{port}: {error}");
            return;
        }
    };
    eprintln!("[mail] IMAP relay listening on {address}:{port}");
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let repo = repo.clone();
                tokio::spawn(async move {
                    if let Err(error) = imap_client(stream, repo).await {
                        eprintln!("[mail] IMAP: {error}");
                    }
                });
            }
            Err(error) => eprintln!("[mail] IMAP accept failed: {error}"),
        }
    }
}

#[cfg(any())]
pub fn spawn_disabled(repo: Arc<app::db::SqlRepository>) -> Vec<tokio::task::JoinHandle<()>> {
    let settings = email::settings();
    if !settings.relay_enabled {
        eprintln!("[mail] SMTP/IMAP relay disabled by KLUBU_MAIL_RELAY_ENABLED");
        return Vec::new();
    }
    let address = bind_address();
    vec![
        tokio::spawn(smtp_listener(
            repo.clone(),
            address.clone(),
            settings.smtp_port,
        )),
        tokio::spawn(imap_listener(repo, address, settings.imap_port)),
    ]
}
