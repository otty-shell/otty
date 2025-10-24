use log::debug;
use otty_vte::CsiParam;

use crate::attributes::Attr;
use crate::color::{Color, Rgb, StdColor};
use crate::cursor::{CursorShape, CursorStyle};
use crate::mode::{
    ClearMode, KeyboardModes, KeyboardModesApplyBehavior, LineClearMode, Mode,
    ModifyOtherKeys, PrivateMode, ScpCharPath, ScpUpdateMode, TabClearMode,
};
use crate::parser::ParserState;
use crate::sync::{SYNC_UPDATE_TIMEOUT, Timeout};
use crate::{Actor, NamedPrivateMode};

/// Operating system command with raw arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CSI {
    /// ICH
    InsertBlank(usize),
    /// CUU
    CursorUp(i64),
    /// CUD
    CursorDown(i64),
    /// VPR
    VerticalPositionRelative(i64),
    /// REP
    RepeatPrecedingCharacter(i64),
    /// DA1
    PrimaryDeviceAttributes,
    /// CUF
    CursorForward(i64),
    /// HPR
    HorizontalPositionRelative(i64),
    /// HPA
    HorizontalPositionAbsolute(i64),
    /// CUB
    CursorBackward(i64),
    /// VPA
    VerticalPositionAbsolute(i64),
    /// CNL
    CursorNextLine(i64),
    /// CPL
    CursorPrecedingLine(i64),
    /// CHA
    CursorHorizontalAbsolute(i64),
    /// DECST8C
    SetTabStops,
    /// TBC
    TabClear(i64),
    /// CUP
    CursorPosition(i32, usize),
    /// HVP
    HorizontalAndVerticalPosition(i32, usize),
    /// SM
    SetMode(Vec<Mode>),
    /// DECSET
    SetModePrivate(Vec<PrivateMode>),
    /// CHT
    CursorHorizontalTabulation(i64),
    /// ED
    EraseDisplay(i64),
    /// EL
    EraseLine(i64),
    /// DECSCA
    SelectCharacterProtectionAttribute(i64, i64),
    /// IL
    InsertLine(i64),
    /// RM
    ResetMode(Vec<Mode>),
    /// DECRST
    ResetModePrivate(Vec<PrivateMode>),
    /// DL
    DeleteLine(i64),
    /// SGR sequences
    SelectGraphicRendition(Vec<u16>),
    SetModifyOtherKeys(Vec<u16>),
    ReportModifyOtherKeys(Vec<u16>),
    /// DSR
    DeviceStatusReport(i64),
    /// DCH
    DeleteCharacter(i64),
    /// DECRQM
    RequestMode(i64),
    RequestModePrivate(i64),
    /// DECSCUSR
    SetCursorStyle(i64),
    /// DECSTBM
    SetTopAndBottomMargin(usize, usize),
    /// SU
    ScrollUp(i64),
    /// SCOSC
    SaveCursor,
    /// SD
    ScrollDown(i64),
    /// Window manipulation sequences
    WindowManipulation(i64),
    /// SCORC
    RestoreCursorPosition,
    ReportKeyboardMode,
    SetKeyboardMode(i64, i64),
    PushKeyboardMode(i64),
    PopKeyboardModes(i64),
    /// ECH Erase Character
    EraseCharacters(i64),
    /// CBT Cursor Backward Tabulation
    CursorBackwardTabulation(i64),
    /// Misc sequences
    Unspecified {
        params: Vec<CsiParam>,
        final_byte: u8,
    },
}

impl From<(&[CsiParam], &[u8], u8)> for CSI {
    fn from(value: (&[CsiParam], &[u8], u8)) -> Self {
        let (raw_params, inter, final_byte) = value;

        let parsed = match (final_byte, raw_params) {
            (b'h', [CsiParam::P(b'?'), rest @ ..]) => {
                let modes = parse_params(rest)
                    .into_iter()
                    .map(PrivateMode::from_raw)
                    .collect();

                Self::SetModePrivate(modes)
            },
            (b'h', params) => {
                let modes = parse_params(params)
                    .into_iter()
                    .map(Mode::from_raw)
                    .collect();

                Self::SetMode(modes)
            },
            (b'l', [CsiParam::P(b'?'), rest @ ..]) => {
                let modes = parse_params(rest)
                    .into_iter()
                    .map(PrivateMode::from_raw)
                    .collect();

                Self::ResetModePrivate(modes)
            },
            (b'l', params) => {
                let modes = parse_params(params)
                    .into_iter()
                    .map(Mode::from_raw)
                    .collect();

                Self::ResetMode(modes)
            },
            (b'm', [CsiParam::P(b'?'), rest @ ..]) => {
                Self::ReportModifyOtherKeys(parse_params(rest))
            },
            (b'm', [CsiParam::P(b'>'), rest @ ..]) => {
                Self::SetModifyOtherKeys(parse_params(rest))
            },
            (b'm', params) => {
                Self::SelectGraphicRendition(parse_params(params))
            },
            (b'p', [CsiParam::P(b'$')]) => Self::RequestMode(0),
            (b'p', [CsiParam::P(b'$'), CsiParam::Integer(mode)]) => {
                Self::RequestMode(*mode)
            },
            (b'p', [CsiParam::P(b'?'), CsiParam::P(b'$')]) => {
                Self::RequestModePrivate(0)
            },
            (
                b'p',
                [
                    CsiParam::P(b'?'),
                    CsiParam::P(b'$'),
                    CsiParam::Integer(mode),
                ],
            ) => Self::RequestModePrivate(*mode),
            (b'q', []) => Self::SetCursorStyle(0),
            (b'q', [CsiParam::Integer(shape)]) => Self::SetCursorStyle(*shape),
            (b'q', [CsiParam::Integer(shape), ..]) => {
                Self::SetCursorStyle(*shape)
            },

            (b'u', []) => Self::RestoreCursorPosition,
            (b'u', [CsiParam::P(b'?'), ..]) => Self::ReportKeyboardMode,
            (b'u', [CsiParam::P(b'='), rest @ ..]) => {
                if let (
                    Some(CsiParam::Integer(flags)),
                    Some(CsiParam::P(b';')),
                    Some(CsiParam::Integer(mode)),
                ) = (rest.get(0), rest.get(1), rest.get(2))
                {
                    Self::SetKeyboardMode(*flags, *mode)
                } else {
                    Self::Unspecified {
                        params: rest.to_vec(),
                        final_byte,
                    }
                }
            },
            (b'u', [CsiParam::P(b'>'), rest @ ..]) => {
                if let Some(CsiParam::Integer(flags)) = rest.get(0) {
                    Self::PushKeyboardMode(*flags)
                } else {
                    Self::Unspecified {
                        params: rest.to_vec(),
                        final_byte,
                    }
                }
            },
            (b'u', [CsiParam::P(b'<'), rest @ ..]) => {
                let count = rest
                    .get(1)
                    .and_then(|param| match param {
                        CsiParam::Integer(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(1);
                Self::PopKeyboardModes(count)
            },
            (b'W', [CsiParam::P(b'?'), CsiParam::Integer(5)]) => {
                Self::SetTabStops
            },
            (
                b'k',
                [
                    CsiParam::P(b' '),
                    CsiParam::Integer(char_path),
                    CsiParam::Integer(update_mode),
                ],
            ) => Self::SelectCharacterProtectionAttribute(
                *char_path,
                *update_mode,
            ),
            (b'k', [CsiParam::P(b' '), _, CsiParam::Integer(update_mode)]) => {
                Self::SelectCharacterProtectionAttribute(0, *update_mode)
            },
            (b'k', [CsiParam::P(b' '), CsiParam::Integer(char_path)]) => {
                Self::SelectCharacterProtectionAttribute(*char_path, 0)
            },
            (b'k', [CsiParam::P(b' '), ..]) => {
                Self::SelectCharacterProtectionAttribute(0, 0)
            },

            (b'@', [CsiParam::Integer(count)]) => {
                Self::InsertBlank(*count as usize)
            },

            (b'A', []) => Self::CursorUp(1),
            (b'A', [CsiParam::Integer(rows)]) => Self::CursorUp(*rows),
            (b'B', []) => Self::CursorDown(1),
            (b'B', [CsiParam::Integer(rows)]) => Self::CursorDown(*rows),

            (b'e', [CsiParam::Integer(rows)]) => {
                Self::VerticalPositionRelative(*rows)
            },
            (b'b', [CsiParam::Integer(count)]) => {
                Self::RepeatPrecedingCharacter(*count)
            },
            (b'C', []) => Self::CursorForward(1),
            (b'C', [CsiParam::Integer(columns)]) => {
                Self::CursorForward(*columns)
            },
            (b'a', []) => Self::HorizontalPositionRelative(1),
            (b'a', [CsiParam::Integer(columns)]) => {
                Self::HorizontalPositionRelative(*columns)
            },
            (b'c', [..]) => Self::PrimaryDeviceAttributes,
            (b'D', []) => Self::CursorBackward(1),
            (b'D', [CsiParam::Integer(columns)]) => {
                Self::CursorBackward(*columns)
            },
            (b'd', []) => Self::VerticalPositionAbsolute(1),
            (b'd', [CsiParam::Integer(line_num)]) => {
                Self::VerticalPositionAbsolute(*line_num)
            },
            (b'E', []) => Self::CursorNextLine(1),
            (b'E', [CsiParam::Integer(line_count)]) => {
                Self::CursorNextLine(*line_count)
            },
            (b'F', []) => Self::CursorPrecedingLine(1),
            (b'F', [CsiParam::Integer(line_count)]) => {
                Self::CursorPrecedingLine(*line_count)
            },
            (b'G', []) => Self::CursorHorizontalAbsolute(1),
            (b'G', [CsiParam::Integer(column_num)]) => {
                Self::CursorHorizontalAbsolute(*column_num)
            },
            (b'`', []) => Self::HorizontalPositionAbsolute(1),
            (b'`', [CsiParam::Integer(column_num)]) => {
                Self::HorizontalPositionAbsolute(*column_num)
            },

            (b'g', []) => Self::TabClear(0),
            (b'g', [CsiParam::Integer(mode)]) => Self::TabClear(*mode),
            (b'H', []) => Self::CursorPosition(1, 1),
            (
                b'H',
                [
                    CsiParam::Integer(y),
                    CsiParam::P(b';'),
                    CsiParam::Integer(x),
                ],
            ) => Self::CursorPosition(*y as i32, *x as usize),
            (
                b'f',
                [
                    CsiParam::Integer(y),
                    CsiParam::P(b';'),
                    CsiParam::Integer(x),
                ],
            ) => Self::HorizontalAndVerticalPosition(*y as i32, *x as usize),
            (b'I', []) => Self::CursorHorizontalTabulation(1),
            (b'I', [CsiParam::Integer(count)]) => {
                Self::CursorHorizontalTabulation(*count)
            },
            (b'J', []) => Self::EraseDisplay(0),
            (b'J', [CsiParam::Integer(mode)]) => Self::EraseDisplay(*mode),
            (b'K', []) => Self::EraseLine(0),
            (b'K', [CsiParam::Integer(mode)]) => Self::EraseLine(*mode),
            (b'L', []) => Self::InsertLine(1),
            (b'L', [CsiParam::Integer(count)]) => Self::InsertLine(*count),
            (b'M', []) => Self::DeleteLine(1),
            (b'M', [CsiParam::Integer(count)]) => Self::DeleteLine(*count),
            (b'n', []) => Self::DeviceStatusReport(0),
            (b'n', [CsiParam::Integer(report)]) => {
                Self::DeviceStatusReport(*report)
            },
            (b'P', []) => Self::DeleteCharacter(1),
            (b'P', [CsiParam::Integer(count)]) => Self::DeleteCharacter(*count),

            (
                b'r',
                [
                    CsiParam::Integer(top),
                    CsiParam::P(b';'),
                    CsiParam::Integer(bottom),
                ],
            ) => Self::SetTopAndBottomMargin(*top as usize, *bottom as usize),
            (b'S', []) => Self::ScrollUp(1),
            (b'S', [CsiParam::Integer(count)]) => Self::ScrollUp(*count),
            (b's', [..]) => Self::SaveCursor,
            (b'T', []) => Self::ScrollDown(1),
            (b'T', [CsiParam::Integer(count)]) => Self::ScrollDown(*count),

            (b't', []) => Self::WindowManipulation(1),
            (b't', [CsiParam::Integer(id)]) => Self::WindowManipulation(*id),
            (b't', [CsiParam::Integer(id), ..]) => {
                Self::WindowManipulation(*id)
            },

            (b'X', []) => Self::EraseCharacters(1),
            (b'X', [CsiParam::Integer(count)]) => Self::EraseCharacters(*count),
            (b'Z', []) => Self::CursorBackwardTabulation(1),
            (b'Z', [CsiParam::Integer(count)]) => {
                Self::CursorBackwardTabulation(*count)
            },
            _ => Self::Unspecified {
                params: raw_params.to_vec(),
                final_byte,
            },
        };

        match parsed {
            Self::Unspecified {
                ref params,
                final_byte,
            } => println!(
                "[parsed] action: {:?} {:?} {}",
                params, inter, final_byte as char
            ),
            _ => {},
        }

        parsed
    }
}

pub(crate) fn perform<A: Actor, T: Timeout>(
    actor: &mut A,
    state: &mut ParserState<T>,
    params: &[CsiParam],
    intermediates: &[u8],
    params_truncated: bool,
    byte: u8,
) {
    if params_truncated || intermediates.len() > 2 {
        return unexpected(params, byte);
    }

    match CSI::from((params, intermediates, byte)) {
        CSI::InsertBlank(count) => actor.insert_blank(count),
        CSI::CursorUp(rows) => actor.move_up(rows as usize),
        CSI::CursorDown(rows) => actor.move_down(rows as usize),
        CSI::VerticalPositionRelative(rows) => actor.move_down(rows as usize),
        CSI::RepeatPrecedingCharacter(count) => {
            repeat_preceding_char(actor, state, count)
        },
        CSI::CursorForward(columns) => actor.move_forward(columns as usize),
        CSI::HorizontalPositionRelative(columns) => {
            actor.move_forward(columns as usize)
        },
        CSI::PrimaryDeviceAttributes => {
            actor.identify_terminal(intermediates.first().map(|&i| i as char))
        },
        CSI::CursorBackward(columns) => actor.move_backward(columns as usize),
        CSI::VerticalPositionAbsolute(line_num) => {
            actor.goto_line(line_num as i32 - 1)
        },
        CSI::CursorNextLine(line_count) => {
            actor.move_down_and_cr(line_count as usize)
        },
        CSI::CursorPrecedingLine(line_count) => {
            actor.move_up_and_cr(line_count as usize)
        },
        CSI::CursorHorizontalAbsolute(column_num) => {
            actor.goto_col(column_num as usize - 1)
        },
        CSI::HorizontalPositionAbsolute(column_num) => {
            actor.goto_col(column_num as usize - 1)
        },
        CSI::SetTabStops => actor.set_tabs(8),
        CSI::TabClear(mode_index) => {
            let mode = match mode_index {
                0 => TabClearMode::Current,
                3 => TabClearMode::All,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.clear_tabs(mode);
        },
        CSI::HorizontalAndVerticalPosition(y, x) => actor.goto(y - 1, x - 1),
        CSI::CursorPosition(y, x) => actor.goto(y - 1, x - 1),
        CSI::SetMode(modes) => {
            for mode in modes {
                actor.set_mode(mode);
            }
        },
        CSI::SetModePrivate(modes) => {
            for mode in modes {
                if mode == PrivateMode::Named(NamedPrivateMode::SyncUpdate) {
                    state.timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
                    state.terminated = true;
                }

                actor.set_private_mode(mode);
            }
        },
        CSI::ResetMode(modes) => {
            for mode in modes {
                actor.unset_mode(mode);
            }
        },
        CSI::ResetModePrivate(modes) => {
            for mode in modes {
                actor.unset_private_mode(mode);
            }
        },
        CSI::CursorHorizontalTabulation(count) => {
            actor.move_forward_tabs(count as u16)
        },
        CSI::EraseDisplay(mode_index) => {
            let mode = match mode_index {
                0 => ClearMode::Below,
                1 => ClearMode::Above,
                2 => ClearMode::All,
                3 => ClearMode::Saved,
                _ => {
                    return unexpected(params, byte);
                },
            };
            println!("{:?}", mode);

            actor.clear_screen(mode);
        },
        CSI::EraseLine(mode_index) => {
            let mode = match mode_index {
                0 => LineClearMode::Right,
                1 => LineClearMode::Left,
                2 => LineClearMode::All,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.clear_line(mode);
        },
        CSI::SelectCharacterProtectionAttribute(
            char_path_index,
            update_mode_index,
        ) => {
            // SCP control.
            let char_path = match char_path_index {
                0 => ScpCharPath::Default,
                1 => ScpCharPath::LTR,
                2 => ScpCharPath::RTL,
                _ => {
                    return unexpected(params, byte);
                },
            };

            let update_mode = match update_mode_index {
                0 => ScpUpdateMode::ImplementationDependant,
                1 => ScpUpdateMode::DataToPresentation,
                2 => ScpUpdateMode::PresentationToData,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.set_scp(char_path, update_mode);
        },
        CSI::InsertLine(count) => actor.insert_blank_lines(count as usize),
        CSI::DeleteLine(count) => actor.delete_lines(count as usize),
        CSI::SelectGraphicRendition(params) => {
            if params.is_empty() {
                actor.terminal_attribute(Attr::Reset);
            } else {
                attrs_from_sgr_parameters(actor, params);
            }
        },
        CSI::SetModifyOtherKeys(vals) => {
            if vals[0] == 4 {
                let mode = match vals[1] {
                    0 => ModifyOtherKeys::Reset,
                    1 => ModifyOtherKeys::EnableExceptWellDefined,
                    2 => ModifyOtherKeys::EnableAll,
                    _ => return unexpected(params, byte),
                };

                actor.set_modify_other_keys(mode);
            } else {
                unexpected(params, byte)
            }
        },
        CSI::ReportModifyOtherKeys(vals) => {
            if vals[0] == 4 {
                actor.report_modify_other_keys();
            } else {
                unexpected(params, byte);
            }
        },
        CSI::DeviceStatusReport(report) => actor.device_status(report as usize),
        CSI::DeleteCharacter(count) => actor.delete_chars(count as usize),
        CSI::RequestMode(raw_mode) => {
            actor.report_mode(Mode::from_raw(raw_mode as u16));
        },
        CSI::RequestModePrivate(raw_mode) => {
            actor.report_private_mode(PrivateMode::from_raw(raw_mode as u16));
        },
        CSI::SetCursorStyle(raw_shape) => {
            let shape = match raw_shape {
                0 => None,
                1 | 2 => Some(CursorShape::Block),
                3 | 4 => Some(CursorShape::Underline),
                5 | 6 => Some(CursorShape::Beam),
                _ => {
                    return unexpected(params, byte);
                },
            };
            let cursor_style = shape.map(|shape| CursorStyle {
                shape,
                blinking: raw_shape % 2 == 1,
            });

            actor.set_cursor_style(cursor_style);
        },
        CSI::SetTopAndBottomMargin(top, bottom) => {
            actor.set_scrolling_region(top, Some(bottom));
        },
        CSI::ScrollUp(count) => actor.scroll_up(count as usize),
        CSI::SaveCursor => actor.save_cursor_position(),
        CSI::ScrollDown(count) => actor.scroll_down(count as usize),
        CSI::WindowManipulation(id) => match id {
            14 => actor.text_area_size_pixels(),
            18 => actor.text_area_size_chars(),
            22 => actor.push_title(),
            23 => actor.pop_title(),
            _ => unexpected(params, byte),
        },
        CSI::RestoreCursorPosition => actor.restore_cursor_position(),
        CSI::ReportKeyboardMode => actor.report_keyboard_mode(),
        CSI::SetKeyboardMode(flags, behav) => {
            let mode = KeyboardModes::from_bits_truncate(flags as u8);
            let behavior = match behav {
                3 => KeyboardModesApplyBehavior::Difference,
                2 => KeyboardModesApplyBehavior::Union,
                // Default is replace.
                _ => KeyboardModesApplyBehavior::Replace,
            };
            actor.set_keyboard_mode(mode, behavior);
        },
        CSI::PushKeyboardMode(flags) => {
            let mode = KeyboardModes::from_bits_truncate(flags as u8);
            actor.push_keyboard_mode(mode);
        },
        CSI::PopKeyboardModes(flags) => actor.pop_keyboard_modes(flags as u16),
        CSI::EraseCharacters(count) => actor.erase_chars(count as usize),
        CSI::CursorBackwardTabulation(count) => {
            actor.move_backward_tabs(count as u16)
        },
        CSI::Unspecified { params, final_byte } => {
            unexpected(params.as_slice(), final_byte)
        },
        _ => unexpected(params, byte),
    }
}

#[inline]
fn attrs_from_sgr_parameters<A: Actor>(handler: &mut A, params: Vec<u16>) {
    let mut iter = params.into_iter().peekable();

    while let Some(param) = iter.next() {
        let attr = match param {
            0 => Some(Attr::Reset),
            1 => Some(Attr::Bold),
            2 => Some(Attr::Dim),
            3 => Some(Attr::Italic),
            4 => match iter.peek().copied() {
                Some(0) => {
                    iter.next();
                    Some(Attr::CancelUnderline)
                },
                Some(2) => {
                    iter.next();
                    Some(Attr::DoubleUnderline)
                },
                Some(3) => {
                    iter.next();
                    Some(Attr::Undercurl)
                },
                Some(4) => {
                    iter.next();
                    Some(Attr::DottedUnderline)
                },
                Some(5) => {
                    iter.next();
                    Some(Attr::DashedUnderline)
                },
                _ => Some(Attr::Underline),
            },
            5 => Some(Attr::BlinkSlow),
            6 => Some(Attr::BlinkFast),
            7 => Some(Attr::Reverse),
            8 => Some(Attr::Hidden),
            9 => Some(Attr::Strike),
            21 => Some(Attr::CancelBold),
            22 => Some(Attr::CancelBoldDim),
            23 => Some(Attr::CancelItalic),
            24 => Some(Attr::CancelUnderline),
            25 => Some(Attr::CancelBlink),
            27 => Some(Attr::CancelReverse),
            28 => Some(Attr::CancelHidden),
            29 => Some(Attr::CancelStrike),
            30..=37 => standard_color(param)
                .map(|color| Attr::Foreground(Color::Std(color))),
            38 => parse_extended_color(&mut iter).map(Attr::Foreground),
            39 => Some(Attr::Foreground(Color::Std(StdColor::Foreground))),
            40..=47 => standard_color(param - 10)
                .map(|color| Attr::Background(Color::Std(color))),
            48 => parse_extended_color(&mut iter).map(Attr::Background),
            49 => Some(Attr::Background(Color::Std(StdColor::Background))),
            58 => parse_extended_color(&mut iter)
                .map(|color| Attr::UnderlineColor(Some(color))),
            59 => Some(Attr::UnderlineColor(None)),
            90..=97 => bright_color(param)
                .map(|color| Attr::Foreground(Color::Std(color))),
            100..=107 => bright_color(param - 10)
                .map(|color| Attr::Background(Color::Std(color))),
            _ => None,
        };

        if let Some(attr) = attr {
            handler.terminal_attribute(attr);
        }
    }
}

fn standard_color(code: u16) -> Option<StdColor> {
    match code {
        30 => Some(StdColor::Black),
        31 => Some(StdColor::Red),
        32 => Some(StdColor::Green),
        33 => Some(StdColor::Yellow),
        34 => Some(StdColor::Blue),
        35 => Some(StdColor::Magenta),
        36 => Some(StdColor::Cyan),
        37 => Some(StdColor::White),
        _ => None,
    }
}

fn bright_color(code: u16) -> Option<StdColor> {
    match code {
        90 => Some(StdColor::BrightBlack),
        91 => Some(StdColor::BrightRed),
        92 => Some(StdColor::BrightGreen),
        93 => Some(StdColor::BrightYellow),
        94 => Some(StdColor::BrightBlue),
        95 => Some(StdColor::BrightMagenta),
        96 => Some(StdColor::BrightCyan),
        97 => Some(StdColor::BrightWhite),
        _ => None,
    }
}

fn parse_extended_color<I>(iter: &mut I) -> Option<Color>
where
    I: Iterator<Item = u16>,
{
    match iter.next() {
        Some(5) => {
            let index = iter.next()?;
            (index <= u8::MAX as u16).then_some(Color::Indexed(index as u8))
        },
        Some(2) => {
            let r = iter.next()?;
            let g = iter.next()?;
            let b = iter.next()?;

            if r > u8::MAX as u16 || g > u8::MAX as u16 || b > u8::MAX as u16 {
                return None;
            }

            Some(Color::TrueColor(Rgb {
                r: r as u8,
                g: g as u8,
                b: b as u8,
            }))
        },
        _ => None,
    }
}

fn repeat_preceding_char<'a, A, T>(
    actor: &mut A,
    state: &mut ParserState<T>,
    count: i64,
) where
    T: Timeout,
    A: Actor,
{
    if let Some(c) = state.last_preceding_char {
        for _ in 0..count {
            actor.print(c);
        }
    } else {
        debug!("tried to repeat with no preceding char");
    }
}

fn next_param_or<'a, I>(default: u16, params_iter: &mut I) -> u16
where
    I: Iterator<Item = &'a CsiParam>,
{
    match params_iter.next() {
        Some(CsiParam::Integer(param)) if *param != 0 => *param as u16,
        _ => default,
    }
}

fn parse_params(params: &[CsiParam]) -> Vec<u16> {
    let mut values = Vec::new();
    let mut pending: Option<u16> = None;

    for param in params.iter() {
        // for param in params.iter().skip(start_idx) {
        match param {
            CsiParam::Integer(value) => {
                let parsed = if (0..=u16::MAX as i64).contains(value) {
                    *value as u16
                } else {
                    0
                };
                pending = Some(parsed);
            },
            CsiParam::P(b';') => {
                values.push(pending.take().unwrap_or(0));
            },
            CsiParam::P(_) => {},
        }
    }

    if let Some(value) = pending {
        values.push(value);
    } else if values.is_empty() {
        values.push(0);
    }

    values
}

fn unexpected(params: &[CsiParam], byte: u8) {
    debug!("[unexpected csi] action: {byte:?}, params: {params:?}",);
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::mode::{Mode, NamedMode, NamedPrivateMode};
//     use otty_vte::{Actor as VteActor, Parser as VteParser};

//     #[derive(Default)]
//     struct RecordingActor {
//         csis: Vec<CSI>,
//     }

//     impl VteActor for RecordingActor {
//         fn print(&mut self, _: char) {}

//         fn execute(&mut self, _: u8) {}

//         fn hook(&mut self, _: &[i64], _: &[u8], _: bool, _: u8) {}

//         fn unhook(&mut self) {}

//         fn put(&mut self, _: u8) {}

//         fn osc_dispatch(&mut self, _: &[&[u8]], _: u8) {}

//         fn csi_dispatch(
//             &mut self,
//             params: &[CsiParam],
//             intermediates: &[u8],
//             parameters_truncated: bool,
//             byte: u8,
//         ) {
//             assert!(
//                 !parameters_truncated,
//                 "unexpected parameter truncation for params {params:?}",
//             );
//             assert!(
//                 intermediates.is_empty(),
//                 "unexpected intermediates delivered separately: {intermediates:?}"
//             );
//             self.csis.push(CSI::from((params, intermediates, byte)));
//         }

//         fn esc_dispatch(&mut self, _: &[i64], _: &[u8], _: bool, _: u8) {}
//     }

//     impl RecordingActor {
//         fn into_csis(self) -> Vec<CSI> {
//             self.csis
//         }
//     }

//     fn collect_csis(bytes: &[u8]) -> Vec<CSI> {
//         let mut parser = VteParser::new();
//         let mut actor = RecordingActor::default();
//         parser.advance(bytes, &mut actor);
//         actor.into_csis()
//     }

//     fn parse_single_csi(bytes: &[u8]) -> CSI {
//         let mut csis = collect_csis(bytes);
//         assert_eq!(
//             csis.len(),
//             1,
//             "expected a single CSI action for sequence {bytes:?}"
//         );
//         csis.pop().unwrap()
//     }

//     #[track_caller]
//     fn assert_csi(bytes: &[u8], expected: CSI) {
//         let parsed = parse_single_csi(bytes);
//         assert_eq!(parsed, expected, "sequence {bytes:?} parsed incorrectly");
//     }

//     #[test]
//     fn parses_cursor_movement_sequences() {
//         let cases: Vec<(&[u8], CSI)> = vec![
//             // (b"\x1b[4@", CSI::InsertBlank),
//             (b"\x1b[3A", CSI::CursorUp),
//             (b"\x1b[2B", CSI::CursorDown),
//             (b"\x1b[5C", CSI::CursorForward),
//             (b"\x1b[4D", CSI::CursorBackward),
//             (b"\x1b[3a", CSI::HorizontalPositionRelative),
//             (b"\x1b[12d", CSI::VerticalPositionAbsolute),
//             (b"\x1b[2e", CSI::VerticalPositionRelative),
//             (b"\x1b[5b", CSI::RepeatPrecedingCharacter),
//             (b"\x1b[2E", CSI::CursorNextLine),
//             (b"\x1b[2F", CSI::CursorPrecedingLine),
//             (b"\x1b[9G", CSI::CursorHorizontalAbsolute),
//             (b"\x1b[5\x60", CSI::HorizontalPositionAbsolute),
//             // (b"\x1b[7;9H", CSI::CursorPosition),
//             // (b"\x1b[7;9f", CSI::HorizontalAndVerticalPosition),
//             (b"\x1b[3I", CSI::CursorHorizontalTabulation),
//         ];

//         for (bytes, expected) in cases {
//             assert_csi(bytes, expected);
//         }
//     }

//     #[test]
//     fn parses_mode_management_sequences() {
//         let cases: Vec<(&[u8], CSI)> = vec![
//             (b"\x1b[4h", CSI::SetMode(vec![NamedMode::Insert.into()])),
//             (
//                 b"\x1b[?1049h",
//                 CSI::SetModePrivate(vec![
//                     NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
//                 ]),
//             ),
//             (b"\x1b[4l", CSI::ResetMode(vec![NamedMode::Insert.into()])),
//             (
//                 b"\x1b[?1049l",
//                 CSI::ResetModePrivate(vec![
//                     NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
//                 ]),
//             ),
//             (b"\x1b[=1;2u", CSI::SetKeyboardMode(1, 2)),
//             (b"\x1b[>5u", CSI::PushKeyboardMode(5)),
//             (b"\x1b[<u", CSI::PopKeyboardModes(1)),
//             (b"\x1b[<3u", CSI::PopKeyboardModes(3)),
//             (b"\x1b[?u", CSI::ReportKeyboardMode),
//             (
//                 b"\x1b[?25h",
//                 CSI::SetModePrivate(vec![NamedPrivateMode::ShowCursor.into()]),
//             ),
//             (
//                 b"\x1b[?25l",
//                 CSI::ResetModePrivate(vec![
//                     NamedPrivateMode::ShowCursor.into(),
//                 ]),
//             ),
//             (b"\x1b[5h", CSI::SetMode(vec![Mode::Unknown(5)])),
//             (b"\x1b[5l", CSI::ResetMode(vec![Mode::Unknown(5)])),
//         ];

//         for (bytes, expected) in cases {
//             assert_csi(bytes, expected);
//         }
//     }

//     #[test]
//     fn parses_character_and_display_sequences() {
//         let cases: Vec<(&[u8], CSI)> = vec![
//             (b"\x1b[c", CSI::PrimaryDeviceAttributes),
//             (b"\x1b[2J", CSI::EraseDisplay),
//             (b"\x1b[2K", CSI::EraseLine),
//             (b"\x1b[3L", CSI::InsertLine),
//             (b"\x1b[2M", CSI::DeleteLine),
//             (b"\x1b[2P", CSI::DeleteCharacter),
//             (b"\x1b[31m", CSI::SelectGraphicRendition(vec![31])),
//             (b"\x1b[>4;2m", CSI::SetModifyOtherKeys(vec![4, 2])),
//             (b"\x1b[?1m", CSI::ReportModifyOtherKeys(vec![1])),
//             (b"\x1b[69$p", CSI::RequestMode),
//             (b"\x1b[?69$p", CSI::RequestModePrivate),
//             (b"\x1b[1\x22q", CSI::SelectCharacterProtectionAttribute),
//             (b"\x1b[2 q", CSI::SetCursorStyle),
//             (b"\x1b[5W", CSI::SetTabStops),
//             (b"\x1b[0g", CSI::TabClear),
//             (b"\x1b[1;24r", CSI::SetTopAndBottomMargin),
//             (b"\x1b[2S", CSI::ScrollUp),
//             (b"\x1b[2T", CSI::ScrollDown),
//             (b"\x1b[s", CSI::SaveCursor),
//             (b"\x1b[u", CSI::RestoreCursorPosition),
//             (b"\x1b[2;0;0t", CSI::WindowManipulation),
//             (b"\x1b[5n", CSI::DeviceStatusReport),
//         ];

//         for (bytes, expected) in cases {
//             assert_csi(bytes, expected);
//         }
//     }

//     #[test]
//     fn parses_fallback_to_unspecified() {
//         let parsed = parse_single_csi(b"\x1b[2~");
//         assert_eq!(
//             parsed,
//             CSI::Unspecified {
//                 params: vec![CsiParam::Integer(2)],
//                 final_byte: b'~',
//             }
//         );
//     }
// }
