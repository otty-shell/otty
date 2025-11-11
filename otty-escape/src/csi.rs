use log::debug;
use otty_vte::CsiParam;

use crate::attributes::CharacterAttribute;
use crate::color::{Color, StdColor, parse_sgr_color};
use crate::cursor::{CursorShape, CursorStyle};
use crate::keyboard::{
    KeyboardMode, KeyboardModeApplyBehavior, ModifyOtherKeysState,
};
use crate::mode::{ClearMode, LineClearMode, Mode, PrivateMode, TabClearMode};
use crate::parser::ParserState;
use crate::{Action, EscapeActor, NamedPrivateMode};

#[inline]
pub(crate) fn perform<A: EscapeActor>(
    actor: &mut A,
    state: &mut ParserState,
    raw_params: &[CsiParam],
    params_truncated: bool,
    byte: u8,
) {
    if params_truncated {
        return unexpected(raw_params, byte);
    }

    let fallback = || unexpected(raw_params, byte);

    use CsiParam::*;

    match (byte, raw_params) {
        // DECSET (CSI ? Pm h) and SM (CSI Pm h)
        (b'h', params) => handle_set_mode(actor, params, fallback),
        // DECRST (CSI ? Pm l) and RM (CSI Pm l)
        (b'l', params) => handle_reset_mode(actor, params, fallback),
        // xterm modifyOtherKeys state report.
        (b'm', [P(b'?'), Integer(4), ..]) => {
            actor.handle(Action::ReportModifyOtherKeysState)
        },
        // xterm modifyOtherKeys state set.
        (b'm', [P(b'>'), Integer(4), _, mode, ..]) => {
            handle_set_xterm_modify_other_keys_state(actor, mode, fallback)
        },
        // SGR (CSI Pm m) Reset
        (b'm', []) => actor.handle(Action::SGR(CharacterAttribute::Reset)),
        // SGR (CSI Pm m) Set
        (b'm', params) => handle_set_sgr_attribute(actor, params, fallback),
        // DECSTR (CSI ! p)
        (b'p', [P(b'!')]) => actor.end_sync(),
        (b'p', params) => handle_report_mode(actor, params, fallback),
        // DECRQM (CSI [ ? ] Ps $ p): [https://vt100.net/docs/vt510-rm/DECRQM.html]
        // DECSCUSR (CSI Ps SP q)
        (b'q', params) => handle_set_cursor_style(actor, params, fallback),
        // SCORC (CSI u)
        (b'u', []) => actor.handle(Action::RestoreCursorPosition),
        // Handle Kitty keyboard protocol: [https://sw.kovidgoyal.net/kitty/keyboard-protocol]
        (b'u', params) => handle_keyboard_mode(actor, params, fallback),
        // DECST8C (CSI ? 5 W)
        (b'W', [P(b'?'), Integer(5)]) => actor.handle(Action::SetTabs(8)),
        // ICH (CSI Ps @)
        (b'@', [Integer(count)]) => {
            actor.handle(Action::InsertBlank(*count as usize));
        },
        // CUU (CSI Ps A)
        (b'A', params) => handle_cursor_up(actor, params, fallback),
        // CUD (CSI Ps B)
        (b'B', params) => handle_cursor_down(actor, params, fallback),
        // VPR (CSI Ps e) which is an alias for CUD.
        (b'e', params) => handle_cursor_down(actor, params, fallback),
        // REP (CSI Ps b)
        (b'b', params) => {
            handle_repeat_preceding_character(actor, state, params, fallback)
        },
        // CUF (CSI Ps C)
        (b'C', params) => handle_cursor_forward(actor, params, fallback),
        // HPR (CSI Ps a)
        (b'a', params) => handle_cursor_forward(actor, params, fallback),
        // DA1/DA2 (CSI Ps c)
        (b'c', params) => handle_identify_terminal(actor, params, fallback),
        // CUB (CSI Ps D)
        (b'D', params) => handle_cursor_backward(actor, params, fallback),
        // VPA (CSI Ps d)
        (b'd', params) => {
            handle_vertical_position_absolute(actor, params, fallback)
        },
        // CNL (CSI Ps E)
        (b'E', params) => handle_cursor_next_line(actor, params, fallback),
        // CPL (CSI Ps F)
        (b'F', params) => handle_cursor_preceding_line(actor, params, fallback),
        // CHA (CSI Ps G)
        (b'G', params) => {
            handle_cursor_horizontal_absolute(actor, params, fallback)
        },
        // HPA (CSI Ps `)
        (b'`', params) => {
            handle_character_position_absolute(actor, params, fallback)
        },
        // TBC (CSI Ps g)
        (b'g', params) => handle_tab_clear(actor, params, fallback),
        // CUP (CSI Ps ; Ps H)
        (b'H', params) => {
            handle_horizontal_and_vertical_position(actor, params, fallback)
        },
        // HVP (CSI Ps ; Ps f)
        (b'f', params) => {
            handle_horizontal_and_vertical_position(actor, params, fallback)
        },
        // CHT (CSI Ps I)
        (b'I', params) => {
            handle_cursor_horizontal_tabulation(actor, params, fallback)
        },
        // ED (CSI Ps J)
        (b'J', params) => handle_erase_display(actor, params, fallback),
        // EL (CSI Ps K)
        (b'K', params) => handle_erase_line(actor, params, fallback),
        // IL (CSI Ps L)
        (b'L', params) => handle_insert_line(actor, params, fallback),
        // DL (CSI Ps M)
        (b'M', params) => handle_delete_line(actor, params, fallback),
        // DSR (CSI Ps n)
        (b'n', params) => handle_device_status_report(actor, params, fallback),
        // DCH (CSI Ps P)
        (b'P', params) => handle_delete_character(actor, params, fallback),
        // DECSTBM (CSI Ps ; Ps r)
        (b'r', params) => handle_set_scrolling_region(actor, params, fallback),
        // SU (CSI Ps S)
        (b'S', params) => handle_scroll_up(actor, params, fallback),
        // SCOSC (CSI s)
        (b's', ..) => actor.handle(Action::SaveCursorPosition),
        // SD (CSI Ps T)
        (b'T', params) => handle_scroll_down(actor, params, fallback),
        // Window manipulation (CSI Ps t) sequences.
        (b't', params) => handle_window_manipulation(actor, params, fallback),
        // ECH (CSI Ps X)
        (b'X', params) => handle_erase_characters(actor, params, fallback),
        // CBT (CSI Ps Z)
        (b'Z', params) => {
            handle_cursor_backward_tabulation(actor, params, fallback)
        },
        _ => fallback(),
    }
}

fn handle_set_mode<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [P(b'?'), rest @ ..] => {
            for param in rest {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = PrivateMode::from_raw(mode as u16);
                        if mode
                            == PrivateMode::Named(NamedPrivateMode::SyncUpdate)
                        {
                            actor.begin_sync();
                        }

                        actor.handle(Action::SetPrivateMode(mode));
                    },
                    None => fallback(),
                }
            }
        },
        params => {
            for param in params {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = Mode::from_raw(mode as u16);
                        actor.handle(Action::SetMode(mode));
                    },
                    None => fallback(),
                }
            }
        },
    }
}

fn handle_reset_mode<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [P(b'?'), rest @ ..] => {
            for param in rest {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = PrivateMode::from_raw(mode as u16);
                        if mode
                            == PrivateMode::Named(NamedPrivateMode::SyncUpdate)
                        {
                            actor.end_sync();
                        }

                        actor.handle(Action::UnsetPrivateMode(mode));
                    },
                    None => fallback(),
                }
            }
        },
        params => {
            for param in params {
                match param.as_integer() {
                    Some(mode) => {
                        let mode = Mode::from_raw(mode as u16);
                        actor.handle(Action::UnsetMode(mode));
                    },
                    None => fallback(),
                }
            }
        },
    }
}

fn handle_report_mode<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [P(b'$')] => actor.handle(Action::ReportMode(Mode::from_raw(0))),
        [Integer(mode), P(b'$')] => {
            actor.handle(Action::ReportMode(Mode::from_raw(*mode as u16)));
        },
        [P(b'?'), P(b'$')] => {
            actor.handle(Action::ReportPrivateMode(PrivateMode::from_raw(0)))
        },
        [P(b'?'), Integer(mode), P(b'$')] => actor.handle(
            Action::ReportPrivateMode(PrivateMode::from_raw(*mode as u16)),
        ),
        _ => fallback(),
    }
}

fn handle_set_xterm_modify_other_keys_state<A, F>(
    actor: &mut A,
    mode: &CsiParam,
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    let mode = match mode {
        Integer(0) => ModifyOtherKeysState::Reset,
        Integer(1) => ModifyOtherKeysState::EnableExceptWellDefined,
        Integer(2) => ModifyOtherKeysState::EnableAll,
        _ => return fallback(),
    };

    actor.handle(Action::SetModifyOtherKeysState(mode));
}

fn handle_set_cursor_style<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    let shape = match params {
        [] => 0,
        [Integer(shape)] => *shape,
        [Integer(shape), ..] => *shape,
        _ => return fallback(),
    };

    let style = parse_cursor_style(shape);
    actor.handle(Action::SetCursorStyle(style));
}

fn handle_keyboard_mode<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [P(b'?'), ..] => {
            actor.handle(Action::ReportKeyboardMode);
        },
        [
            P(b'='),
            Integer(raw_mode),
            P(b';'),
            Integer(raw_behavior),
            ..,
        ] => {
            let mode = KeyboardMode::from_bits_truncate(*raw_mode as u8);
            let behavior = match raw_behavior {
                3 => KeyboardModeApplyBehavior::Difference,
                2 => KeyboardModeApplyBehavior::Union,
                _ => KeyboardModeApplyBehavior::Replace,
            };

            actor.handle(Action::SetKeyboardMode(mode, behavior));
        },
        [P(b'>'), Integer(mode), ..] => {
            let mode = KeyboardMode::from_bits_truncate(*mode as u8);
            actor.handle(Action::PushKeyboardMode(mode));
        },
        [P(b'<'), Integer(count), ..] => {
            let count = if *count > 1 { *count } else { 1 } as u16;
            actor.handle(Action::PopKeyboardModes(count));
        },
        _ => fallback(),
    }
}

fn handle_cursor_up<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveUp {
            rows: 1,
            carrage_return_needed: false,
        }),
        [Integer(rows)] => actor.handle(Action::MoveUp {
            rows: *rows as usize,
            carrage_return_needed: false,
        }),
        _ => fallback(),
    }
}

fn handle_cursor_down<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveDown {
            rows: 1,
            carrage_return_needed: false,
        }),
        [Integer(rows)] => actor.handle(Action::MoveDown {
            rows: *rows as usize,
            carrage_return_needed: false,
        }),
        _ => fallback(),
    }
}

fn handle_repeat_preceding_character<A, F>(
    actor: &mut A,
    state: &mut ParserState,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    if let [Integer(count)] = params {
        if let Some(c) = state.last_preceding_char {
            for _ in 0..*count {
                actor.handle(Action::Print(c));
            }
        } else {
            debug!("tried to repeat with no preceding char");
        }
    } else {
        fallback()
    }
}

fn handle_cursor_forward<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveForward(1)),
        [Integer(columns)] => {
            actor.handle(Action::MoveForward(*columns as usize));
        },
        _ => fallback(),
    }
}

fn handle_identify_terminal<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::IdentifyTerminal(None)),
        [P(b'>')] => actor.handle(Action::IdentifyTerminal(Some('>'))),
        [Integer(attr)] => {
            actor.handle(Action::IdentifyTerminal(char::from_u32(*attr as u32)))
        },
        _ => fallback(),
    }
}

fn handle_cursor_backward<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveBackward(1)),
        [Integer(columns)] => {
            actor.handle(Action::MoveBackward(*columns as usize));
        },
        _ => fallback(),
    }
}

fn handle_vertical_position_absolute<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::GotoRow(0)),
        [Integer(line_num)] => {
            actor.handle(Action::GotoRow(*line_num as i32));
        },
        _ => fallback(),
    }
}

fn handle_cursor_next_line<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveDown {
            rows: 1,
            carrage_return_needed: true,
        }),
        [Integer(line_count)] => {
            actor.handle(Action::MoveDown {
                rows: *line_count as usize,
                carrage_return_needed: true,
            });
        },
        _ => fallback(),
    }
}

fn handle_cursor_preceding_line<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveUp {
            rows: 1,
            carrage_return_needed: true,
        }),
        [Integer(line_count)] => {
            actor.handle(Action::MoveUp {
                rows: *line_count as usize,
                carrage_return_needed: true,
            });
        },
        _ => fallback(),
    }
}

fn handle_cursor_horizontal_absolute<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::GotoColumn(0)),
        [Integer(column_num)] => {
            actor.handle(Action::GotoColumn(*column_num as usize));
        },
        _ => fallback(),
    }
}

fn handle_character_position_absolute<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::GotoColumn(0)),
        [Integer(column_num)] => {
            actor.handle(Action::GotoColumn(*column_num as usize));
        },
        _ => fallback(),
    }
}

fn handle_tab_clear<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::ClearTabs(TabClearMode::Current)),
        [Integer(mode)] => match mode {
            0 => actor.handle(Action::ClearTabs(TabClearMode::Current)),
            3 => actor.handle(Action::ClearTabs(TabClearMode::All)),
            _ => fallback(),
        },
        _ => fallback(),
    }
}

fn handle_horizontal_and_vertical_position<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::Goto(0, 0)),
        [Integer(y), P(b';'), Integer(x)] => {
            actor.handle(Action::Goto(*y as i32, *x as usize));
        },
        _ => fallback(),
    }
}

fn handle_cursor_horizontal_tabulation<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveForwardTabs(1)),
        [Integer(count)] => {
            actor.handle(Action::MoveForwardTabs(*count as u16));
        },
        _ => fallback(),
    }
}

fn handle_erase_display<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::ClearScreen(ClearMode::Below)),
        [Integer(mode)] => match mode {
            0 => actor.handle(Action::ClearScreen(ClearMode::Below)),
            1 => actor.handle(Action::ClearScreen(ClearMode::Above)),
            2 => actor.handle(Action::ClearScreen(ClearMode::All)),
            3 => actor.handle(Action::ClearScreen(ClearMode::Saved)),
            _ => fallback(),
        },
        _ => fallback(),
    }
}

fn handle_erase_line<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::ClearLine(LineClearMode::Right)),
        [Integer(mode)] => match mode {
            0 => actor.handle(Action::ClearLine(LineClearMode::Right)),
            1 => actor.handle(Action::ClearLine(LineClearMode::Left)),
            2 => actor.handle(Action::ClearLine(LineClearMode::All)),
            _ => fallback(),
        },
        _ => fallback(),
    }
}

fn handle_insert_line<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::InsertBlankLines(1)),
        [Integer(count)] => {
            actor.handle(Action::InsertBlankLines(*count as usize));
        },
        _ => fallback(),
    }
}

fn handle_delete_line<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::DeleteLines(1)),
        [Integer(count)] => {
            actor.handle(Action::DeleteLines(*count as usize));
        },
        _ => fallback(),
    }
}

fn handle_device_status_report<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::ReportDeviceStatus(0)),
        [Integer(report)] => {
            actor.handle(Action::ReportDeviceStatus(*report as usize))
        },
        _ => fallback(),
    }
}

fn handle_delete_character<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::DeleteChars(1)),
        [Integer(count)] => {
            actor.handle(Action::DeleteChars(*count as usize));
        },
        _ => fallback(),
    }
}

fn handle_set_scrolling_region<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [Integer(top), P(b';'), Integer(bottom)] => actor.handle(
            Action::SetScrollingRegion(*top as usize, *bottom as usize),
        ),
        _ => fallback(),
    }
}

fn handle_scroll_up<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::ScrollUp(1)),
        [Integer(count)] => {
            actor.handle(Action::ScrollUp(*count as usize));
        },
        _ => fallback(),
    }
}

fn handle_scroll_down<A, F>(actor: &mut A, params: &[CsiParam], fallback: F)
where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::ScrollDown(1)),
        [Integer(count)] => {
            actor.handle(Action::ScrollDown(*count as usize));
        },
        _ => fallback(),
    }
}

fn handle_window_manipulation<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [Integer(id)] => match *id {
            14 => actor.handle(Action::RequestTextAreaSizeByPixels),
            18 => actor.handle(Action::RequestTextAreaSizeByChars),
            22 => actor.handle(Action::PushWindowTitle),
            23 => actor.handle(Action::PopWindowTitle),
            _ => fallback(),
        },
        _ => fallback(),
    }
}

fn handle_erase_characters<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::EraseChars(1)),
        [Integer(count)] => {
            actor.handle(Action::EraseChars(*count as usize));
        },
        _ => fallback(),
    }
}

fn handle_cursor_backward_tabulation<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    use CsiParam::*;

    match params {
        [] => actor.handle(Action::MoveBackwardTabs(1)),
        [Integer(count)] => {
            actor.handle(Action::MoveBackwardTabs(*count as u16));
        },
        _ => fallback(),
    }
}

#[inline]
fn handle_set_sgr_attribute<A, F>(
    actor: &mut A,
    params: &[CsiParam],
    fallback: F,
) where
    A: EscapeActor,
    F: Fn(),
{
    let mut iter = params.iter().peekable();

    use CsiParam::*;

    while let Some(param) = iter.next() {
        let attr = match param {
            Integer(0) => Some(CharacterAttribute::Reset),
            Integer(1) => Some(CharacterAttribute::Bold),
            Integer(2) => Some(CharacterAttribute::Dim),
            Integer(3) => Some(CharacterAttribute::Italic),
            Integer(4) => match iter.peek().copied() {
                Some(Integer(0)) => {
                    iter.next();
                    Some(CharacterAttribute::CancelUnderline)
                },
                Some(Integer(2)) => {
                    iter.next();
                    Some(CharacterAttribute::DoubleUnderline)
                },
                Some(Integer(3)) => {
                    iter.next();
                    Some(CharacterAttribute::Undercurl)
                },
                Some(Integer(4)) => {
                    iter.next();
                    Some(CharacterAttribute::DottedUnderline)
                },
                Some(Integer(5)) => {
                    iter.next();
                    Some(CharacterAttribute::DashedUnderline)
                },
                _ => Some(CharacterAttribute::Underline),
            },
            Integer(5) => Some(CharacterAttribute::BlinkSlow),
            Integer(6) => Some(CharacterAttribute::BlinkFast),
            Integer(7) => Some(CharacterAttribute::Reverse),
            Integer(8) => Some(CharacterAttribute::Hidden),
            Integer(9) => Some(CharacterAttribute::Strike),
            Integer(21) => Some(CharacterAttribute::CancelBold),
            Integer(22) => Some(CharacterAttribute::CancelBoldDim),
            Integer(23) => Some(CharacterAttribute::CancelItalic),
            Integer(24) => Some(CharacterAttribute::CancelUnderline),
            Integer(25) => Some(CharacterAttribute::CancelBlink),
            Integer(27) => Some(CharacterAttribute::CancelReverse),
            Integer(28) => Some(CharacterAttribute::CancelHidden),
            Integer(29) => Some(CharacterAttribute::CancelStrike),
            Integer(30) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Black,
            ))),
            Integer(31) => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Red)))
            },
            Integer(32) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Green,
            ))),
            Integer(33) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Yellow,
            ))),
            Integer(34) => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Blue)))
            },
            Integer(35) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Magenta,
            ))),
            Integer(36) => {
                Some(CharacterAttribute::Foreground(Color::Std(StdColor::Cyan)))
            },
            Integer(37) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::White,
            ))),
            Integer(38) => {
                parse_sgr_color(&mut iter).map(CharacterAttribute::Foreground)
            },
            Integer(39) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::Foreground,
            ))),
            Integer(40) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Black,
            ))),
            Integer(41) => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Red)))
            },
            Integer(42) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Green,
            ))),
            Integer(43) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Yellow,
            ))),
            Integer(44) => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Blue)))
            },
            Integer(45) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Magenta,
            ))),
            Integer(46) => {
                Some(CharacterAttribute::Background(Color::Std(StdColor::Cyan)))
            },
            Integer(47) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::White,
            ))),
            Integer(48) => {
                parse_sgr_color(&mut iter).map(CharacterAttribute::Background)
            },
            Integer(49) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::Background,
            ))),
            Integer(58) => parse_sgr_color(&mut iter)
                .map(|color| CharacterAttribute::UnderlineColor(Some(color))),
            Integer(59) => Some(CharacterAttribute::UnderlineColor(None)),
            Integer(90) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightBlack,
            ))),
            Integer(91) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightRed,
            ))),
            Integer(92) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightGreen,
            ))),
            Integer(93) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightYellow,
            ))),
            Integer(94) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightBlue,
            ))),
            Integer(95) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightMagenta,
            ))),
            Integer(96) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightCyan,
            ))),
            Integer(97) => Some(CharacterAttribute::Foreground(Color::Std(
                StdColor::BrightWhite,
            ))),
            Integer(100) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightBlack,
            ))),
            Integer(101) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightRed,
            ))),
            Integer(102) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightGreen,
            ))),
            Integer(103) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightYellow,
            ))),
            Integer(104) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightBlue,
            ))),
            Integer(105) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightMagenta,
            ))),
            Integer(106) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightCyan,
            ))),
            Integer(107) => Some(CharacterAttribute::Background(Color::Std(
                StdColor::BrightWhite,
            ))),
            _ => None,
        };

        if let Some(attr) = attr {
            actor.handle(Action::SGR(attr));
        } else {
            fallback()
        }
    }
}

fn parse_cursor_style(raw_shape: i64) -> Option<CursorStyle> {
    let shape = match raw_shape {
        0..=2 => Some(CursorShape::Block),
        3 | 4 => Some(CursorShape::Underline),
        5 | 6 => Some(CursorShape::Beam),
        _ => None,
    };

    shape.map(|shape| CursorStyle {
        shape,
        blinking: raw_shape % 2 == 1,
    })
}

#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EscapeParser,
        color::Rgb,
        keyboard::{KeyboardMode, KeyboardModeApplyBehavior},
        mode::{NamedMode, NamedPrivateMode},
        parser::Parser,
    };

    #[derive(Default)]
    struct RecordingEscapeActor {
        actions: Vec<Action>,
        begin_sync_calls: usize,
        end_sync_calls: usize,
    }

    impl EscapeActor for RecordingEscapeActor {
        fn handle(&mut self, action: Action) {
            self.actions.push(action);
        }

        fn begin_sync(&mut self) {
            self.begin_sync_calls += 1;
        }

        fn end_sync(&mut self) {
            self.end_sync_calls += 1;
        }
    }

    impl RecordingEscapeActor {
        fn parse(input: &str) -> Self {
            let mut parser: Parser<otty_vte::Parser> = Parser::new();
            let mut actor = Self::default();
            parser.advance(input.as_bytes(), &mut actor);
            actor
        }
    }

    #[test]
    fn csi_mode_sequences() {
        let actor = RecordingEscapeActor::parse(
            "\x1b[4h\x1b[4l\x1b[?25h\x1b[?25l\x1b[?2026h\x1b[?2026l\x1b[?1h\x1b[?1l\x1b[!p\x1b[20$p\x1b[?25$p\x1b[0$p",
        );

        assert_eq!(actor.begin_sync_calls, 1);
        assert_eq!(actor.end_sync_calls, 2);
        assert_eq!(actor.actions.len(), 11);

        assert_eq!(
            actor.actions,
            vec![
                Action::SetMode(Mode::Named(NamedMode::Insert)),
                Action::UnsetMode(Mode::Named(NamedMode::Insert)),
                Action::SetPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::ShowCursor
                )),
                Action::UnsetPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::ShowCursor
                )),
                Action::SetPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::SyncUpdate
                )),
                Action::UnsetPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::SyncUpdate
                )),
                Action::SetPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::CursorKeys
                )),
                Action::UnsetPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::CursorKeys
                )),
                Action::ReportMode(Mode::Named(NamedMode::LineFeedNewLine)),
                Action::ReportPrivateMode(PrivateMode::Named(
                    NamedPrivateMode::ShowCursor
                )),
                Action::ReportMode(Mode::Unknown(0))
            ]
        );
    }

    #[test]
    fn csi_modify_other_keys() {
        let cases = vec![
            ("\x1b[?4m", vec![Action::ReportModifyOtherKeysState]),
            (
                "\x1b[>4;1m",
                vec![Action::SetModifyOtherKeysState(
                    ModifyOtherKeysState::EnableExceptWellDefined,
                )],
            ),
        ];

        for (input, expected) in cases {
            let actual = RecordingEscapeActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn csi_keyboard_controls() {
        let cases = vec![
            (
                "\x1b[=5;2u",
                vec![Action::SetKeyboardMode(
                    KeyboardMode::DISAMBIGUATE_ESC_CODES
                        | KeyboardMode::REPORT_ALTERNATE_KEYS,
                    KeyboardModeApplyBehavior::Union,
                )],
            ),
            (
                "\x1b[>3u",
                vec![Action::PushKeyboardMode(
                    KeyboardMode::DISAMBIGUATE_ESC_CODES
                        | KeyboardMode::REPORT_EVENT_TYPES,
                )],
            ),
            ("\x1b[<4u", vec![Action::PopKeyboardModes(4)]),
            ("\x1b[?u", vec![Action::ReportKeyboardMode]),
            ("\x1b[<0u", vec![Action::PopKeyboardModes(1)]),
        ];

        for (input, expected) in cases {
            let actual = RecordingEscapeActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn csi_window_manipulation() {
        let actor =
            RecordingEscapeActor::parse("\x1b[14t\x1b[18t\x1b[22t\x1b[23t");

        assert_eq!(actor.begin_sync_calls, 0);
        assert_eq!(actor.end_sync_calls, 0);
        assert_eq!(actor.actions.len(), 4);

        assert_eq!(
            actor.actions,
            vec![
                Action::RequestTextAreaSizeByPixels,
                Action::RequestTextAreaSizeByChars,
                Action::PushWindowTitle,
                Action::PopWindowTitle
            ]
        );
    }

    #[test]
    fn csi_cursor_motion_and_positioning_sequences() {
        let actor = RecordingEscapeActor::parse(
            "\x1b[A\x1b[5A\x1b[B\x1b[2e\x1b[3C\x1b[4a\x1b[2D\x1b[3d\x1b[E\x1b[2F\x1b[10G\x1b[6`\x1b[5;9H\x1b[3;4f",
        );

        assert_eq!(actor.actions.len(), 14);

        assert_eq!(
            actor.actions,
            vec![
                Action::MoveUp {
                    rows: 1,
                    carrage_return_needed: false,
                },
                Action::MoveUp {
                    rows: 5,
                    carrage_return_needed: false,
                },
                Action::MoveDown {
                    rows: 1,
                    carrage_return_needed: false,
                },
                Action::MoveDown {
                    rows: 2,
                    carrage_return_needed: false,
                },
                Action::MoveForward(3),
                Action::MoveForward(4),
                Action::MoveBackward(2),
                Action::GotoRow(3),
                Action::MoveDown {
                    rows: 1,
                    carrage_return_needed: true,
                },
                Action::MoveUp {
                    rows: 2,
                    carrage_return_needed: true,
                },
                Action::GotoColumn(10),
                Action::GotoColumn(6),
                Action::Goto(5, 9),
                Action::Goto(3, 4)
            ]
        );
    }

    #[test]
    fn csi_tab_scrolling_and_region_sequences() {
        let actor = RecordingEscapeActor::parse(
            "\x1b[?5W\x1b[g\x1b[3g\x1b[I\x1b[4I\x1b[Z\x1b[3Z\x1b[1;24r\x1b[S\x1b[2S\x1b[T\x1b[3T",
        );

        assert_eq!(actor.actions.len(), 12);

        assert_eq!(
            actor.actions,
            vec![
                Action::SetTabs(8),
                Action::ClearTabs(TabClearMode::Current),
                Action::ClearTabs(TabClearMode::All),
                Action::MoveForwardTabs(1),
                Action::MoveForwardTabs(4),
                Action::MoveBackwardTabs(1),
                Action::MoveBackwardTabs(3),
                Action::SetScrollingRegion(1, 24),
                Action::ScrollUp(1),
                Action::ScrollUp(2),
                Action::ScrollDown(1),
                Action::ScrollDown(3),
            ]
        );
    }

    #[test]
    fn csi_editing_sequences() {
        let cases = vec![
            ("\x1b[1@", vec![Action::InsertBlank(1)]),
            ("\x1b[5@", vec![Action::InsertBlank(5)]),
            ("\x1b[L", vec![Action::InsertBlankLines(1)]),
            ("\x1b[3L", vec![Action::InsertBlankLines(3)]),
            ("\x1b[M", vec![Action::DeleteLines(1)]),
            ("\x1b[2M", vec![Action::DeleteLines(2)]),
            ("\x1b[P", vec![Action::DeleteChars(1)]),
            ("\x1b[2P", vec![Action::DeleteChars(2)]),
            ("\x1b[X", vec![Action::EraseChars(1)]),
            ("\x1b[4X", vec![Action::EraseChars(4)]),
            ("\x1b[J", vec![Action::ClearScreen(ClearMode::Below)]),
            ("\x1b[1J", vec![Action::ClearScreen(ClearMode::Above)]),
            ("\x1b[2J", vec![Action::ClearScreen(ClearMode::All)]),
            ("\x1b[3J", vec![Action::ClearScreen(ClearMode::Saved)]),
            ("\x1b[K", vec![Action::ClearLine(LineClearMode::Right)]),
            ("\x1b[1K", vec![Action::ClearLine(LineClearMode::Left)]),
            ("\x1b[2K", vec![Action::ClearLine(LineClearMode::All)]),
        ];

        for (input, expected) in cases {
            let actual = RecordingEscapeActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn csi_device_status_and_identify_sequences() {
        let actor =
            RecordingEscapeActor::parse("\x1b[n\x1b[6n\x1b[c\x1b[>c\x1b[65c");

        assert_eq!(actor.actions.len(), 5);

        assert_eq!(
            actor.actions,
            vec![
                Action::ReportDeviceStatus(0),
                Action::ReportDeviceStatus(6),
                Action::IdentifyTerminal(None),
                Action::IdentifyTerminal(Some('>')),
                Action::IdentifyTerminal(Some('A'))
            ]
        );
    }

    #[test]
    fn csi_sgr_sequences_cover_standard_and_extended_colors() {
        let cases = vec![
            ("\x1b[m", vec![Action::SGR(CharacterAttribute::Reset)]),
            (
                "\x1b[1;31m",
                vec![
                    Action::SGR(CharacterAttribute::Bold),
                    Action::SGR(CharacterAttribute::Foreground(Color::Std(
                        StdColor::Red,
                    ))),
                ],
            ),
            (
                "\x1b[38;5;42m",
                vec![Action::SGR(CharacterAttribute::Foreground(
                    Color::Indexed(42),
                ))],
            ),
            (
                "\x1b[38;2;10;20;30m",
                vec![Action::SGR(CharacterAttribute::Foreground(
                    Color::TrueColor(Rgb {
                        r: 10,
                        g: 20,
                        b: 30,
                    }),
                ))],
            ),
            (
                "\x1b[58;2;40;50;60m",
                vec![Action::SGR(CharacterAttribute::UnderlineColor(Some(
                    Color::TrueColor(Rgb {
                        r: 40,
                        g: 50,
                        b: 60,
                    }),
                )))],
            ),
            (
                "\x1b[59m",
                vec![Action::SGR(CharacterAttribute::UnderlineColor(None))],
            ),
            (
                "\x1b[90;100m",
                vec![
                    Action::SGR(CharacterAttribute::Foreground(Color::Std(
                        StdColor::BrightBlack,
                    ))),
                    Action::SGR(CharacterAttribute::Background(Color::Std(
                        StdColor::BrightBlack,
                    ))),
                ],
            ),
            ("\x1b[38;2;300;0;0m", vec![]),
        ];

        for (input, expected) in cases {
            let actual = RecordingEscapeActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn cursor_position_save_and_restore() {
        let cases = vec![
            ("\x1b[s", vec![Action::SaveCursorPosition]),
            ("\x1b[u", vec![Action::RestoreCursorPosition]),
        ];

        for (input, expected) in cases {
            let actual = RecordingEscapeActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn csi_cursor_style() {
        let cases = vec![
            (
                "\x1b[5 q",
                vec![Action::SetCursorStyle(Some(CursorStyle {
                    shape: CursorShape::Beam,
                    blinking: true,
                }))],
            ),
            ("\x1b[9 q", vec![Action::SetCursorStyle(None)]),
        ];

        for (input, expected) in cases {
            let actual = RecordingEscapeActor::parse(input).actions;
            assert_eq!(expected, actual)
        }
    }

    #[test]
    fn csi_repeat_character() {
        let actor = RecordingEscapeActor::parse("A\x1b[3b");

        assert_eq!(actor.actions.len(), 4);

        assert_eq!(
            actor.actions,
            vec![
                Action::Print('A'),
                Action::Print('A'),
                Action::Print('A'),
                Action::Print('A'),
            ]
        );
    }

    #[test]
    fn mixed_sequences_preserve_order() {
        let actor = RecordingEscapeActor::parse("A\x1b[2J\x1b[sB\x1b[u\x1b[0K");

        assert_eq!(actor.actions.len(), 6);

        assert_eq!(
            actor.actions,
            vec![
                Action::Print('A'),
                Action::ClearScreen(ClearMode::All),
                Action::SaveCursorPosition,
                Action::Print('B'),
                Action::RestoreCursorPosition,
                Action::ClearLine(LineClearMode::Right)
            ]
        );
    }
}
