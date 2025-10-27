use log::debug;
use otty_vte::CsiParam;

use crate::actor::{Action, TerminalControlAction};
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

// impl From<(&[CsiParam], &[u8], u8)> for CSI {
//     fn from(value: (&[CsiParam], &[u8], u8)) -> Self {
//         let (raw_params, inter, final_byte) = value;

//         let parsed = match (final_byte, raw_params) {
//             (b'h', [CsiParam::P(b'?'), rest @ ..]) => Self::SetModePrivate(
//                 parse_params_with(rest, PrivateMode::from_raw),
//             ),
//             (b'h', params) => {
//                 Self::SetMode(parse_params_with(params, Mode::from_raw))
//             },
//             (b'l', [CsiParam::P(b'?'), rest @ ..]) => Self::ResetModePrivate(
//                 parse_params_with(rest, PrivateMode::from_raw),
//             ),
//             (b'l', params) => {
//                 Self::ResetMode(parse_params_with(params, Mode::from_raw))
//             },
//             (b'm', [CsiParam::P(b'?'), rest @ ..]) => {
//                 Self::ReportModifyOtherKeys(parse_params(rest))
//             },
//             (b'm', [CsiParam::P(b'>'), rest @ ..]) => {
//                 Self::SetModifyOtherKeys(parse_params(rest))
//             },
//             (b'm', params) => {
//                 Self::SelectGraphicRendition(parse_params(params))
//             },
//             (b'p', [CsiParam::P(b'$')]) => Self::RequestMode(0),
//             (b'p', [CsiParam::P(b'$'), CsiParam::Integer(mode)]) => {
//                 Self::RequestMode(*mode)
//             },
//             (b'p', [CsiParam::P(b'?'), CsiParam::P(b'$')]) => {
//                 Self::RequestModePrivate(0)
//             },
//             (
//                 b'p',
//                 [
//                     CsiParam::P(b'?'),
//                     CsiParam::P(b'$'),
//                     CsiParam::Integer(mode),
//                 ],
//             ) => Self::RequestModePrivate(*mode),
//             (b'q', []) => Self::SetCursorStyle(0),
//             (b'q', [CsiParam::Integer(shape)]) => Self::SetCursorStyle(*shape),
//             (b'q', [CsiParam::Integer(shape), ..]) => {
//                 Self::SetCursorStyle(*shape)
//             },

//             (b'u', []) => Self::RestoreCursorPosition,
//             (b'u', [CsiParam::P(b'?'), ..]) => Self::ReportKeyboardMode,
//             (b'u', [CsiParam::P(b'='), rest @ ..]) => {
//                 if let (
//                     Some(CsiParam::Integer(flags)),
//                     Some(CsiParam::P(b';')),
//                     Some(CsiParam::Integer(mode)),
//                 ) = (rest.get(0), rest.get(1), rest.get(2))
//                 {
//                     Self::SetKeyboardMode(*flags, *mode)
//                 } else {
//                     Self::Unspecified {
//                         params: rest.to_vec(),
//                         final_byte,
//                     }
//                 }
//             },
//             (b'u', [CsiParam::P(b'>'), rest @ ..]) => {
//                 if let Some(CsiParam::Integer(flags)) = rest.first() {
//                     Self::PushKeyboardMode(*flags)
//                 } else {
//                     Self::Unspecified {
//                         params: rest.to_vec(),
//                         final_byte,
//                     }
//                 }
//             },
//             (b'u', [CsiParam::P(b'<'), rest @ ..]) => {
//                 let count = rest
//                     .first()
//                     .and_then(|param| match param {
//                         CsiParam::Integer(value) => Some(*value),
//                         _ => None,
//                     })
//                     .unwrap_or(1);

//                 Self::PopKeyboardModes(count)
//             },
//             (b'W', [CsiParam::P(b'?'), CsiParam::Integer(5)]) => {
//                 Self::SetTabStops
//             },
//             (
//                 b'k',
//                 [
//                     CsiParam::P(b' '),
//                     CsiParam::Integer(char_path),
//                     CsiParam::Integer(update_mode),
//                 ],
//             ) => Self::SelectCharacterProtectionAttribute(
//                 *char_path,
//                 *update_mode,
//             ),
//             (b'k', [CsiParam::P(b' '), _, CsiParam::Integer(update_mode)]) => {
//                 Self::SelectCharacterProtectionAttribute(0, *update_mode)
//             },
//             (b'k', [CsiParam::P(b' '), CsiParam::Integer(char_path)]) => {
//                 Self::SelectCharacterProtectionAttribute(*char_path, 0)
//             },
//             (b'k', [CsiParam::P(b' '), ..]) => {
//                 Self::SelectCharacterProtectionAttribute(0, 0)
//             },

//             (b'@', [CsiParam::Integer(count)]) => {
//                 Self::InsertBlank(*count as usize)
//             },
//             (b'A', []) => Self::CursorUp(1),
//             (b'A', [CsiParam::Integer(rows)]) => Self::CursorUp(*rows),
//             (b'B', []) => Self::CursorDown(1),
//             (b'B', [CsiParam::Integer(rows)]) => Self::CursorDown(*rows),

//             (b'e', [CsiParam::Integer(rows)]) => {
//                 Self::VerticalPositionRelative(*rows)
//             },
//             (b'b', [CsiParam::Integer(count)]) => {
//                 Self::RepeatPrecedingCharacter(*count)
//             },
//             (b'C', []) => Self::CursorForward(1),
//             (b'C', [CsiParam::Integer(columns)]) => {
//                 Self::CursorForward(*columns)
//             },
//             (b'a', []) => Self::HorizontalPositionRelative(1),
//             (b'a', [CsiParam::Integer(columns)]) => {
//                 Self::HorizontalPositionRelative(*columns)
//             },
//             (b'c', [CsiParam::Integer(attr)]) => {
//                 Self::PrimaryDeviceAttributes(*attr)
//             },
//             (b'D', []) => Self::CursorBackward(1),
//             (b'D', [CsiParam::Integer(columns)]) => {
//                 Self::CursorBackward(*columns)
//             },
//             (b'd', []) => Self::VerticalPositionAbsolute(1),
//             (b'd', [CsiParam::Integer(line_num)]) => {
//                 Self::VerticalPositionAbsolute(*line_num)
//             },
//             (b'E', []) => Self::CursorNextLine(1),
//             (b'E', [CsiParam::Integer(line_count)]) => {
//                 Self::CursorNextLine(*line_count)
//             },
//             (b'F', []) => Self::CursorPrecedingLine(1),
//             (b'F', [CsiParam::Integer(line_count)]) => {
//                 Self::CursorPrecedingLine(*line_count)
//             },
//             (b'G', []) => Self::CursorHorizontalAbsolute(1),
//             (b'G', [CsiParam::Integer(column_num)]) => {
//                 Self::CursorHorizontalAbsolute(*column_num)
//             },
//             (b'`', []) => Self::HorizontalPositionAbsolute(1),
//             (b'`', [CsiParam::Integer(column_num)]) => {
//                 Self::HorizontalPositionAbsolute(*column_num)
//             },

//             (b'g', []) => Self::TabClear(0),
//             (b'g', [CsiParam::Integer(mode)]) => Self::TabClear(*mode),
//             (b'H', []) => Self::CursorPosition(1, 1),
//             (
//                 b'H',
//                 [
//                     CsiParam::Integer(y),
//                     CsiParam::P(b';'),
//                     CsiParam::Integer(x),
//                 ],
//             ) => Self::CursorPosition(*y as i32, *x as usize),
//             (
//                 b'f',
//                 [
//                     CsiParam::Integer(y),
//                     CsiParam::P(b';'),
//                     CsiParam::Integer(x),
//                 ],
//             ) => Self::HorizontalAndVerticalPosition(*y as i32, *x as usize),
//             (b'I', []) => Self::CursorHorizontalTabulation(1),
//             (b'I', [CsiParam::Integer(count)]) => {
//                 Self::CursorHorizontalTabulation(*count)
//             },
//             (b'J', []) => Self::EraseDisplay(0),
//             (b'J', [CsiParam::Integer(mode)]) => Self::EraseDisplay(*mode),
//             (b'K', []) => Self::EraseLine(0),
//             (b'K', [CsiParam::Integer(mode)]) => Self::EraseLine(*mode),
//             (b'L', []) => Self::InsertLine(1),
//             (b'L', [CsiParam::Integer(count)]) => Self::InsertLine(*count),
//             (b'M', []) => Self::DeleteLine(1),
//             (b'M', [CsiParam::Integer(count)]) => Self::DeleteLine(*count),
//             (b'n', []) => Self::DeviceStatusReport(0),
//             (b'n', [CsiParam::Integer(report)]) => {
//                 Self::DeviceStatusReport(*report)
//             },
//             (b'P', []) => Self::DeleteCharacter(1),
//             (b'P', [CsiParam::Integer(count)]) => Self::DeleteCharacter(*count),

//             (
//                 b'r',
//                 [
//                     CsiParam::Integer(top),
//                     CsiParam::P(b';'),
//                     CsiParam::Integer(bottom),
//                 ],
//             ) => Self::SetTopAndBottomMargin(*top as usize, *bottom as usize),
//             (b'S', []) => Self::ScrollUp(1),
//             (b'S', [CsiParam::Integer(count)]) => Self::ScrollUp(*count),
//             (b's', [..]) => Self::SaveCursor,
//             (b'T', []) => Self::ScrollDown(1),
//             (b'T', [CsiParam::Integer(count)]) => Self::ScrollDown(*count),

//             (b't', []) => Self::WindowManipulation(1),
//             (b't', [CsiParam::Integer(id)]) => Self::WindowManipulation(*id),
//             (b't', [CsiParam::Integer(id), ..]) => {
//                 Self::WindowManipulation(*id)
//             },

//             (b'X', []) => Self::EraseCharacters(1),
//             (b'X', [CsiParam::Integer(count)]) => Self::EraseCharacters(*count),
//             (b'Z', []) => Self::CursorBackwardTabulation(1),
//             (b'Z', [CsiParam::Integer(count)]) => {
//                 Self::CursorBackwardTabulation(*count)
//             },
//             _ => Self::Unspecified {
//                 params: raw_params.to_vec(),
//                 final_byte,
//             },
//         };

//         match parsed {
//             Self::Unspecified {
//                 ref params,
//                 final_byte,
//             } => debug!(
//                 "[parsed] action: {:?} {:?} {}",
//                 params, inter, final_byte as char
//             ),
//             _ => {},
//         }

//         parsed
//     }
// }

#[inline]
pub(crate) fn perform<A: Actor>(
    actor: &mut A,
    state: &mut ParserState,
    raw_params: &[CsiParam],
    params_truncated: bool,
    byte: u8,
) {
    if params_truncated {
        return unexpected(raw_params, byte);
    }

    match (byte, raw_params) {
        (b'h', [CsiParam::P(b'?'), rest @ ..]) => {
            for param in rest {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = PrivateMode::from_raw(mode as u16);
                        if mode
                            == PrivateMode::Named(NamedPrivateMode::SyncUpdate)
                        {
                            actor.begin_sync();
                        }

                        actor.handle(
                            TerminalControlAction::SetPrivateMode(mode).into(),
                        );
                    },
                    None => unexpected(raw_params, byte),
                }
            }
        },
        (b'h', params) => {
            for param in params {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = Mode::from_raw(mode as u16);
                        actor.handle(
                            TerminalControlAction::SetMode(mode).into(),
                        );
                    },
                    None => unexpected(raw_params, byte),
                }
            }
        },
        (b'l', [CsiParam::P(b'?'), rest @ ..]) => {
            for param in rest {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = PrivateMode::from_raw(mode as u16);
                        if mode
                            == PrivateMode::Named(NamedPrivateMode::SyncUpdate)
                        {
                            actor.end_sync();
                        }

                        actor.handle(
                            TerminalControlAction::UnsetPrivateMode(mode)
                                .into(),
                        );
                    },
                    None => unexpected(raw_params, byte),
                }
            }
        },
        (b'l', params) => {
            for param in params {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = Mode::from_raw(mode as u16);
                        actor.handle(
                            TerminalControlAction::UnsetMode(mode).into(),
                        );
                    },
                    None => unexpected(raw_params, byte),
                }
            }
        },
        (b'm', [CsiParam::P(b'?'), CsiParam::Integer(4), ..]) => {
            actor.handle(
                TerminalControlAction::ReportModifyOtherKeysState.into(),
            );
        },
        (b'm', [CsiParam::P(b'>'), CsiParam::Integer(4), rest @ ..]) => {
            let mode = match rest[0] {
                CsiParam::Integer(0) => ModifyOtherKeysState::Reset,
                CsiParam::Integer(1) => {
                    ModifyOtherKeysState::EnableExceptWellDefined
                },
                CsiParam::Integer(2) => ModifyOtherKeysState::EnableAll,
                _ => return unexpected(raw_params, byte),
            };

            actor.handle(
                TerminalControlAction::SetModifyOtherKeysState(mode).into(),
            );
        },
        (b'm', params) => {
            if params.is_empty() {
                actor.handle(Action::SetCharacterAttribute(
                    CharacterAttribute::Reset,
                ));
            } else {
                attrs_from_sgr_parameters(actor, params);
            }
        },
        (b'p', [CsiParam::P(b'!')]) => actor.end_sync(),
        (b'p', [CsiParam::P(b'$')]) => actor.handle(
            TerminalControlAction::ReportMode(Mode::from_raw(0)).into(),
        ),
        (b'p', [CsiParam::P(b'$'), CsiParam::Integer(mode)]) => {
            actor.handle(
                TerminalControlAction::ReportMode(Mode::from_raw(*mode as u16))
                    .into(),
            );
        },
        (b'p', [CsiParam::P(b'?'), CsiParam::P(b'$')]) => {
            actor.handle(
                TerminalControlAction::ReportPrivateMode(
                    PrivateMode::from_raw(0),
                )
                .into(),
            );
        },
        (
            b'p',
            [
                CsiParam::P(b'?'),
                CsiParam::P(b'$'),
                CsiParam::Integer(mode),
            ],
        ) => actor.handle(
            TerminalControlAction::ReportPrivateMode(PrivateMode::from_raw(
                *mode as u16,
            ))
            .into(),
        ),
        (b'q', []) => {
            let style = parse_cursor_style(0);
            actor.handle(Action::SetCursorStyle(style));
        },
        (b'q', [CsiParam::Integer(shape)]) => {
            let style = parse_cursor_style(*shape);
            actor.handle(Action::SetCursorStyle(style));
        },
        (b'q', [CsiParam::Integer(shape), ..]) => {
            let style = parse_cursor_style(*shape);
            actor.handle(Action::SetCursorStyle(style));
        },
        (b'u', []) => actor.handle(Action::RestoreCursorPosition),
        (b'u', [CsiParam::P(b'?'), ..]) => {
            actor.handle(TerminalControlAction::ReportKeyboardMode.into())
        },
        (
            b'u',
            [
                CsiParam::P(b'='),
                CsiParam::Integer(raw_mode),
                CsiParam::P(b';'),
                CsiParam::Integer(raw_behavior),
                ..,
            ],
        ) => {
            let mode = KeyboardMode::from_bits_truncate(*raw_mode as u8);
            let behavior = match raw_behavior {
                3 => KeyboardModeApplyBehavior::Difference,
                2 => KeyboardModeApplyBehavior::Union,
                // Default is replace.
                _ => KeyboardModeApplyBehavior::Replace,
            };

            actor.handle(
                TerminalControlAction::SetKeyboardMode(mode, behavior).into(),
            );
        },
        (b'u', [CsiParam::P(b'>'), CsiParam::Integer(mode), ..]) => {
            let mode = KeyboardMode::from_bits_truncate(*mode as u8);
            actor.handle(TerminalControlAction::PushKeyboardMode(mode).into());
        },
        (b'u', [CsiParam::P(b'<'), CsiParam::Integer(count), ..]) => {
            let count = if *count > 1 { *count } else { 1 } as u16;
            actor.handle(TerminalControlAction::PopKeyboardModes(count).into())
        },
        (b'W', [CsiParam::P(b'?'), CsiParam::Integer(5)]) => {
            actor.handle(Action::SetTabs(8));
        },
        (b'@', [CsiParam::Integer(count)]) => {
            actor.handle(Action::InsertBlank(*count as usize))
        },
        (b'A', []) => actor.handle(Action::MoveUp {
            rows: 1,
            carrage_return_needed: false,
        }),
        (b'A', [CsiParam::Integer(rows)]) => actor.handle(Action::MoveUp {
            rows: *rows as usize,
            carrage_return_needed: false,
        }),
        (b'B', []) => actor.handle(Action::MoveDown {
            rows: 1,
            carrage_return_needed: false,
        }),
        (b'B', [CsiParam::Integer(rows)]) => actor.handle(Action::MoveDown {
            rows: *rows as usize,
            carrage_return_needed: false,
        }),
        (b'e', [CsiParam::Integer(rows)]) => actor.handle(Action::MoveDown {
            rows: *rows as usize,
            carrage_return_needed: false,
        }),
        (b'b', [CsiParam::Integer(count)]) => {
            if let Some(c) = state.last_preceding_char {
                for _ in 0..*count {
                    actor.handle(Action::Print(c));
                }
            } else {
                debug!("tried to repeat with no preceding char");
            }
        },
        (b'C', []) => actor.handle(Action::MoveForward(1)),
        (b'C', [CsiParam::Integer(columns)]) => {
            actor.handle(Action::MoveForward(*columns as usize))
        },
        (b'a', []) => actor.handle(Action::MoveForward(1)),
        (b'a', [CsiParam::Integer(columns)]) => {
            actor.handle(Action::MoveForward(*columns as usize))
        },
        (b'c', []) => {
            actor.handle(TerminalControlAction::IdentifyTerminal(None).into())
        },
        (b'c', [CsiParam::P(b'>')]) => actor
            .handle(TerminalControlAction::IdentifyTerminal(Some('>')).into()),
        (b'c', [CsiParam::Integer(attr)]) => actor.handle(
            TerminalControlAction::IdentifyTerminal(char::from_u32(
                *attr as u32,
            ))
            .into(),
        ),
        (b'D', []) => actor.handle(Action::MoveBackward(1)),
        (b'D', [CsiParam::Integer(columns)]) => {
            actor.handle(Action::MoveBackward(*columns as usize))
        },
        (b'd', []) => actor.handle(Action::GotoRow(1)),
        (b'd', [CsiParam::Integer(line_num)]) => {
            actor.handle(Action::GotoRow(*line_num as i32 - 1))
        },
        (b'E', []) => actor.handle(Action::MoveDown {
            rows: 1,
            carrage_return_needed: true,
        }),
        (b'E', [CsiParam::Integer(line_count)]) => {
            actor.handle(Action::MoveDown {
                rows: *line_count as usize,
                carrage_return_needed: true,
            })
        },
        (b'F', []) => actor.handle(Action::MoveUp {
            rows: 1,
            carrage_return_needed: true,
        }),
        (b'F', [CsiParam::Integer(line_count)]) => {
            actor.handle(Action::MoveUp {
                rows: *line_count as usize,
                carrage_return_needed: true,
            })
        },
        (b'G', []) => actor.handle(Action::GotoColumn(1)),
        (b'G', [CsiParam::Integer(column_num)]) => {
            actor.handle(Action::GotoColumn(*column_num as usize - 1))
        },
        (b'`', []) => actor.handle(Action::GotoColumn(1)),
        (b'`', [CsiParam::Integer(column_num)]) => {
            actor.handle(Action::GotoColumn(*column_num as usize - 1))
        },
        (b'g', []) => {
            actor.handle(Action::ClearTabs(TabClearMode::Current));
        },
        (b'g', [CsiParam::Integer(mode)]) => {
            let mode = match mode {
                0 => TabClearMode::Current,
                3 => TabClearMode::All,
                _ => {
                    return unexpected(raw_params, byte);
                },
            };

            actor.handle(Action::ClearTabs(mode));
        },
        (b'H', []) => actor.handle(Action::Goto(0, 0)),
        (
            b'H',
            [
                CsiParam::Integer(y),
                CsiParam::P(b';'),
                CsiParam::Integer(x),
            ],
        ) => actor.handle(Action::Goto(*y as i32 - 1, *x as usize - 1)),
        (
            b'f',
            [
                CsiParam::Integer(y),
                CsiParam::P(b';'),
                CsiParam::Integer(x),
            ],
        ) => actor.handle(Action::Goto(*y as i32 - 1, *x as usize - 1)),
        (b'I', []) => actor.handle(Action::MoveForwardTabs(1)),
        (b'I', [CsiParam::Integer(count)]) => {
            actor.handle(Action::MoveForwardTabs(*count as u16))
        },
        (b'J', []) => actor.handle(Action::ClearScreen(ClearMode::Below)),
        (b'J', [CsiParam::Integer(mode)]) => {
            let mode = match mode {
                0 => ClearMode::Below,
                1 => ClearMode::Above,
                2 => ClearMode::All,
                3 => ClearMode::Saved,
                _ => {
                    return unexpected(raw_params, byte);
                },
            };

            actor.handle(Action::ClearScreen(mode));
        },
        (b'K', []) => actor.handle(Action::ClearLine(LineClearMode::Right)),
        (b'K', [CsiParam::Integer(mode)]) => {
            let mode = match mode {
                0 => LineClearMode::Right,
                1 => LineClearMode::Left,
                2 => LineClearMode::All,
                _ => {
                    return unexpected(raw_params, byte);
                },
            };

            actor.handle(Action::ClearLine(mode));
        },
        (b'L', []) => actor.handle(Action::InsertBlankLines(1)),
        (b'L', [CsiParam::Integer(count)]) => {
            actor.handle(Action::InsertBlankLines(*count as usize))
        },
        (b'M', []) => actor.handle(Action::DeleteLines(1)),
        (b'M', [CsiParam::Integer(count)]) => {
            actor.handle(Action::DeleteLines(*count as usize))
        },
        (b'n', []) => {
            actor.handle(TerminalControlAction::ReportDeviceStatus(0).into())
        },
        (b'n', [CsiParam::Integer(report)]) => actor.handle(
            TerminalControlAction::ReportDeviceStatus(*report as usize).into(),
        ),
        (b'P', []) => actor.handle(Action::DeleteChars(1)),
        (b'P', [CsiParam::Integer(count)]) => {
            actor.handle(Action::DeleteChars(*count as usize))
        },

        (
            b'r',
            [
                CsiParam::Integer(top),
                CsiParam::P(b';'),
                CsiParam::Integer(bottom),
            ],
        ) => actor.handle(Action::SetScrollingRegion(
            *top as usize,
            *bottom as usize,
        )),
        (b'S', []) => actor.handle(Action::ScrollUp(1)),
        (b'S', [CsiParam::Integer(count)]) => {
            actor.handle(Action::ScrollUp(*count as usize))
        },
        (b's', [..]) => actor.handle(Action::SaveCursorPosition),
        (b'T', []) => actor.handle(Action::ScrollDown(1)),
        (b'T', [CsiParam::Integer(count)]) => {
            actor.handle(Action::ScrollDown(*count as usize))
        },
        (b't', [CsiParam::Integer(id)]) => match *id {
            14 => actor.handle(
                TerminalControlAction::RequestTextAreaSizeByPixels.into(),
            ),
            18 => actor.handle(
                TerminalControlAction::RequestTextAreaSizeByChars.into(),
            ),
            22 => actor.handle(TerminalControlAction::PushWindowTitle.into()),
            23 => actor.handle(TerminalControlAction::PopWindowTitle.into()),
            _ => unexpected(raw_params, byte),
        },
        (b'X', []) => actor.handle(Action::EraseChars(1)),
        (b'X', [CsiParam::Integer(count)]) => {
            actor.handle(Action::EraseChars(*count as usize))
        },
        (b'Z', []) => actor.handle(Action::MoveBackwardTabs(1)),
        (b'Z', [CsiParam::Integer(count)]) => {
            actor.handle(Action::MoveBackwardTabs(*count as u16))
        },
        _ => unexpected(raw_params, byte),
    };
}

#[inline]
fn attrs_from_sgr_parameters<A: Actor>(actor: &mut A, params: &[CsiParam]) {
    let mut iter = params.into_iter().peekable();

    while let Some(param) = iter.next() {
        let attr = match param {
            CsiParam::Integer(0) => Some(CharacterAttribute::Reset),
            CsiParam::Integer(1) => Some(CharacterAttribute::Bold),
            CsiParam::Integer(2) => Some(CharacterAttribute::Dim),
            CsiParam::Integer(3) => Some(CharacterAttribute::Italic),
            CsiParam::Integer(4) => match iter.peek().copied() {
                Some(CsiParam::Integer(0)) => {
                    iter.next();
                    Some(CharacterAttribute::CancelUnderline)
                },
                Some(CsiParam::Integer(2)) => {
                    iter.next();
                    Some(CharacterAttribute::DoubleUnderline)
                },
                Some(CsiParam::Integer(3)) => {
                    iter.next();
                    Some(CharacterAttribute::Undercurl)
                },
                Some(CsiParam::Integer(4)) => {
                    iter.next();
                    Some(CharacterAttribute::DottedUnderline)
                },
                Some(CsiParam::Integer(5)) => {
                    iter.next();
                    Some(CharacterAttribute::DashedUnderline)
                },
                _ => Some(CharacterAttribute::Underline),
            },
            CsiParam::Integer(5) => Some(CharacterAttribute::BlinkSlow),
            CsiParam::Integer(6) => Some(CharacterAttribute::BlinkFast),
            CsiParam::Integer(7) => Some(CharacterAttribute::Reverse),
            CsiParam::Integer(8) => Some(CharacterAttribute::Hidden),
            CsiParam::Integer(9) => Some(CharacterAttribute::Strike),
            CsiParam::Integer(21) => Some(CharacterAttribute::CancelBold),
            CsiParam::Integer(22) => Some(CharacterAttribute::CancelBoldDim),
            CsiParam::Integer(23) => Some(CharacterAttribute::CancelItalic),
            CsiParam::Integer(24) => Some(CharacterAttribute::CancelUnderline),
            CsiParam::Integer(25) => Some(CharacterAttribute::CancelBlink),
            CsiParam::Integer(27) => Some(CharacterAttribute::CancelReverse),
            CsiParam::Integer(28) => Some(CharacterAttribute::CancelHidden),
            CsiParam::Integer(29) => Some(CharacterAttribute::CancelStrike),
            CsiParam::Integer(30) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::Black),
            )),
            CsiParam::Integer(31) => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Red)))
            },
            CsiParam::Integer(32) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::Green),
            )),
            CsiParam::Integer(33) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::Yellow),
            )),
            CsiParam::Integer(34) => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Blue)))
            },
            CsiParam::Integer(35) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::Magenta),
            )),
            CsiParam::Integer(36) => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Cyan)))
            },
            CsiParam::Integer(37) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::White),
            )),
            CsiParam::Integer(38) => {
                parse_sgr_color(&mut iter).map(CharacterAttribute::Foreground)
            },
            CsiParam::Integer(39) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::Foreground),
            )),
            CsiParam::Integer(40) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::Black),
            )),
            CsiParam::Integer(41) => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Red)))
            },
            CsiParam::Integer(42) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::Green),
            )),
            CsiParam::Integer(43) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::Yellow),
            )),
            CsiParam::Integer(44) => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Blue)))
            },
            CsiParam::Integer(45) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::Magenta),
            )),
            CsiParam::Integer(46) => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Cyan)))
            },
            CsiParam::Integer(47) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::White),
            )),
            CsiParam::Integer(48) => {
                parse_sgr_color(&mut iter).map(CharacterAttribute::Background)
            },
            CsiParam::Integer(49) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::Background),
            )),
            CsiParam::Integer(58) => parse_sgr_color(&mut iter)
                .map(|color| CharacterAttribute::UnderlineColor(Some(color))),
            CsiParam::Integer(59) => {
                Some(CharacterAttribute::UnderlineColor(None))
            },
            CsiParam::Integer(90) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightBlack),
            )),
            CsiParam::Integer(91) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightRed),
            )),
            CsiParam::Integer(92) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightGreen),
            )),
            CsiParam::Integer(93) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightYellow),
            )),
            CsiParam::Integer(94) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightBlue),
            )),
            CsiParam::Integer(95) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightMagenta),
            )),
            CsiParam::Integer(96) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightCyan),
            )),
            CsiParam::Integer(97) => Some(CharacterAttribute::Foreground(
                Color::Std(StdColor::BrightWhite),
            )),
            CsiParam::Integer(100) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightBlack),
            )),
            CsiParam::Integer(101) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightRed),
            )),
            CsiParam::Integer(102) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightGreen),
            )),
            CsiParam::Integer(103) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightYellow),
            )),
            CsiParam::Integer(104) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightBlue),
            )),
            CsiParam::Integer(105) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightMagenta),
            )),
            CsiParam::Integer(106) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightCyan),
            )),
            CsiParam::Integer(107) => Some(CharacterAttribute::Background(
                Color::Std(StdColor::BrightWhite),
            )),
            _ => None,
        };

        if let Some(attr) = attr {
            actor.handle(Action::SetCharacterAttribute(attr));
        }
    }
}

fn parse_cursor_style(raw_shape: i64) -> Option<CursorStyle> {
    let shape = match raw_shape {
        0 | 1 | 2 => Some(CursorShape::Block),
        3 | 4 => Some(CursorShape::Underline),
        5 | 6 => Some(CursorShape::Beam),
        _ => None,
    };

    shape.map(|shape| CursorStyle {
        shape,
        blinking: raw_shape % 2 == 1,
    })
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
    }

    values
}

fn unexpected(params: &[CsiParam], byte: u8) {
    debug!("[unexpected csi] action: {byte:?}, params: {params:?}",);
}
