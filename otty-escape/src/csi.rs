use log::debug;
use otty_vte::CsiParam;

use crate::actor::Action;
use crate::attributes::CharacterAttribute;
use crate::color::{Color, StdColor};
use crate::cursor::{CursorShape, CursorStyle};
use crate::keyboard::{
    KeyboardMode, KeyboardModeApplyBehavior, ModifyOtherKeysState,
};
use crate::mode::{
    ClearMode, LineClearMode, Mode, PrivateMode, ScpCharPath, ScpUpdateMode,
    TabClearMode,
};
use crate::parser::ParserState;
use crate::sync::{SYNC_UPDATE_TIMEOUT, Timeout};
use crate::{Actor, NamedPrivateMode, parse_sgr_color};

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
    PrimaryDeviceAttributes(i64),
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
                    .first()
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
            (b'c', [CsiParam::Integer(attr)]) => {
                Self::PrimaryDeviceAttributes(*attr)
            },
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
            } => debug!(
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
    if params_truncated {
        return unexpected(params, byte);
    }

    match CSI::from((params, intermediates, byte)) {
        CSI::InsertBlank(count) => actor.handle(Action::InsertBlank(count)),
        CSI::CursorUp(rows) => actor.handle(Action::MoveUp {
            rows: rows as usize,
            carrage_return_needed: false,
        }),
        CSI::CursorDown(rows) => actor.handle(Action::MoveDown {
            rows: rows as usize,
            carrage_return_needed: false,
        }),
        CSI::VerticalPositionRelative(rows) => actor.handle(Action::MoveDown {
            rows: rows as usize,
            carrage_return_needed: false,
        }),
        CSI::RepeatPrecedingCharacter(count) => {
            if let Some(c) = state.last_preceding_char {
                for _ in 0..count {
                    actor.handle(Action::Print(c));
                }
            } else {
                debug!("tried to repeat with no preceding char");
            }
        },
        CSI::CursorForward(columns) => {
            actor.handle(Action::MoveForward(columns as usize))
        },
        CSI::HorizontalPositionRelative(columns) => {
            actor.handle(Action::MoveForward(columns as usize))
        },
        CSI::PrimaryDeviceAttributes(attr) => {
            actor.handle(Action::IdentifyTerminal(char::from_u32(attr as u32)))
        },
        CSI::CursorBackward(columns) => {
            actor.handle(Action::MoveBackward(columns as usize))
        },
        CSI::VerticalPositionAbsolute(line_num) => {
            actor.handle(Action::GotoRow(line_num as i32 - 1))
        },
        CSI::CursorNextLine(line_count) => actor.handle(Action::MoveDown {
            rows: line_count as usize,
            carrage_return_needed: true,
        }),
        CSI::CursorPrecedingLine(line_count) => actor.handle(Action::MoveUp {
            rows: line_count as usize,
            carrage_return_needed: true,
        }),
        CSI::CursorHorizontalAbsolute(column_num) => {
            actor.handle(Action::GotoColumn(column_num as usize - 1))
        },
        CSI::HorizontalPositionAbsolute(column_num) => {
            actor.handle(Action::GotoColumn(column_num as usize - 1))
        },
        CSI::SetTabStops => actor.handle(Action::SetTabs(8)),
        CSI::TabClear(mode_index) => {
            let mode = match mode_index {
                0 => TabClearMode::Current,
                3 => TabClearMode::All,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.handle(Action::ClearTabs(mode));
        },
        CSI::HorizontalAndVerticalPosition(y, x) => {
            actor.handle(Action::Goto(y - 1, x - 1))
        },
        CSI::CursorPosition(y, x) => actor.handle(Action::Goto(y - 1, x - 1)),
        CSI::SetMode(modes) => {
            for mode in modes {
                actor.handle(Action::SetMode(mode));
            }
        },
        CSI::SetModePrivate(modes) => {
            for mode in modes {
                if mode == PrivateMode::Named(NamedPrivateMode::SyncUpdate) {
                    state.timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
                    state.terminated = true;
                }

                actor.handle(Action::SetPrivateMode(mode));
            }
        },
        CSI::ResetMode(modes) => {
            for mode in modes {
                actor.handle(Action::UnsetMode(mode));
            }
        },
        CSI::ResetModePrivate(modes) => {
            for mode in modes {
                actor.handle(Action::UnsetPrivateMode(mode));
            }
        },
        CSI::CursorHorizontalTabulation(count) => {
            actor.handle(Action::MoveForwardTabs(count as u16))
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

            actor.handle(Action::ClearScreen(mode));
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

            actor.handle(Action::ClearLine(mode));
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

            actor.handle(Action::SetSCP(char_path, update_mode));
        },
        CSI::InsertLine(count) => {
            actor.handle(Action::InsertBlankLines(count as usize))
        },
        CSI::DeleteLine(count) => {
            actor.handle(Action::DeleteLines(count as usize))
        },
        CSI::SelectGraphicRendition(params) => {
            if params.is_empty() {
                actor.handle(Action::SetCharacterAttribute(
                    CharacterAttribute::Reset,
                ));
            } else {
                attrs_from_sgr_parameters(actor, params);
            }
        },
        CSI::SetModifyOtherKeys(vals) => {
            if vals[0] == 4 {
                let mode = match vals[1] {
                    0 => ModifyOtherKeysState::Reset,
                    1 => ModifyOtherKeysState::EnableExceptWellDefined,
                    2 => ModifyOtherKeysState::EnableAll,
                    _ => return unexpected(params, byte),
                };

                actor.handle(Action::SetModifyOtherKeysState(mode));
            } else {
                unexpected(params, byte)
            }
        },
        CSI::ReportModifyOtherKeys(vals) => {
            if vals[0] == 4 {
                actor.handle(Action::ReportModifyOtherKeysState);
            } else {
                unexpected(params, byte);
            }
        },
        CSI::DeviceStatusReport(report) => {
            actor.handle(Action::ReportDeviceStatus(report as usize))
        },
        CSI::DeleteCharacter(count) => {
            actor.handle(Action::DeleteChars(count as usize))
        },
        CSI::RequestMode(raw_mode) => {
            actor.handle(Action::ReportMode(Mode::from_raw(raw_mode as u16)));
        },
        CSI::RequestModePrivate(raw_mode) => {
            actor.handle(Action::ReportPrivateMode(PrivateMode::from_raw(
                raw_mode as u16,
            )));
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

            actor.handle(Action::SetCursorStyle(cursor_style));
        },
        CSI::SetTopAndBottomMargin(top, bottom) => {
            actor.handle(Action::SetScrollingRegion(top, bottom));
        },
        CSI::ScrollUp(count) => actor.handle(Action::ScrollUp(count as usize)),
        CSI::SaveCursor => actor.handle(Action::SaveCursorPosition),
        CSI::ScrollDown(count) => {
            actor.handle(Action::ScrollDown(count as usize))
        },
        CSI::WindowManipulation(id) => match id {
            14 => actor.handle(Action::RequestTextAreaSizeByPixels),
            18 => actor.handle(Action::RequestTextAreaSizeByChars),
            22 => actor.handle(Action::PushWindowTitle),
            23 => actor.handle(Action::PopWindowTitle),
            _ => unexpected(params, byte),
        },
        CSI::RestoreCursorPosition => {
            actor.handle(Action::RestoreCursorPosition)
        },
        CSI::ReportKeyboardMode => actor.handle(Action::ReportKeyboardMode),
        CSI::SetKeyboardMode(flags, behav) => {
            let mode = KeyboardMode::from_bits_truncate(flags as u8);
            let behavior = match behav {
                3 => KeyboardModeApplyBehavior::Difference,
                2 => KeyboardModeApplyBehavior::Union,
                // Default is replace.
                _ => KeyboardModeApplyBehavior::Replace,
            };
            actor.handle(Action::SetKeyboardMode(mode, behavior));
        },
        CSI::PushKeyboardMode(flags) => {
            let mode = KeyboardMode::from_bits_truncate(flags as u8);
            actor.handle(Action::PushKeyboardMode(mode));
        },
        CSI::PopKeyboardModes(count) => {
            actor.handle(Action::PopKeyboardModes(count as u16))
        },
        CSI::EraseCharacters(count) => {
            actor.handle(Action::EraseChars(count as usize))
        },
        CSI::CursorBackwardTabulation(count) => {
            actor.handle(Action::MoveBackwardTabs(count as u16))
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
            0 => Some(CharacterAttribute::Reset),
            1 => Some(CharacterAttribute::Bold),
            2 => Some(CharacterAttribute::Dim),
            3 => Some(CharacterAttribute::Italic),
            4 => match iter.peek().copied() {
                Some(0) => {
                    iter.next();
                    Some(CharacterAttribute::CancelUnderline)
                },
                Some(2) => {
                    iter.next();
                    Some(CharacterAttribute::DoubleUnderline)
                },
                Some(3) => {
                    iter.next();
                    Some(CharacterAttribute::Undercurl)
                },
                Some(4) => {
                    iter.next();
                    Some(CharacterAttribute::DottedUnderline)
                },
                Some(5) => {
                    iter.next();
                    Some(CharacterAttribute::DashedUnderline)
                },
                _ => Some(CharacterAttribute::Underline),
            },
            5 => Some(CharacterAttribute::BlinkSlow),
            6 => Some(CharacterAttribute::BlinkFast),
            7 => Some(CharacterAttribute::Reverse),
            8 => Some(CharacterAttribute::Hidden),
            9 => Some(CharacterAttribute::Strike),
            21 => Some(CharacterAttribute::CancelBold),
            22 => Some(CharacterAttribute::CancelBoldDim),
            23 => Some(CharacterAttribute::CancelItalic),
            24 => Some(CharacterAttribute::CancelUnderline),
            25 => Some(CharacterAttribute::CancelBlink),
            27 => Some(CharacterAttribute::CancelReverse),
            28 => Some(CharacterAttribute::CancelHidden),
            29 => Some(CharacterAttribute::CancelStrike),
            30 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Black,
            ))),
            31 => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Red)))
            },
            32 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Green,
            ))),
            33 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Yellow,
            ))),
            34 => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Blue)))
            },
            35 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Magenta,
            ))),
            36 => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Cyan)))
            },
            37 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::White,
            ))),
            38 => {
                parse_sgr_color(&mut iter).map(CharacterAttribute::Foreground)
            },
            39 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Foreground,
            ))),
            40 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Black,
            ))),
            41 => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Red)))
            },
            42 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Green,
            ))),
            43 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Yellow,
            ))),
            44 => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Blue)))
            },
            45 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Magenta,
            ))),
            46 => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Cyan)))
            },
            47 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::White,
            ))),
            48 => {
                parse_sgr_color(&mut iter).map(CharacterAttribute::Background)
            },
            49 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Background,
            ))),
            58 => parse_sgr_color(&mut iter)
                .map(|color| CharacterAttribute::UnderlineColor(Some(color))),
            59 => Some(CharacterAttribute::UnderlineColor(None)),
            90 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightBlack,
            ))),
            91 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightRed,
            ))),
            92 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightGreen,
            ))),
            93 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightYellow,
            ))),
            94 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightBlue,
            ))),
            95 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightMagenta,
            ))),
            96 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightCyan,
            ))),
            97 => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightWhite,
            ))),
            100 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightBlack,
            ))),
            101 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightRed,
            ))),
            102 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightGreen,
            ))),
            103 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightYellow,
            ))),
            104 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightBlue,
            ))),
            105 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightMagenta,
            ))),
            106 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightCyan,
            ))),
            107 => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightWhite,
            ))),
            _ => None,
        };

        if let Some(attr) = attr {
            handler.handle(Action::SetCharacterAttribute(attr));
        }
    }
}

fn parse_params(params: &[CsiParam]) -> Vec<u16> {
    let mut values = Vec::new();
    let mut pending: Option<u16> = None;

    for param in params.iter() {
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
