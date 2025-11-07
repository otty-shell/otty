use crate::{Action, EscapeActor, EscapeParser, control, csi, esc, osc};
use log::debug;
use otty_vte::{self, CsiParam, VTActor, VTParser};

struct Performer<'a, A: EscapeActor> {
    actor: &'a mut A,
    state: &'a mut ParserState,
}

impl<'a, A: EscapeActor> VTActor for Performer<'a, A> {
    fn print(&mut self, c: char) {
        self.actor.handle(Action::Print(c));
        self.state.last_preceding_char = Some(c)
    }

    fn execute(&mut self, byte: u8) {
        control::perform(byte, self.actor);
    }

    fn hook(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
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
        debug!("[unexpected put] byte: {:?}", byte);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _: u8) {
        osc::perform(self.actor, params);
    }

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        _: &[u8],
        has_ignored_intermediates: bool,
        byte: u8,
    ) {
        csi::perform(
            self.actor,
            self.state,
            params,
            has_ignored_intermediates,
            byte,
        )
    }

    fn esc_dispatch(
        &mut self,
        _: &[i64],
        intermediates: &[u8],
        _: bool,
        byte: u8,
    ) {
        esc::perform(self.actor, intermediates, byte);
    }
}

impl<'a, A: EscapeActor> Performer<'a, A> {
    #[must_use]
    fn new(state: &'a mut ParserState, actor: &'a mut A) -> Self {
        Self { actor, state }
    }
}

#[derive(Default)]
pub(crate) struct ParserState {
    pub last_preceding_char: Option<char>,
}

/// High-level escape sequence parser that forwards semantic events to an
/// [`EscapeActor`](crate::actor::EscapeActor).
#[derive(Default)]
pub struct Parser<P: VTParser + Default> {
    vt: P,
    state: ParserState,
}

impl<P: VTParser + Default> EscapeParser for Parser<P> {
    /// Advance the parser with a new chunk of bytes.
    ///
    /// All escape sequences are parsed and forwarded to the actor as actions.
    /// Synchronized update buffering is handled by the terminal layer, not the parser.
    fn advance<A: EscapeActor>(&mut self, bytes: &[u8], actor: &mut A) {
        let mut performer = Performer::new(&mut self.state, actor);
        self.vt.advance(bytes, &mut performer);
    }
}

impl<P: VTParser + Default> Parser<P> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            vt: P::default(),
            state: ParserState::default(),
        }
    }
}

pub(crate) fn parse_number(input: &[u8]) -> Option<u8> {
    if input.is_empty() {
        return None;
    }

    input.iter().try_fold(0u8, |acc, &b| {
        let d = (b as char).to_digit(10)? as u8;
        acc.checked_mul(10)?.checked_add(d)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invalid_number() {
        assert_eq!(parse_number(b"1abc"), None);
    }

    #[test]
    fn parse_valid_number() {
        assert_eq!(parse_number(b"123"), Some(123));
    }

    #[test]
    fn parse_number_too_large() {
        assert_eq!(parse_number(b"321"), None);
    }
}
