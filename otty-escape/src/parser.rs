use crate::{
    actor::Actor, charset::{Charset, CharsetIndex}, color::StdColor, control::ControlCode, esc::EscSequence, osc::OperatingSystemCommand
};
use log::debug;
use otty_vte::{Actor as VteActor, CsiParam, Parser as VTParser};

/// High-level escape sequence parser that forwards semantic events to an
/// [`Actor`](crate::actor::Actor).
#[derive(Default)]
pub struct Parser {
    vt: VTParser,
    state: ParseState,
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the parser with a new chunk of bytes.
    pub fn advance<A: Actor>(&mut self, bytes: &[u8], actor: &mut A) {
        let mut performer = Performer {
            actor,
            state: &mut self.state,
        };
        self.vt.advance(bytes, &mut performer);
    }
}

#[derive(Default)]
struct ParseState {
    // short_dcs: Option<ShortDeviceControl>,
    // get_tcap: Option<GetTcapBuilder>,
}

struct Performer<'a, A: Actor> {
    actor: &'a mut A,
    state: &'a mut ParseState,
}

impl<'a, A: Actor> VteActor for Performer<'a, A> {
    fn print(&mut self, c: char) {
        self.actor.print(c);
    }

    fn execute(&mut self, byte: u8) {
        ControlCode::perform(byte, self.actor);
    }

    fn hook(
        &mut self,
        byte: u8,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
    ) {
        debug!(
            "[unexpected hook] params: {:?}, intermediates: {:?}, ignore: {:?}, action: {:?}",
            params, intermediates, ignored_excess_intermediates, byte
        );
    }

    fn unhook(&mut self) {
        debug!("[unexpected unhook]");
    }

    fn put(&mut self, byte: u8) {
        debug!("[unexpected put] byte={:?}", byte);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        if params.is_empty() || params[0].is_empty() {
            return;
        }

        use OperatingSystemCommand::*;
        match OperatingSystemCommand::from(params[0]) {
            ResetIndexedColors => self.reset_indexed_colors(params),
            ResetBackgroundColor => self.actor.reset_color(StdColor::Background as usize),
            ResetForegroundColor => self.actor.reset_color(StdColor::Foreground as usize),
            ResetCursorColor => self.actor.reset_color(StdColor::Cursor as usize),
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        intermediates: &[u8],
        parameters_truncated: bool,
        byte: u8,
    ) {}

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        intermediates: &[u8],
        _ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        use EscSequence::*;
        let esc = EscSequence::from((intermediates, byte));

        match esc {
            Index => self.actor.linefeed(),
            NextLine => {
                self.actor.linefeed();
                self.actor.carriage_return();
            },
            HorizontalTabSet => self.actor.set_horizontal_tab(),
            ReverseIndex => self.actor.reverse_index(),
            ReturnTerminalId => self.actor.identify_terminal(None),
            FullReset => self.actor.reset_state(),
            DecSaveCursorPosition => self.actor.save_cursor_position(),
            DecScreenAlignmentDisplay => self.actor.screen_alignment_display(),
            DecRestoreCursorPosition => self.actor.restore_cursor_position(),
            DecApplicationKeyPad => self.actor.set_keypad_application_mode(),
            DecNormalKeyPad => self.actor.unset_keypad_application_mode(),
            DecLineDrawingG0 => self.actor.configure_charset(Charset::DecLineDrawing(CharsetIndex::G0)),
            DecLineDrawingG1 => self.actor.configure_charset(Charset::DecLineDrawing(CharsetIndex::G1)),
            DecLineDrawingG2 => self.actor.configure_charset(Charset::DecLineDrawing(CharsetIndex::G2)),
            DecLineDrawingG3 => self.actor.configure_charset(Charset::DecLineDrawing(CharsetIndex::G3)),
            AsciiCharacterSetG0 => self.actor.configure_charset(Charset::Ascii(CharsetIndex::G0)),
            AsciiCharacterSetG1 => self.actor.configure_charset(Charset::Ascii(CharsetIndex::G1)),
            AsciiCharacterSetG2 => self.actor.configure_charset(Charset::Ascii(CharsetIndex::G2)),
            AsciiCharacterSetG3 => self.actor.configure_charset(Charset::Ascii(CharsetIndex::G3)),
            // now do nothing
            _ => {}
        }
    }
}

impl<'a, A: Actor> Performer<'a, A> {
    fn reset_indexed_colors(&mut self, params: &[&[u8]]) {
        if params.len() == 1 || params[1].is_empty() {
            // Reset all
            for i in 0..256 {
                self.actor.reset_color(i);
            }
        } else {
            // Reset by params
            // for param in &params[1..] {
            //     match parse_number(param) {
            //         Some(index) => self.actor.reset_color(index as usize),
            //         None => {},
            //     }
            // }
        }
    }
}
