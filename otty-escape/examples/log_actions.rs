//! Print every [`Action`] emitted by the parser for a given byte stream.
use otty_escape::{Action, Actor, Parser};

#[derive(Default)]
struct LoggingActor {
    seq: usize,
}

impl Actor for LoggingActor {
    fn handle(&mut self, action: Action) {
        self.seq += 1;
        println!("{:02}: {action:?}", self.seq);
    }
}

fn main() {
    let mut parser = Parser::new();
    let mut actor = LoggingActor::default();

    let bytes = b"Hello \x1b[1mOtty\x1b[0m!\n\
                  \x1b]8;id=docs;https://otty.sh\x07click me\x1b]8;;\x07";

    parser.advance(bytes, &mut actor);
}
