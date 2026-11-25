//! The check of a plugin icon: a small SVG with no script, no external
//! link and no embedded image (spec 2026-11-19-agent-plugins, "What a
//! plugin is").

use std::fmt;
use std::sync::Arc;

/// The largest icon OTTY keeps.
pub const MAX_ICON_BYTES: u64 = 16 * 1024;

/// Why an icon is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconProblem {
    /// Larger than [`MAX_ICON_BYTES`].
    TooLarge,
    /// Not UTF-8 text.
    NotText,
    /// It holds a script or an event handler such as `onload`.
    Script,
    /// It holds an `<image>` or a `<foreignObject>`.
    EmbeddedContent,
    /// It links to something outside itself.
    ExternalLink(String),
    /// It does not parse as SVG.
    NotSvg(String),
}

impl fmt::Display for IconProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IconProblem::TooLarge => {
                write!(formatter, "is larger than {MAX_ICON_BYTES} bytes")
            },
            IconProblem::NotText => write!(formatter, "is not text"),
            IconProblem::Script => write!(formatter, "holds a script"),
            IconProblem::EmbeddedContent => {
                write!(formatter, "holds an embedded image or document")
            },
            IconProblem::ExternalLink(link) => {
                write!(formatter, "links to `{link}` outside itself")
            },
            IconProblem::NotSvg(reason) => {
                write!(formatter, "is not an SVG image: {reason}")
            },
        }
    }
}

/// Checks the icon `bytes`: at most [`MAX_ICON_BYTES`], no script, no
/// external link, no embedded image or document, and it parses as SVG.
pub(crate) fn check_icon(bytes: &[u8]) -> Result<(), IconProblem> {
    if bytes.len() as u64 > MAX_ICON_BYTES {
        return Err(IconProblem::TooLarge);
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Err(IconProblem::NotText);
    };

    let lower = text.to_ascii_lowercase();
    if lower.contains("<script") || has_event_handler(&lower) {
        return Err(IconProblem::Script);
    }
    if lower.contains("<image") || lower.contains("<foreignobject") {
        return Err(IconProblem::EmbeddedContent);
    }
    if let Some(link) = external_link(&lower) {
        return Err(IconProblem::ExternalLink(link));
    }

    usvg::Tree::from_data(bytes, &closed_options())
        .map(|_| ())
        .map_err(|error| IconProblem::NotSvg(error.to_string()))
}

/// Parse options that load nothing from outside the icon.
fn closed_options() -> usvg::Options<'static> {
    usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _: Arc<Vec<u8>>, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    }
}

/// Whether an attribute such as `onload=` is in `lower`: one starting
/// with `on` after a space, then letters, then `=`.
fn has_event_handler(lower: &str) -> bool {
    lower.match_indices("on").any(|(start, _)| {
        let after_space = lower[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_whitespace());
        let rest = &lower[start + "on".len()..];
        let name_end = rest
            .find(|character: char| !character.is_ascii_alphabetic())
            .unwrap_or(rest.len());

        after_space
            && name_end > 0
            && rest[name_end..].trim_start().starts_with('=')
    })
}

/// The first link in `lower` that points outside the icon: an `href`, a
/// `url(...)` or an `@import` that is not `#id`.
fn external_link(lower: &str) -> Option<String> {
    if lower.contains("@import") {
        return Some("@import".to_string());
    }

    for (start, _) in lower.match_indices("href") {
        let rest = lower[start + "href".len()..].trim_start();
        let Some(value) = rest.strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start().trim_start_matches(['"', '\'']);
        if !value.starts_with('#') {
            return Some(link_text(value));
        }
    }

    for (start, _) in lower.match_indices("url(") {
        let value = lower[start + "url(".len()..]
            .trim_start()
            .trim_start_matches(['"', '\'']);
        if !value.starts_with('#') {
            return Some(link_text(value));
        }
    }

    None
}

/// The link at the start of `value`, up to its closing quote or bracket.
fn link_text(value: &str) -> String {
    value
        .split(['"', '\'', ')', ' ', '>'])
        .next()
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M1 1h14v14H1z" fill="currentColor"/></svg>"#;

    fn with_body(body: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">{body}</svg>"#
        )
    }

    #[test]
    fn a_plain_icon_is_kept() {
        assert_eq!(check_icon(ICON.as_bytes()), Ok(()));
    }

    #[test]
    fn a_link_inside_the_icon_is_kept() {
        let icon = with_body(
            r##"<defs><linearGradient id="g"/></defs><use href="#p"/><path id="p" d="M1 1h4v4z" fill="url(#g)"/>"##,
        );

        assert_eq!(check_icon(icon.as_bytes()), Ok(()));
    }

    #[test]
    fn a_large_icon_is_refused() {
        let icon = with_body(&" ".repeat(17 * 1024));

        assert_eq!(check_icon(icon.as_bytes()), Err(IconProblem::TooLarge));
    }

    #[test]
    fn a_script_is_refused() {
        let script = with_body("<script>alert(1)</script>");
        let handler = with_body(r#"<path onload = "alert(1)" d="M1 1z"/>"#);

        assert_eq!(check_icon(script.as_bytes()), Err(IconProblem::Script));
        assert_eq!(check_icon(handler.as_bytes()), Err(IconProblem::Script));
    }

    #[test]
    fn an_image_is_refused() {
        let icon = with_body(r#"<image href="data:image/png;base64,AAAA"/>"#);

        assert_eq!(
            check_icon(icon.as_bytes()),
            Err(IconProblem::EmbeddedContent)
        );
    }

    #[test]
    fn an_external_link_is_refused() {
        let href =
            with_body(r#"<use xlink:href="https://evil.test/a.svg#x"/>"#);
        let style = with_body(
            r#"<path d="M1 1z" style="fill: url('https://evil.test/p')"/>"#,
        );

        assert_eq!(
            check_icon(href.as_bytes()),
            Err(IconProblem::ExternalLink(
                "https://evil.test/a.svg#x".to_string()
            ))
        );
        assert_eq!(
            check_icon(style.as_bytes()),
            Err(IconProblem::ExternalLink("https://evil.test/p".to_string()))
        );
    }

    #[test]
    fn text_that_is_not_svg_is_refused() {
        let problem = check_icon(b"<html></html>").expect_err("not SVG");

        assert!(matches!(problem, IconProblem::NotSvg(_)));
    }
}
