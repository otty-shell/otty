use std::borrow::Cow;

use crate::{
    actor::{Actor, ControlCode},
    csi::CsiSequence,
    dcs::{self, EnterDeviceControl, ShortDeviceControl},
    esc::EscapeSequence,
    osc::OperatingSystemCommand,
};
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
    short_dcs: Option<ShortDeviceControl>,
    get_tcap: Option<GetTcapBuilder>,
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
        self.actor.control(ControlCode::from_byte(byte));
    }

    fn hook(
        &mut self,
        byte: u8,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
    ) {
        self.state.short_dcs.take();
        self.state.get_tcap.take();

        if byte == b'q' && intermediates == [b'+'] {
            self.state.get_tcap.replace(GetTcapBuilder::default());
            return;
        }

        if !ignored_excess_intermediates
            && dcs::is_short_dcs(intermediates, byte)
        {
            self.state.short_dcs.replace(ShortDeviceControl {
                params: params.to_vec(),
                intermediates: intermediates.to_vec(),
                byte,
                data: Vec::new(),
            });
            return;
        }

        self.actor.device_control_enter(EnterDeviceControl {
            params: params.to_vec(),
            intermediates: intermediates.to_vec(),
            byte,
            ignored_extra_intermediates: ignored_excess_intermediates,
        });
    }

    fn unhook(&mut self) {
        if let Some(short) = self.state.short_dcs.take() {
            self.actor.short_device_control(short);
        } else if let Some(builder) = self.state.get_tcap.take() {
            self.actor.xt_get_tcap(builder.finish());
        } else {
            self.actor.device_control_exit();
        }
    }

    fn put(&mut self, byte: u8) {
        if let Some(short) = self.state.short_dcs.as_mut() {
            short.data.push(byte);
        } else if let Some(builder) = self.state.get_tcap.as_mut() {
            builder.push(byte);
        } else {
            self.actor.device_control_data(byte);
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        let arguments = params.iter().map(|slice| slice.to_vec()).collect();
        self.actor.osc(OperatingSystemCommand { arguments });
    }

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        intermediates: &[u8],
        parameters_truncated: bool,
        byte: u8,
    ) {
        self.actor.csi(CsiSequence {
            params: params.to_vec(),
            intermediates: intermediates.to_vec(),
            parameters_truncated,
            final_byte: byte,
        });
    }

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        intermediates: &[u8],
        _ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        let escape_sequence = EscapeSequence::parse(intermediates, byte);
        self.actor.esc(escape_sequence);
    }
}

#[derive(Default)]
struct GetTcapBuilder {
    current: Vec<u8>,
    names: Vec<String>,
}

impl GetTcapBuilder {
    fn push(&mut self, byte: u8) {
        if byte == b';' {
            self.flush_current();
        } else {
            self.current.push(byte);
        }
    }

    fn finish(mut self) -> Vec<String> {
        self.flush_current();
        self.names
    }

    fn flush_current(&mut self) {
        if self.current.is_empty() {
            return;
        }

        let text: Cow<'_, str> = decode_hex(&self.current)
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .map(Cow::Owned)
            .unwrap_or_else(|| {
                Cow::Owned(String::from_utf8_lossy(&self.current).into_owned())
            });

        self.names.push(text.into_owned());
        self.current.clear();
    }
}

fn decode_hex(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() % 2 != 0 {
        return None;
    }

    let mut out = Vec::with_capacity(data.len() / 2);
    for chunk in data.chunks(2) {
        let hi = decode_nibble(chunk[0])?;
        let lo = decode_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn decode_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(10 + byte - b'a'),
        b'A'..=b'F' => Some(10 + byte - b'A'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otty_vte::CsiParam;

    #[derive(Default)]
    struct RecordingActor {
        events: Vec<Event>,
    }

    #[derive(Debug, PartialEq)]
    enum Event {
        Print(char),
        Control(ControlCode),
        Csi(CsiSequence),
        Esc(EscapeSequence),
        Osc(OperatingSystemCommand),
        Enter(EnterDeviceControl),
        Data(u8),
        Exit,
        Short(ShortDeviceControl),
        XtGetTcap(Vec<String>),
    }

    impl Actor for RecordingActor {
        fn print(&mut self, c: char) {
            self.events.push(Event::Print(c));
        }

        fn control(&mut self, code: ControlCode) {
            self.events.push(Event::Control(code));
        }

        fn csi(&mut self, seq: CsiSequence) {
            self.events.push(Event::Csi(seq));
        }

        fn esc(&mut self, seq: EscapeSequence) {
            self.events.push(Event::Esc(seq));
        }

        fn osc(&mut self, osc: OperatingSystemCommand) {
            self.events.push(Event::Osc(osc));
        }

        fn device_control_enter(&mut self, mode: EnterDeviceControl) {
            self.events.push(Event::Enter(mode));
        }

        fn device_control_data(&mut self, byte: u8) {
            self.events.push(Event::Data(byte));
        }

        fn device_control_exit(&mut self) {
            self.events.push(Event::Exit);
        }

        fn short_device_control(&mut self, short: ShortDeviceControl) {
            self.events.push(Event::Short(short));
        }

        fn xt_get_tcap(&mut self, names: Vec<String>) {
            self.events.push(Event::XtGetTcap(names));
        }
    }

    #[test]
    fn print_and_control() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"a\x07", &mut actor);

        assert_eq!(
            actor.events,
            vec![Event::Print('a'), Event::Control(ControlCode::Bell),]
        );
    }

    #[test]
    fn csi_sequence() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"\x1b[31m", &mut actor);

        assert_eq!(
            actor.events,
            vec![Event::Csi(CsiSequence {
                params: vec![CsiParam::Integer(31)],
                intermediates: Vec::new(),
                parameters_truncated: false,
                final_byte: b'm',
            })]
        );
    }

    #[test]
    fn osc_sequence() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"\x1b]0;hello\x07", &mut actor);

        assert_eq!(
            actor.events,
            vec![Event::Osc(OperatingSystemCommand {
                arguments: vec![b"0".to_vec(), b"hello".to_vec()],
            })]
        );
    }

    #[test]
    fn esc_sequence() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"\x1b7", &mut actor);

        assert_eq!(
            actor.events,
            vec![Event::Esc(EscapeSequence::DecSaveCursorPosition)]
        );
    }

    #[test]
    fn device_control_streaming() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"\x1bP1;2pab\x1b\\", &mut actor);

        assert_eq!(
            actor.events,
            vec![
                Event::Enter(EnterDeviceControl {
                    params: vec![1, 0, 2],
                    intermediates: Vec::new(),
                    byte: b'p',
                    ignored_extra_intermediates: false,
                }),
                Event::Data(b'a'),
                Event::Data(b'b'),
                Event::Exit,
                Event::Esc(EscapeSequence::StringTerminator),
            ]
        );
    }

    #[test]
    fn short_device_control_sequence() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"\x1bP$qabc\x1b\\", &mut actor);

        assert_eq!(
            actor.events,
            vec![
                Event::Short(ShortDeviceControl {
                    params: Vec::new(),
                    intermediates: vec![b'$'],
                    byte: b'q',
                    data: b"abc".to_vec(),
                }),
                Event::Esc(EscapeSequence::StringTerminator)
            ]
        );
    }

    #[test]
    fn xt_get_tcap_sequence() {
        let mut parser = Parser::new();
        let mut actor = RecordingActor::default();
        parser.advance(b"\x1bP+q616263;7A\x1b\\", &mut actor);

        assert_eq!(
            actor.events,
            vec![
                Event::XtGetTcap(vec!["abc".to_string(), "z".to_string()]),
                Event::Esc(EscapeSequence::StringTerminator),
            ]
        );
    }
}
