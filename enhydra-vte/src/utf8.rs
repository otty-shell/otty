use utf8parse::Receiver;

#[derive(Debug, Default)]
pub struct Decoder {
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
