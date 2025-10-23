use std::collections::btree_map::Values;

use log::debug;
use otty_vte::CsiParam;

use crate::attributes::Attr;
use crate::color::{Color, Rgb, StdColor};
use crate::cursor::{CursorShape, CursorStyle};
use crate::mode::{
    ClearMode, KeyboardModes, KeyboardModesApplyBehavior, LineClearMode, Mode,
    ModifyOtherKeys, PrivateMode, ScpCharPath, ScpUpdateMode, TabClearMode,
};
use crate::parser::{ParseState, SYNC_UPDATE_TIMEOUT};
use crate::timeout::Timeout;
use crate::{Actor, NamedPrivateMode};

/// Operating system command with raw arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
enum CSI {
    /// ICH
    InsertBlank,
    /// CUU
    CursorUp,
    /// CUD
    CursorDown,
    /// VPR
    VerticalPositionRelative,
    /// REP
    RepeatPrecedingCharacter,
    /// DA1
    PrimaryDeviceAttributes,
    /// CUF
    CursorForward,
    /// HPR
    HorizontalPositionRelative,
    /// HPA
    HorizontalPositionAbsolute,
    /// CUB
    CursorBackward,
    /// VPA
    VerticalPositionAbsolute,
    /// CNL
    CursorNextLine,
    /// CPL
    CursorPrecedingLine,
    /// CHA
    CursorHorizontalAbsolute,
    /// DECST8C
    SetTabStops,
    /// TBC
    TabClear,
    /// CUP
    CursorPosition,
    /// HVP
    HorizontalAndVerticalPosition,
    /// SM
    SetMode(Vec<Mode>),
    /// DECSET
    SetModePrivate(Vec<PrivateMode>),
    /// CHT
    CursorHorizontalTabulation,
    /// ED
    EraseDisplay,
    /// EL
    EraseLine,
    /// DECSCA
    SelectCharacterProtectionAttribute,
    /// IL
    InsertLine,
    /// RM
    ResetMode(Vec<Mode>),
    /// DECRST
    ResetModePrivate(Vec<PrivateMode>),
    /// DL
    DeleteLine,
    /// SGR sequences
    SelectGraphicRendition(Vec<u16>),
    SetModifyOtherKeys(Vec<u16>),
    ReportModifyOtherKeys(Vec<u16>),
    /// DSR
    DeviceStatusReport,
    /// DCH
    DeleteCharacter,
    /// DECRQM
    RequestMode,
    RequestModePrivate,
    /// DECSCUSR
    SetCursorStyle,
    /// DECSTBM
    SetTopAndBottomMargin,
    /// SU
    ScrollUp,
    /// SCOSC
    SaveCursor,
    /// SD
    ScrollDown,
    /// Window manipulation sequences
    WindowManipulation,
    /// SCORC
    RestoreCursorPosition,
    ReportKeyboardMode,
    SetKeyboardMode(i64, i64),
    PushKeyboardMode(i64),
    PopKeyboardModes(i64),
    /// ECH Erase Character
    EraseCharacters,
    /// CBT Cursor Backward Tabulation
    CursorBackwardTabulation,
    /// Misc sequences
    Unspecified {
        params: Vec<CsiParam>,
        final_byte: u8,
    },
}

impl From<(&[CsiParam], &[u8], u8)> for CSI {
    fn from(value: (&[CsiParam], &[u8], u8)) -> Self {
        let (params, intermediates, final_byte) = value;
        let first = params.first();

        match (final_byte, intermediates) {
            (b'h', []) => {
                let (prefix, params) = parse_mode_params(params);
                let modes =
                    params.into_iter().map(Mode::from_raw).collect();
                Self::SetMode(modes)
            },
            (b'h', [b'?']) => {
                let (prefix, params) = parse_mode_params(params);

                let modes =
                    params.into_iter().map(PrivateMode::from_raw).collect();
                Self::SetModePrivate(modes)
            }
            (b'l', []) => {
                let (prefix, params) = parse_mode_params(params);
                let modes =
                    params.into_iter().map(Mode::from_raw).collect();
                Self::ResetMode(modes)
            },
            (b'l', [b'?']) => {
                let (prefix, params) = parse_mode_params(params);
                let modes =
                    params.into_iter().map(PrivateMode::from_raw).collect();
                Self::ResetModePrivate(modes)
            }
            (b'm', []) => {
                let (prefix, values) = parse_mode_params(params);
                Self::SelectGraphicRendition(values)
            },
            (b'm', [b'?']) => {
                let (prefix, values) = parse_mode_params(params);
                Self::ReportModifyOtherKeys(values)
            },
            (b'm', [b'>']) => {
                let (prefix, values) = parse_mode_params(params);
                Self::SetModifyOtherKeys(values)
            },
            (b'p', [b'$']) => {
                Self::RequestMode
            },
            (b'p', [b'?', b'$']) => {
                Self::RequestModePrivate
            }
            // b'p' => {
            //     if params
            //         .iter()
            //         .any(|param| matches!(param, CsiParam::P(b'$')))
            //     {
            //         if matches!(first, Some(CsiParam::P(b'?'))) {
            //             Self::RequestModePrivate
            //         } else {
            //             Self::RequestMode
            //         }
            //     } else {
            //         Self::Unspecified {
            //             params: params.to_vec(),
            //             final_byte,
            //         }
            //     }
            // },
            (b'q', []) => {
                Self::SetCursorStyle
                // if params
                //     .iter()
                //     .any(|param| matches!(param, CsiParam::P(b'"')))
                // {
                //     Self::SelectCharacterProtectionAttribute
                // } else if params
                //     .iter()
                //     .any(|param| matches!(param, CsiParam::P(b' ')))
                // {
                //     Self::SetCursorStyle
                // } else {
                //     Self::Unspecified {
                //         params: params.to_vec(),
                //         final_byte,
                //     }
                // }
            },
            (b'u', []) => Self::RestoreCursorPosition,
            (b'u', [b'?']) => Self::ReportKeyboardMode,
            (b'u', [b'=']) => {
                if let (
                    Some(CsiParam::Integer(flags)),
                    Some(CsiParam::P(b';')),
                    Some(CsiParam::Integer(mode)),
                ) =
                    (params.get(0), params.get(1), params.get(2))
                {
                    Self::SetKeyboardMode(*flags, *mode)
                } else {
                    Self::Unspecified { params: params.to_vec(), final_byte }
                }
            },
            (b'u', [b'>']) => {
                if let
                    Some(CsiParam::Integer(flags)) = params.get(0) {
                    Self::PushKeyboardMode(*flags)
                } else {
                    Self::Unspecified { params: params.to_vec(), final_byte }
                }
            }
            (b'u', [b'<']) => {
                if matches!(first, Some(CsiParam::P(b'<'))) {
                    let count = params
                        .get(1)
                        .and_then(|param| match param {
                            CsiParam::Integer(value) => Some(*value),
                            _ => None,
                        })
                        .unwrap_or(1);
                    Self::PopKeyboardModes(count)
                } else {
                    Self::Unspecified { params: params.to_vec(), final_byte }
                }
                // if matches!(first, Some(CsiParam::P(b'?'))) {
                //     Self::ReportKeyboardMode
                // } else if let (
                //     Some(CsiParam::P(b'=')),
                //     Some(CsiParam::Integer(flags)),
                //     Some(CsiParam::P(b';')),
                //     Some(CsiParam::Integer(mode)),
                // ) =
                //     (params.get(0), params.get(1), params.get(2), params.get(3))
                // {
                //     Self::SetKeyboardMode(*flags, *mode)
                // } else if let (
                //     Some(CsiParam::P(b'>')),
                //     Some(CsiParam::Integer(flags)),
                // ) = (params.get(0), params.get(1))
                // {
                //     Self::PushKeyboardMode(*flags)
                // } else if matches!(first, Some(CsiParam::P(b'<'))) {
                //     let count = params
                //         .get(1)
                //         .and_then(|param| match param {
                //             CsiParam::Integer(value) => Some(*value),
                //             _ => None,
                //         })
                //         .unwrap_or(1);
                //     Self::PopKeyboardModes(count)
                // } else {
                //     Self::RestoreCursorPosition
                // }
            },
            (b'W', [b'?']) => Self::SetTabStops,
            (b'k', [b' ']) => Self::SelectCharacterProtectionAttribute,
            (b'@', []) => Self::InsertBlank,
            (b'A', []) => Self::CursorUp,
            (b'B', []) => Self::CursorDown,
            (b'e', []) => Self::VerticalPositionRelative,
            (b'b', []) => Self::RepeatPrecedingCharacter,
            (b'C', []) => Self::CursorForward,
            (b'a', []) => Self::HorizontalPositionRelative,
            (b'c', _) => Self::PrimaryDeviceAttributes,
            (b'D', []) => Self::CursorBackward,
            (b'd', []) => Self::VerticalPositionAbsolute,
            (b'E', []) => Self::CursorNextLine,
            (b'F', []) => Self::CursorPrecedingLine,
            (b'G', []) => Self::CursorHorizontalAbsolute,
            (b'`', []) => Self::HorizontalPositionAbsolute,
            (b'g', []) => Self::TabClear,
            (b'H', []) => Self::CursorPosition,
            (b'f', []) => Self::HorizontalAndVerticalPosition,
            (b'I', []) => Self::CursorHorizontalTabulation,
            (b'J', []) => Self::EraseDisplay,
            (b'K', []) => Self::EraseLine,
            (b'L', []) => Self::InsertLine,
            (b'M', []) => Self::DeleteLine,
            (b'n', []) => Self::DeviceStatusReport,
            (b'P', []) => Self::DeleteCharacter,
            (b'r', []) => Self::SetTopAndBottomMargin,
            (b'S', []) => Self::ScrollUp,
            (b's', []) => Self::SaveCursor,
            (b'T', []) => Self::ScrollDown,
            (b't', []) => Self::WindowManipulation,
            (b'X', []) => Self::EraseCharacters,
            (b'Z', []) => Self::CursorBackwardTabulation,
            _ => Self::Unspecified {
                params: params.to_vec(),
                final_byte,
            },
        }
    }
}

pub(crate) fn perform<A: Actor, T: Timeout>(
    actor: &mut A,
    state: &mut ParseState<T>,
    params: &[CsiParam],
    intermediates: &[u8],
    params_truncated: bool,
    byte: u8,
) {
    if params_truncated || intermediates.len() > 2 {
        return unexpected(params, byte);
    }

    let mut params_iter = params.iter();

    match CSI::from((params, intermediates, byte)) {
        CSI::InsertBlank => {
            actor.insert_blank(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::CursorUp => {
            actor.move_up(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::CursorDown | CSI::VerticalPositionRelative => {
            actor.move_down(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::RepeatPrecedingCharacter => {
            repeat_preceding_char(actor, state, &mut params_iter)
        },
        CSI::CursorForward | CSI::HorizontalPositionRelative => {
            actor.move_forward(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::PrimaryDeviceAttributes
            if next_param_or(0, &mut params_iter) == 0 =>
        {
            actor.identify_terminal(intermediates.first().map(|&i| i as char))
        },
        CSI::CursorBackward => {
            actor.move_backward(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::VerticalPositionAbsolute => {
            actor.goto_line(next_param_or(1, &mut params_iter) as i32 - 1)
        },
        CSI::CursorNextLine => {
            actor.move_down_and_cr(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::CursorPrecedingLine => {
            actor.move_up_and_cr(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::CursorHorizontalAbsolute | CSI::HorizontalPositionAbsolute => {
            actor.goto_col(next_param_or(1, &mut params_iter) as usize - 1)
        },
        CSI::SetTabStops if next_param_or(0, &mut params_iter) == 5 => {
            actor.set_tabs(8)
        },
        CSI::TabClear => {
            let mode = match next_param_or(0, &mut params_iter) {
                0 => TabClearMode::Current,
                3 => TabClearMode::All,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.clear_tabs(mode);
        },
        CSI::CursorPosition | CSI::HorizontalAndVerticalPosition => {
            let y = next_param_or(1, &mut params_iter) as i32;
            let x = next_param_or(1, &mut params_iter) as usize;
            actor.goto(y - 1, x - 1);
        },
        CSI::SetMode(modes) => {
            for mode in modes {
                actor.set_mode(mode);
            }
        },
        CSI::SetModePrivate(modes) => {
            for mode in modes {
                if mode == PrivateMode::Named(NamedPrivateMode::SyncUpdate) {
                    state.sync_state.timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
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
        CSI::CursorHorizontalTabulation => {
            actor.move_forward_tabs(next_param_or(1, &mut params_iter))
        },
        CSI::EraseDisplay => {
            let mode = match next_param_or(0, &mut params_iter) {
                0 => ClearMode::Below,
                1 => ClearMode::Above,
                2 => ClearMode::All,
                3 => ClearMode::Saved,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.clear_screen(mode);
        },
        CSI::EraseLine => {
            let mode = match next_param_or(0, &mut params_iter) {
                0 => LineClearMode::Right,
                1 => LineClearMode::Left,
                2 => LineClearMode::All,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.clear_line(mode);
        },
        CSI::SelectCharacterProtectionAttribute => {
            if intermediates != [b' '] {
                return unexpected(params, byte);
            }
            // SCP control.
            let char_path = match next_param_or(0, &mut params_iter) {
                0 => ScpCharPath::Default,
                1 => ScpCharPath::LTR,
                2 => ScpCharPath::RTL,
                _ => {
                    return unexpected(params, byte);
                },
            };

            let update_mode = match next_param_or(0, &mut params_iter) {
                0 => ScpUpdateMode::ImplementationDependant,
                1 => ScpUpdateMode::DataToPresentation,
                2 => ScpUpdateMode::PresentationToData,
                _ => {
                    return unexpected(params, byte);
                },
            };

            actor.set_scp(char_path, update_mode);
        },
        CSI::InsertLine => actor
            .insert_blank_lines(next_param_or(1, &mut params_iter) as usize),
        CSI::DeleteLine => {
            actor.delete_lines(next_param_or(1, &mut params_iter) as usize)
        },
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
        CSI::DeviceStatusReport => {
            actor.device_status(next_param_or(0, &mut params_iter) as usize)
        },
        CSI::DeleteCharacter => {
            actor.delete_chars(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::RequestMode => {
            let mode = next_param_or(0, &mut params_iter) ;
            actor.report_mode(Mode::from_raw(mode));
        },
        CSI::RequestModePrivate => {
            let mode = next_param_or(0, &mut params_iter) ;
            actor.report_private_mode(PrivateMode::from_raw(mode));
        },
        CSI::SetCursorStyle => {
            let cursor_style_id = next_param_or(0, &mut params_iter);
            let shape = match cursor_style_id {
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
                blinking: cursor_style_id % 2 == 1,
            });

            actor.set_cursor_style(cursor_style);
        },
        CSI::SetTopAndBottomMargin => {
            let top = next_param_or(1, &mut params_iter) as usize;
            let bottom = match params_iter.next() {
                Some(CsiParam::Integer(value)) if *value > 0 => {
                    Some(*value as usize)
                },
                _ => None,
            };

            actor.set_scrolling_region(top, bottom);
        },
        CSI::ScrollUp => {
            actor.scroll_up(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::SaveCursor => actor.save_cursor_position(),
        CSI::ScrollDown => {
            actor.scroll_down(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::WindowManipulation => {
            match next_param_or(1, &mut params_iter) as usize {
                14 => actor.text_area_size_pixels(),
                18 => actor.text_area_size_chars(),
                22 => actor.push_title(),
                23 => actor.pop_title(),
                _ => unexpected(params, byte),
            }
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
        CSI::EraseCharacters => {
            actor.erase_chars(next_param_or(1, &mut params_iter) as usize)
        },
        CSI::CursorBackwardTabulation => {
            actor.move_backward_tabs(next_param_or(1, &mut params_iter) as u16)
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

fn repeat_preceding_char<'a, I, A, T>(
    actor: &mut A,
    state: &mut ParseState<T>,
    params_iter: &mut I,
) where
    T: Timeout,
    A: Actor,
    I: Iterator<Item = &'a CsiParam>,
{
    if let Some(c) = state.last_preceding_char {
        for _ in 0..next_param_or(1, params_iter) {
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

fn parse_mode_params(params: &[CsiParam]) -> (Option<u8>, Vec<u16>) {
    let mut prefix = None;
    let mut start_idx = 0usize;

    if let Some(CsiParam::P(byte)) = params.first() {
        if matches!(byte, b'?' | b'>' | b'=') {
            prefix = Some(*byte);
            start_idx = 1;
        }
    }

    let mut values = Vec::new();
    let mut pending: Option<u16> = None;
    // let mut last_separator = true;

    for param in params.iter().skip(start_idx) {
        match param {
            CsiParam::Integer(value) => {
                let parsed = if (0..=u16::MAX as i64).contains(value) {
                    *value as u16
                } else {
                    0
                };
                pending = Some(parsed);
                // last_separator = false;
            },
            CsiParam::P(b';') => {
                values.push(pending.take().unwrap_or(0));
                // last_separator = true;
            },
            CsiParam::P(_) => {
                // Ignore unexpected parameter designators for now.
                // last_separator = false;
            },
        }
    }

    if let Some(value) = pending {
        values.push(value);
    } else if values.is_empty() {
        values.push(0);
    }

    (prefix, values)
}

fn unexpected(params: &[CsiParam], byte: u8) {
    debug!("[unexpected csi] action: {byte:?}, params: {params:?}",);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::{Mode, NamedMode, NamedPrivateMode};
    use otty_vte::{Actor as VteActor, Parser as VteParser};

    #[derive(Default)]
    struct RecordingActor {
        csis: Vec<CSI>,
    }

    impl VteActor for RecordingActor {
        fn print(&mut self, _: char) {}

        fn execute(&mut self, _: u8) {}

        fn hook(&mut self, _: &[i64], _: &[u8], _: bool, _: u8) {}

        fn unhook(&mut self) {}

        fn put(&mut self, _: u8) {}

        fn osc_dispatch(&mut self, _: &[&[u8]], _: u8) {}

        fn csi_dispatch(
            &mut self,
            params: &[CsiParam],
            intermediates: &[u8],
            parameters_truncated: bool,
            byte: u8,
        ) {
            assert!(
                !parameters_truncated,
                "unexpected parameter truncation for params {params:?}",
            );
            assert!(
                intermediates.is_empty(),
                "unexpected intermediates delivered separately: {intermediates:?}"
            );
            self.csis.push(CSI::from((params, byte)));
        }

        fn esc_dispatch(&mut self, _: &[i64], _: &[u8], _: bool, _: u8) {}
    }

    impl RecordingActor {
        fn into_csis(self) -> Vec<CSI> {
            self.csis
        }
    }

    fn collect_csis(bytes: &[u8]) -> Vec<CSI> {
        let mut parser = VteParser::new();
        let mut actor = RecordingActor::default();
        parser.advance(bytes, &mut actor);
        actor.into_csis()
    }

    fn parse_single_csi(bytes: &[u8]) -> CSI {
        let mut csis = collect_csis(bytes);
        assert_eq!(
            csis.len(),
            1,
            "expected a single CSI action for sequence {bytes:?}"
        );
        csis.pop().unwrap()
    }

    #[track_caller]
    fn assert_csi(bytes: &[u8], expected: CSI) {
        let parsed = parse_single_csi(bytes);
        assert_eq!(parsed, expected, "sequence {bytes:?} parsed incorrectly");
    }

    #[test]
    fn parses_cursor_movement_sequences() {
        let cases: Vec<(&[u8], CSI)> = vec![
            (b"\x1b[4@", CSI::InsertBlank),
            (b"\x1b[3A", CSI::CursorUp),
            (b"\x1b[2B", CSI::CursorDown),
            (b"\x1b[5C", CSI::CursorForward),
            (b"\x1b[4D", CSI::CursorBackward),
            (b"\x1b[3a", CSI::HorizontalPositionRelative),
            (b"\x1b[12d", CSI::VerticalPositionAbsolute),
            (b"\x1b[2e", CSI::VerticalPositionRelative),
            (b"\x1b[5b", CSI::RepeatPrecedingCharacter),
            (b"\x1b[2E", CSI::CursorNextLine),
            (b"\x1b[2F", CSI::CursorPrecedingLine),
            (b"\x1b[9G", CSI::CursorHorizontalAbsolute),
            (b"\x1b[5\x60", CSI::HorizontalPositionAbsolute),
            (b"\x1b[7;9H", CSI::CursorPosition),
            (b"\x1b[7;9f", CSI::HorizontalAndVerticalPosition),
            (b"\x1b[3I", CSI::CursorHorizontalTabulation),
        ];

        for (bytes, expected) in cases {
            assert_csi(bytes, expected);
        }
    }

    #[test]
    fn parses_mode_management_sequences() {
        let cases: Vec<(&[u8], CSI)> = vec![
            (b"\x1b[4h", CSI::SetMode(vec![NamedMode::Insert.into()])),
            (
                b"\x1b[?1049h",
                CSI::SetModePrivate(vec![
                    NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
                ]),
            ),
            (b"\x1b[4l", CSI::ResetMode(vec![NamedMode::Insert.into()])),
            (
                b"\x1b[?1049l",
                CSI::ResetModePrivate(vec![
                    NamedPrivateMode::SwapScreenAndSetRestoreCursor.into(),
                ]),
            ),
            (b"\x1b[=1;2u", CSI::SetKeyboardMode(1, 2)),
            (b"\x1b[>5u", CSI::PushKeyboardMode(5)),
            (b"\x1b[<u", CSI::PopKeyboardModes(1)),
            (b"\x1b[<3u", CSI::PopKeyboardModes(3)),
            (b"\x1b[?u", CSI::ReportKeyboardMode),
            (
                b"\x1b[?25h",
                CSI::SetModePrivate(vec![NamedPrivateMode::ShowCursor.into()]),
            ),
            (
                b"\x1b[?25l",
                CSI::ResetModePrivate(vec![
                    NamedPrivateMode::ShowCursor.into(),
                ]),
            ),
            (b"\x1b[5h", CSI::SetMode(vec![Mode::Unknown(5)])),
            (b"\x1b[5l", CSI::ResetMode(vec![Mode::Unknown(5)])),
        ];

        for (bytes, expected) in cases {
            assert_csi(bytes, expected);
        }
    }

    #[test]
    fn parses_character_and_display_sequences() {
        let cases: Vec<(&[u8], CSI)> = vec![
            (b"\x1b[c", CSI::PrimaryDeviceAttributes),
            (b"\x1b[2J", CSI::EraseDisplay),
            (b"\x1b[2K", CSI::EraseLine),
            (b"\x1b[3L", CSI::InsertLine),
            (b"\x1b[2M", CSI::DeleteLine),
            (b"\x1b[2P", CSI::DeleteCharacter),
            (b"\x1b[31m", CSI::SelectGraphicRendition(vec![31])),
            (b"\x1b[>4;2m", CSI::SetModifyOtherKeys(vec![4, 2])),
            (b"\x1b[?1m", CSI::ReportModifyOtherKeys(vec![1])),
            (b"\x1b[69$p", CSI::RequestMode),
            (b"\x1b[?69$p", CSI::RequestModePrivate),
            (b"\x1b[1\x22q", CSI::SelectCharacterProtectionAttribute),
            (b"\x1b[2 q", CSI::SetCursorStyle),
            (b"\x1b[5W", CSI::SetTabStops),
            (b"\x1b[0g", CSI::TabClear),
            (b"\x1b[1;24r", CSI::SetTopAndBottomMargin),
            (b"\x1b[2S", CSI::ScrollUp),
            (b"\x1b[2T", CSI::ScrollDown),
            (b"\x1b[s", CSI::SaveCursor),
            (b"\x1b[u", CSI::RestoreCursorPosition),
            (b"\x1b[2;0;0t", CSI::WindowManipulation),
            (b"\x1b[5n", CSI::DeviceStatusReport),
        ];

        for (bytes, expected) in cases {
            assert_csi(bytes, expected);
        }
    }

    #[test]
    fn parses_fallback_to_unspecified() {
        let parsed = parse_single_csi(b"\x1b[2~");
        assert_eq!(
            parsed,
            CSI::Unspecified {
                params: vec![CsiParam::Integer(2)],
                final_byte: b'~',
            }
        );
    }
}
