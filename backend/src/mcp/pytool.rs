//! Sandboxed Python execution for the assistant tools.
//!
//! This is a process-level sandbox, not a VM: the interpreter runs as the
//! unprivileged server user in isolated mode (`python3 -I`) with a cleared
//! environment, a throwaway working directory, and hard resource limits (CPU
//! time, address space, file size, descriptors, process count) applied via
//! `setrlimit` between fork and exec. It deliberately has no access to Klubu
//! secrets through the environment, but it is *not* a network boundary —
//! installations that need one should disable `klubu.tools.pythonEnabled`.

use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Wall-clock budget; generous next to the CPU limit so an interpreter that
/// blocks (sleep, slow I/O) still gets reaped.
const WALL_TIMEOUT: Duration = Duration::from_secs(20);
const CPU_SECONDS: libc::rlim_t = 10;
const ADDRESS_SPACE_BYTES: libc::rlim_t = 1024 << 20;
const FILE_SIZE_BYTES: libc::rlim_t = 8 << 20;
const OPEN_FILES: libc::rlim_t = 64;
/// Fork-bomb brake. Counts all processes of the server user, so it must stay
/// above what the server itself keeps running.
const PROCESSES: libc::rlim_t = 512;
const OUTPUT_LIMIT_BYTES: usize = 64 * 1024;

pub async fn run(code: String) -> Result<Value, String> {
    let workdir = tempfile::tempdir()
        .map_err(|error| format!("Could not create a scratch directory: {error}"))?;

    let mut command = tokio::process::Command::new("python3");
    command
        // Isolated mode: no PYTHON* environment influence, no user site dir,
        // no current directory on sys.path.
        .args(["-I", "-"])
        .current_dir(workdir.path())
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("HOME", workdir.path())
        .env("TMPDIR", workdir.path())
        // BLAS/OpenMP worker pools add nothing for short scripts and count
        // against the process limit.
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("OMP_NUM_THREADS", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    unsafe {
        command.pre_exec(|| {
            for (resource, limit) in [
                (libc::RLIMIT_CPU, CPU_SECONDS),
                (libc::RLIMIT_AS, ADDRESS_SPACE_BYTES),
                (libc::RLIMIT_FSIZE, FILE_SIZE_BYTES),
                (libc::RLIMIT_NOFILE, OPEN_FILES),
                (libc::RLIMIT_NPROC, PROCESSES),
            ] {
                let rlimit = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                if libc::setrlimit(resource, &rlimit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "Python 3 is not installed on this server. Install python3 or disable the tool via klubu.tools.pythonEnabled."
                .to_string()
        } else {
            format!("Could not start python3: {error}")
        }
    })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Could not open stdin of the Python process".to_string())?;
    stdin
        .write_all(code.as_bytes())
        .await
        .map_err(|error| format!("Could not pass the code to Python: {error}"))?;
    drop(stdin);

    // On timeout the future is dropped, which kills the child (kill_on_drop).
    let output = tokio::time::timeout(WALL_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| {
            format!(
                "Python execution exceeded the {}s wall-clock limit and was killed",
                WALL_TIMEOUT.as_secs()
            )
        })?
        .map_err(|error| format!("Python execution failed: {error}"))?;

    let (stdout, stdout_truncated) = capped_text(&output.stdout);
    let (stderr, stderr_truncated) = capped_text(&output.stderr);
    Ok(json!({
        "exit_code": output.status.code(),
        "stdout": stdout,
        "stdout_truncated": stdout_truncated,
        "stderr": stderr,
        "stderr_truncated": stderr_truncated,
    }))
}

fn capped_text(bytes: &[u8]) -> (String, bool) {
    if bytes.len() <= OUTPUT_LIMIT_BYTES {
        return (String::from_utf8_lossy(bytes).into_owned(), false);
    }
    (
        String::from_utf8_lossy(&bytes[..OUTPUT_LIMIT_BYTES]).into_owned(),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_a_print_and_captures_stdout() {
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_err()
        {
            return; // no python3 on this machine; the tool itself reports that
        }
        let result = run("print(21 * 2)".to_string()).await.unwrap();
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdout"].as_str().unwrap().trim(), "42");
    }

    #[tokio::test]
    async fn reports_errors_via_stderr_and_exit_code() {
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let result = run("raise ValueError('boom')".to_string()).await.unwrap();
        assert_ne!(result["exit_code"], 0);
        assert!(result["stderr"].as_str().unwrap().contains("boom"));
    }
}
