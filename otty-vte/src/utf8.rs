use crate::enums::State;
use utf8parse::Receiver;

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

    pub(crate) fn advance(&mut self, byte: u8) -> Decoder {
        let mut decoder = Decoder::new();
        self.inner.advance(&mut decoder, byte);
        decoder
    }
}
