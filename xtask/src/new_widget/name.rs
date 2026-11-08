//! The name of a new widget and the forms of it the generated code uses.

use super::error::NewWidgetError;

/// Rust keywords, strict and reserved for later editions; none of them can
/// name a module.
const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const",
    "continue", "crate", "do", "dyn", "else", "enum", "extern", "false",
    "final", "fn", "for", "gen", "if", "impl", "in", "let", "loop", "macro",
    "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// A checked widget name in the forms the generated code uses.
#[derive(Debug)]
pub(super) struct WidgetName {
    snake: String,
    camel: String,
    words: String,
}

impl WidgetName {
    /// The folder, module and field name, e.g. `sample_notes`.
    pub(super) fn snake(&self) -> &str {
        &self.snake
    }

    /// The start of the widget's type names, e.g. `SampleNotes`.
    pub(super) fn camel(&self) -> &str {
        &self.camel
    }

    /// The name for doc comments, e.g. `sample notes`.
    pub(super) fn words(&self) -> &str {
        &self.words
    }

    /// Check `name` and build its forms.
    pub(super) fn parse(name: &str) -> Result<Self, NewWidgetError> {
        if !is_snake_case(name) {
            return Err(NewWidgetError::NotSnakeCase(name.to_string()));
        }
        if KEYWORDS.contains(&name) {
            return Err(NewWidgetError::Keyword(name.to_string()));
        }

        Ok(Self {
            snake: name.to_string(),
            camel: name.split('_').map(capitalized).collect(),
            words: name.replace('_', " "),
        })
    }
}

/// Whether `name` is lowercase ASCII letters and digits in parts joined by
/// single underscores, starting with a letter.
fn is_snake_case(name: &str) -> bool {
    let starts_with_letter = name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase());
    let parts_are_plain = name.split('_').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
    });

    starts_with_letter && parts_are_plain
}

/// `word` with its first letter in upper case.
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    let first = first.to_ascii_uppercase();
    let rest = chars.as_str();
    format!("{first}{rest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_names_are_accepted() {
        for name in ["notes", "sample_notes", "tab2", "ui_2d"] {
            let parsed = WidgetName::parse(name).expect("a valid name");

            assert_eq!(parsed.snake(), name);
        }
    }

    #[test]
    fn names_that_are_not_snake_case_are_refused() {
        let names = [
            "",
            "Notes",
            "sampleNotes",
            "sample-notes",
            "sample notes",
            "_notes",
            "notes_",
            "sample__notes",
            "2notes",
            "notes!",
            "caf\u{e9}",
        ];

        for name in names {
            let error = WidgetName::parse(name).expect_err(name);

            assert!(
                matches!(&error, NewWidgetError::NotSnakeCase(refused) if refused == name),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn rust_keywords_are_refused() {
        for name in ["type", "mod", "self", "crate", "async", "gen", "try"] {
            let error = WidgetName::parse(name).expect_err(name);

            assert!(
                matches!(&error, NewWidgetError::Keyword(refused) if refused == name),
                "{name}: {error}"
            );
        }
    }

    #[test]
    fn a_refused_name_says_why() {
        let messages = [
            WidgetName::parse("Notes").map(|_| ()),
            WidgetName::parse("type").map(|_| ()),
        ]
        .map(|result| result.map_err(|error| error.to_string()));

        assert_eq!(
            messages,
            [
                Err("`Notes` is not snake_case: use lowercase ASCII letters, \
                     digits and single underscores, starting with a letter"
                    .to_string()),
                Err("`type` is a Rust keyword".to_string()),
            ]
        );
    }

    #[test]
    fn the_camel_case_name_starts_every_word_with_a_capital() {
        let cases = [
            ("notes", "Notes"),
            ("sample_notes", "SampleNotes"),
            ("terminal_workspace", "TerminalWorkspace"),
            ("tab2", "Tab2"),
            ("ui_2d", "Ui2d"),
        ];

        for (snake, camel) in cases {
            let parsed = WidgetName::parse(snake).expect("a valid name");

            assert_eq!(parsed.camel(), camel);
        }
    }

    #[test]
    fn the_words_are_the_parts_of_the_name() {
        let parsed = WidgetName::parse("sample_notes").expect("a valid name");

        assert_eq!(parsed.words(), "sample notes");
    }
}
