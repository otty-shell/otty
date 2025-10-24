use crate::enums::{Action, State};
use crate::transitions;
use utf8parse::Receiver;

/// Result of advancing the UTF-8 parser by one byte.
pub(crate) enum StepResult {
    /// More bytes are needed to complete a codepoint.
    Pending,
    /// A decoded character is ready.
    Completed(char),
    /// A control-like transition should be applied by the caller.
    Control {
        byte: u8,
        from: State,
        next: State,
        action: Action,
    },
}

#[derive(Default)]
pub(crate) struct Decoder {
    inner: Option<char>,
}

impl Decoder {
    #[must_use]
    pub fn new() -> Self {
        Decoder::default()
    }

    pub fn get(self) -> Option<char> {
        self.inner
    }
}

impl Receiver for Decoder {
    fn codepoint(&mut self, c: char) {
        self.inner.replace(c);
    }

    fn invalid_sequence(&mut self) {
        self.codepoint(char::REPLACEMENT_CHARACTER);
    }
}

#[derive(Default)]
pub(crate) struct Utf8Parser {
    state: State,
    inner: utf8parse::Parser,
}

impl Utf8Parser {
    pub(crate) fn state(&self) -> State {
        self.state
    }

    pub(crate) fn set_state(&mut self, new_state: State) {
        self.state = new_state;
    }

    pub(crate) fn advance(&mut self, byte: u8) -> Option<char> {
        let mut decoder = Decoder::new();
        self.inner.advance(&mut decoder, byte);
        decoder.get()
    }

    /// Process a single byte of a UTF-8 sequence and classify the outcome.
    ///
    /// - Returns `StepResult::Pending` if more bytes are needed to complete a codepoint.
    /// - Returns `StepResult::Completed(c)` when a Unicode scalar value is decoded.
    /// - Returns `StepResult::Control { .. }` for special C1/control cases that should
    ///   be handled as state transitions rather than as printable data.
    pub(crate) fn step(&mut self, byte: u8) -> StepResult {
        let Some(c) = self.advance(byte) else {
            return StepResult::Pending;
        };

        // Handle special cases for C1 controls that may be encoded as UTF-8
        // rather than as raw 8-bit; if decoding yields a single-byte codepoint
        // and that would trigger a state transition, report that instead of
        // the printable character path so the caller can perform those actions.
        if (c as u32) <= 0xff {
            let byte = c as u8;
            let current = self.state();
            let (next_state, action) = transitions::transit(current, byte);

            if action == Action::Execute
                || (next_state != current && next_state != State::Utf8Sequence)
            {
                return StepResult::Control {
                    byte,
                    from: current,
                    next: next_state,
                    action,
                };
            }
        }

        StepResult::Completed(c as char)
    }
}
