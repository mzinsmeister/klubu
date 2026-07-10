//! Markdown for the free-text blocks of invoices and offers.
//!
//! The templates render these blocks with `eval(.., mode: "markup")`, so what we
//! hand over must be Typst markup. Every scrap of *user* text is escaped before
//! it goes in — the only unescaped characters in the result are the ones this
//! module emits itself. That is what keeps `eval` from becoming a way to run
//! arbitrary Typst code from an invoice footer.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Escapes the characters that mean something in Typst markup.
///
/// Deliberately heavy-handed: escaping a `-` that did not need it renders the
/// same `-`, whereas missing a `#` hands the user a code injection.
fn escape_typst(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(
            ch,
            '\\' | '#' | '$' | '*' | '_' | '`' | '<' | '>' | '@' | '[' | ']' | '~' | '=' | '+'
                | '-' | '/' | '\''| '"'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Converts a Markdown fragment to Typst markup.
///
/// Supports what a payment note actually needs: headings, emphasis, bullet and
/// numbered lists, links, inline code, block quotes and rules. Anything else
/// degrades to its plain text.
pub fn markdown_to_typst(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, options);

    let mut out = String::new();
    // Ordered lists carry their own counter in Typst (`+`), so we only need to
    // know *which kind* of list we are inside, not how far along.
    let mut list_stack: Vec<bool> = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                let depth = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                out.push('\n');
                out.push_str(&"=".repeat(depth));
                out.push(' ');
            }
            Event::End(TagEnd::Heading(_)) => out.push_str("\n\n"),

            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => out.push_str("\n\n"),

            Event::Start(Tag::Strong) => out.push('*'),
            Event::End(TagEnd::Strong) => out.push('*'),

            Event::Start(Tag::Emphasis) => out.push('_'),
            Event::End(TagEnd::Emphasis) => out.push('_'),

            Event::Start(Tag::Strikethrough) => out.push_str("#strike["),
            Event::End(TagEnd::Strikethrough) => out.push(']'),

            Event::Start(Tag::List(first)) => {
                list_stack.push(first.is_some());
                out.push('\n');
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                out.push('\n');
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                let marker = if *list_stack.last().unwrap_or(&false) { "+" } else { "-" };
                out.push_str(&format!("{indent}{marker} "));
            }
            Event::End(TagEnd::Item) => out.push('\n'),

            Event::Start(Tag::BlockQuote(_)) => out.push_str("#quote(block: true)["),
            Event::End(TagEnd::BlockQuote(_)) => out.push_str("]\n\n"),

            Event::Start(Tag::Link { dest_url, .. }) => {
                let url = dest_url.replace('\\', "\\\\").replace('"', "\\\"");
                out.push_str(&format!("#link(\"{url}\")["));
            }
            Event::End(TagEnd::Link) => out.push(']'),

            Event::Start(Tag::CodeBlock(_)) => out.push_str("\n#raw(block: true, \""),
            Event::End(TagEnd::CodeBlock) => out.push_str("\")\n\n"),

            Event::Code(text) => {
                out.push_str(&format!("#raw(\"{}\")", text.replace('\\', "\\\\").replace('"', "\\\"")));
            }
            Event::Text(text) => out.push_str(&escape_typst(&text)),

            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push_str(" \\ "),
            Event::Rule => out.push_str("\n#line(length: 100%)\n\n"),

            // Raw HTML has no meaning here; show it as the text it is.
            Event::Html(t) | Event::InlineHtml(t) => out.push_str(&escape_typst(&t)),
            _ => {}
        }
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_and_paragraph() {
        let out = markdown_to_typst("# Zahlungsziel\nBitte bezahlen sie innerhalb von 14 Tagen.");
        assert!(out.starts_with("= Zahlungsziel"));
        assert!(out.contains("Bitte bezahlen sie innerhalb von 14 Tagen."));
    }

    #[test]
    fn emphasis_and_lists() {
        let out = markdown_to_typst("**fett** und *kursiv*\n\n- eins\n- zwei");
        assert!(out.contains("*fett*"));
        assert!(out.contains("_kursiv_"));
        assert!(out.contains("- eins"));
        assert!(out.contains("- zwei"));
    }

    #[test]
    fn ordered_list_uses_typst_counter() {
        let out = markdown_to_typst("1. eins\n2. zwei");
        assert!(out.contains("+ eins"));
        assert!(out.contains("+ zwei"));
    }

    /// Drops every `\x` escape pair, leaving only the characters that Typst
    /// would still treat as active markup.
    fn strip_escapes(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                chars.next();
            } else {
                out.push(c);
            }
        }
        out
    }

    /// The whole point of escaping: a footer must not be able to run Typst code.
    /// `\#panic` still *contains* the substring `#panic`, so assert the property
    /// rather than the spelling — nothing active may survive escape-stripping.
    #[test]
    fn user_text_cannot_inject_typst_code() {
        let out = markdown_to_typst("#panic(\"boom\") and $x^2$ and [link]");
        let active = strip_escapes(&out);
        assert!(!active.contains('#'), "unescaped # in {out:?}");
        assert!(!active.contains('$'), "unescaped $ in {out:?}");
        assert!(!active.contains('['), "unescaped [ in {out:?}");
        // ...and the text itself is preserved for the reader.
        assert!(out.contains("panic"));
        assert!(out.contains("link"));
    }

    /// Markup this module emits itself must stay active — otherwise the escape
    /// pass would have flattened the formatting too.
    #[test]
    fn emitted_markup_survives() {
        let out = markdown_to_typst("# Titel\n\n**fett**");
        let active = strip_escapes(&out);
        assert!(active.starts_with("= Titel"));
        assert!(active.contains("*fett*"));
    }

    #[test]
    fn link_renders_as_typst_link() {
        let out = markdown_to_typst("[Klubu](https://example.com)");
        assert!(out.contains("#link(\"https://example.com\")["));
    }
}

#[cfg(test)]
mod render_tests {
    /// The unit tests above only check the *string* we emit. This one feeds it
    /// through the real Typst compiler, which is the only way to know that
    /// `eval(.., mode: "markup")` accepts what `markdown_to_typst` produces.
    #[test]
    fn converted_markdown_compiles_under_typst() {
        let md = "# Zahlungsziel\n\nBitte bezahlen sie innerhalb von **14 Tagen**.\n\n\
                  - Konto: DE12 3456\n- BIC: ABCDEF\n\n\
                  Ein #boeser $Versuch$ und [nur Text].";
        let markup = super::markdown_to_typst(md);
        let doc = format!("#set page(width: 200mm, height: 100mm)\n{}", markup);
        let pdf = crate::pdf::compiler::compile_typst(doc)
            .unwrap_or_else(|e| panic!("typst rejected converted markdown: {e}\n---\n{markup}"));
        assert!(pdf.starts_with(b"%PDF"), "not a pdf");
    }
}
