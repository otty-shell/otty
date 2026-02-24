use std::fmt::Write;
use std::str::FromStr;

use cursor_icon::CursorIcon;
use log::debug;

use crate::color::{Rgb, StdColor, xparse_color};
use crate::cursor::CursorShape;
use crate::hyperlink::Hyperlink;
use crate::parser::parse_number;
use crate::{Action, EscapeActor};

/// Operating system command with raw arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
enum OSC {
    SetWindowTitle,
    SetColorIndex,
    Hyperlink,
    SetTextBackgroundColor,
    SetTextForegroundColor,
    SetTextCursorColor,
    SetMouseCursorIcon,
    SetCursorShape,
    Clipboard,
    ResetIndexedColors,
    ResetForegroundColor,
    ResetBackgroundColor,
    ResetCursorColor,
    Unhandled,
}

impl From<&[u8]> for OSC {
    fn from(action: &[u8]) -> Self {
        match action {
            b"0" | b"2" => Self::SetWindowTitle,
            b"4" => Self::SetColorIndex,
            b"8" => Self::Hyperlink,
            // xterm dynamic colors: 10=foreground, 11=background, 12=cursor
            b"10" => Self::SetTextForegroundColor,
            b"11" => Self::SetTextBackgroundColor,
            b"12" => Self::SetTextCursorColor,
            b"22" => Self::SetMouseCursorIcon,
            b"50" => Self::SetCursorShape,
            b"52" => Self::Clipboard,
            b"104" => Self::ResetIndexedColors,
            b"110" => Self::ResetForegroundColor,
            b"111" => Self::ResetBackgroundColor,
            b"112" => Self::ResetCursorColor,
            _ => Self::Unhandled,
        }
    }
}

pub(crate) fn perform<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    if params.is_empty() || params[0].is_empty() {
        return;
    }

    match OSC::from(params[0]) {
        OSC::Hyperlink if params.len() > 2 => {
            hyperlink_processing(actor, params)
        },
        OSC::SetColorIndex => set_indexed_color(actor, params),
        OSC::SetWindowTitle => set_titile(actor, params),
        OSC::SetMouseCursorIcon => set_mouse_cursor_shape(actor, params),
        OSC::SetCursorShape => set_cursor_style(actor, params),
        OSC::ResetIndexedColors => reset_indexed_colors(actor, params),
        OSC::ResetBackgroundColor => {
            actor.handle(Action::ResetColor(StdColor::Background as usize))
        },
        OSC::ResetForegroundColor => {
            actor.handle(Action::ResetColor(StdColor::Foreground as usize))
        },
        OSC::ResetCursorColor => {
            actor.handle(Action::ResetColor(StdColor::Cursor as usize))
        },
        OSC::SetTextForegroundColor => {
            set_dynamic_std_color(actor, params, StdColor::Foreground)
        },
        OSC::SetTextBackgroundColor => {
            set_dynamic_std_color(actor, params, StdColor::Background)
        },
        OSC::SetTextCursorColor => {
            set_dynamic_std_color(actor, params, StdColor::Cursor)
        },
        _ => unexpected(params),
    }
}

fn set_titile<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    if params.len() < 2 {
        return unexpected(params);
    }

    let title = params[1..]
        .iter()
        .flat_map(|x| str::from_utf8(x))
        .collect::<Vec<&str>>()
        .join(";")
        .trim()
        .to_owned();

    actor.handle(Action::SetWindowTitle(title));
}

fn hyperlink_processing<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    let link_params = params[1];

    // NOTE: The escape sequence is of form 'OSC 8 ; params ; URI ST', where
    // URI is URL-encoded. However `;` is a special character and might be
    // passed as is, thus we need to rebuild the URI.
    let mut uri = str::from_utf8(params[2]).unwrap_or_default().to_string();
    for param in params[3..].iter() {
        uri.push(';');
        uri.push_str(str::from_utf8(param).unwrap_or_default());
    }

    // The OSC 8 escape sequence must be stopped when getting an empty `uri`.
    if uri.is_empty() {
        actor.handle(Action::SetHyperlink(None));
        return;
    }

    // Link parameters are in format of `key1=value1:key2=value2`. Currently only
    // key `id` is defined.
    let id = link_params
        .split(|&b| b == b':')
        .find_map(|kv| kv.strip_prefix(b"id="))
        .and_then(|kv| str::from_utf8(kv).ok().map(|e| e.to_owned()));

    actor.handle(Action::SetHyperlink(Some(Hyperlink { id, uri })));
}

fn set_indexed_color<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    if params.len() <= 1 || params.len().is_multiple_of(2) {
        return unexpected(params);
    }

    for chunk in params[1..].chunks(2) {
        let index = match parse_number(chunk[0]) {
            Some(index) => index,
            None => {
                unexpected(params);
                continue;
            },
        };

        if let Some(c) = xparse_color(chunk[1]) {
            actor.handle(Action::SetColor {
                index: index as usize,
                color: c,
            });
        } else if chunk[1] == b"?" {
            actor.handle(Action::QueryColor(index as usize));
        } else {
            unexpected(params)
        }
    }
}

fn set_mouse_cursor_shape<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    let shape = String::from_utf8_lossy(params[1]);
    match CursorIcon::from_str(&shape) {
        Ok(cursor_icon) => actor.handle(Action::SetCursorIcon(cursor_icon)),
        Err(_) => debug!("[osc 22] unrecognized cursor icon shape: {shape:?}"),
    }
}

fn set_cursor_style<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    if params.len() >= 2
        && params[1].len() >= 13
        && params[1][0..12] == *b"CursorShape="
    {
        let shape = match params[1][12] as char {
            '0' => CursorShape::Block,
            '1' => CursorShape::Beam,
            '2' => CursorShape::Underline,
            _ => return unexpected(params),
        };
        actor.handle(Action::SetCursorShape(shape));
        return;
    }

    unexpected(params);
}

fn reset_indexed_colors<A: EscapeActor>(actor: &mut A, params: &[&[u8]]) {
    if params.len() == 1 || params[1].is_empty() {
        // Reset all
        for i in 0..256 {
            actor.handle(Action::ResetColor(i));
        }
    } else {
        // Reset by params
        for param in &params[1..] {
            match parse_number(param) {
                Some(index) => actor.handle(Action::ResetColor(index as usize)),
                None => unexpected(params),
            }
        }
    }
}

fn set_dynamic_std_color<A: EscapeActor>(
    actor: &mut A,
    params: &[&[u8]],
    color: StdColor,
) {
    // Expect at least action + color parameter
    if params.len() < 2 {
        return unexpected(params);
    }

    let spec = params[1];

    // Query current color: OSC Ps ; ? ST
    if spec == b"?" {
        return actor.handle(Action::QueryColor(color as usize));
    }

    // Accept only first color spec; extra params are ignored per xterm behavior
    match xparse_color(spec) {
        Some(rgb) => actor.handle(Action::SetColor {
            index: color as usize,
            color: rgb,
        }),
        None => {
            // Also try hex forms understood by Rgb::from_str
            if let Some(rgb) = std::str::from_utf8(spec)
                .ok()
                .and_then(|s| Rgb::from_str(s).ok())
            {
                actor.handle(Action::SetColor {
                    index: color as usize,
                    color: rgb,
                })
            } else {
                unexpected(params)
            }
        },
    }
}

fn unexpected(params: &[&[u8]]) {
    let mut buf = String::new();

    for param in params {
        buf.push('[');
        for param_item in *param {
            let _ = write!(buf, "{:?}", *param_item as char);
        }
        buf.push(']');
    }

    debug!("[unexpected osc]: params: [{}], line: {}", &buf, line!());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EscapeParser, Parser};

    #[derive(Default, Debug)]
    struct RecordingActor {
        actions: Vec<Action>,
    }

    impl EscapeActor for RecordingActor {
        fn handle(&mut self, action: Action) {
            self.actions.push(action);
        }
    }

    impl RecordingActor {
        fn parse(input: &str) -> Self {
            let mut parser: Parser<otty_vte::Parser> = Parser::default();
            let mut actor = Self::default();
            parser.advance(input.as_bytes(), &mut actor);
            actor
        }
    }

    #[test]
    fn set_window_title_variants() {
        let cases = vec![
            (
                "\x1b]0;  First Title  \x07",
                vec![Action::SetWindowTitle("First Title".to_string())],
            ),
            (
                "\x1b]2;Part1;Part2\x1b\\",
                vec![Action::SetWindowTitle("Part1;Part2".to_string())],
            ),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn hyperlink_open_and_close() {
        let cases = vec![(
            "\x1b]8;id=session;https://example.com;foo=bar\x07",
            vec![Action::SetHyperlink(Some(Hyperlink {
                id: Some(String::from("session")),
                uri: String::from("https://example.com;foo=bar"),
            }))],
        )];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn set_indexed_colors_and_query() {
        let cases = vec![
            (
                "\x1b]4;1;#112233;2;#445566\x07",
                vec![
                    Action::SetColor {
                        index: 1,
                        color: Rgb {
                            r: 0x11,
                            g: 0x22,
                            b: 0x33,
                        },
                    },
                    Action::SetColor {
                        index: 2,
                        color: Rgb {
                            r: 0x44,
                            g: 0x55,
                            b: 0x66,
                        },
                    },
                ],
            ),
            ("\x1b]4;7;?\x07", vec![Action::QueryColor(7)]),
            ("\x1b]4;8;?\x1b\\", vec![Action::QueryColor(8)]),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn set_dynamic_standard_colors_with_queries() {
        let cases = vec![
            (
                "\x1b]10;#010203\x07",
                vec![Action::SetColor {
                    index: StdColor::Foreground as usize,
                    color: Rgb {
                        r: 0x01,
                        g: 0x02,
                        b: 0x03,
                    },
                }],
            ),
            (
                "\x1b]11;rgb:aa/bb/cc\x07",
                vec![Action::SetColor {
                    index: StdColor::Background as usize,
                    color: Rgb {
                        r: 0xAA,
                        g: 0xBB,
                        b: 0xCC,
                    },
                }],
            ),
            (
                "\x1b]12;0x172b3f\x07",
                vec![Action::SetColor {
                    index: StdColor::Cursor as usize,
                    color: Rgb {
                        r: 0x17,
                        g: 0x2B,
                        b: 0x3F,
                    },
                }],
            ),
            (
                "\x1b]10;?\x1b\\",
                vec![Action::QueryColor(StdColor::Foreground as usize)],
            ),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn set_mouse_cursor_icon_and_ignore_invalid() {
        let cases = vec![
            (
                "\x1b]22;pointer\x07",
                vec![Action::SetCursorIcon(CursorIcon::Pointer)],
            ),
            ("\x1b]22;unknown\x07", vec![]),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn set_cursor_shape_variants() {
        let cases = vec![
            (
                "\x1b]50;CursorShape=0\x07",
                vec![Action::SetCursorShape(CursorShape::Block)],
            ),
            (
                "\x1b]50;CursorShape=1\x07",
                vec![Action::SetCursorShape(CursorShape::Beam)],
            ),
            (
                "\x1b]50;CursorShape=2\x07",
                vec![Action::SetCursorShape(CursorShape::Underline)],
            ),
            ("\x1b]50;CursorShape=9\x07", vec![]),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn reset_indexed_colors_all_and_subset() {
        let all_colors = 0..256;

        let cases = vec![
            ("\x1b]104\x07", all_colors.map(Action::ResetColor).collect()),
            (
                "\x1b]104;1;3\x1b\\",
                vec![Action::ResetColor(1), Action::ResetColor(3)],
            ),
        ];

        for (input, expected) in cases {
            let actual = RecordingActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }
}
