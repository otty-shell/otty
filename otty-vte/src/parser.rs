use crate::actor::Actor;
use crate::enums::{Action, State};
use crate::{transitions, utf8};

const MAX_INTERMEDIATES: usize = 2;
const MAX_OSC_PARAMS: usize = 32;
const MAX_PARAMS: usize = 256;

/// Represents a parameter to a CSI-based escaped sequence.
///
/// CSI escapes typically have the form: `CSI 3 m`, but can also
/// bundle multiple values together: `CSI 3 ; 4 m`.  In both
/// of those examples the parameters are simple integer values
/// and latter of which would be expressed as a slice containing
/// `[CsiParam::Integer(3), CsiParam::Integer(4)]`.
///
/// There are some escape sequences that use colons to subdivide and
/// extend the meaning.  For example: `CSI 4:3 m` is a sequence used
/// to denote a curly underline.  That would be represented as:
/// `[CsiParam::ColonList(vec![Some(4), Some(3)])]`.
///
/// Later: reading ECMA 48, CSI is defined as:
/// CSI P ... P  I ... I  F
/// Where P are parameter bytes in the range 0x30-0x3F [0-9:;<=>?]
/// and I are intermediate bytes in the range 0x20-0x2F
/// and F is the final byte in the range 0x40-0x7E
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CsiParam {
    Integer(i64),
    P(u8),
}

impl Default for CsiParam {
    fn default() -> Self {
        Self::Integer(0)
    }
}

#[derive(Debug)]
struct Params {
    items: [CsiParam; MAX_PARAMS],
    current: Option<CsiParam>,
    full: bool,
    idx: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            items: [CsiParam::default(); MAX_PARAMS],
            current: None,
            full: false,
            idx: 0,
        }
    }
}

impl Params {
    fn get(&self) -> Vec<CsiParam> {
        self.items[..self.idx].to_vec()
    }

    fn get_integers(&self) -> Vec<i64> {
        self.items[..self.idx]
            .iter()
            .map(|param| {
                if let CsiParam::Integer(val) = param {
                    *val
                } else {
                    0
                }
            })
            .collect()
    }

    fn push(&mut self, param: CsiParam) {
        if self.idx >= MAX_PARAMS {
            self.full = true;
            return;
        }

        self.items[self.idx] = param;
        self.idx += 1;
    }

    fn finish(&mut self) {
        if let Some(val) = self.current.take() {
            self.push(val);
        }
    }

    fn clear(&mut self) {
        self.current.take();
        self.full = false;
        self.idx = 0;
        self.items = [CsiParam::default(); MAX_PARAMS];
    }
}

#[derive(Debug, Default)]
struct OscState {
    buffer: Vec<u8>,
    params: [usize; MAX_OSC_PARAMS],
    idx: usize,
    full: bool,
}

impl OscState {
    fn put(&mut self, param: char) {
        if param == ';' {
            match self.idx {
                MAX_OSC_PARAMS => {
                    self.full = true;
                }
                num => {
                    self.params[num.saturating_sub(1)] = self.buffer.len();
                    self.idx += 1;
                }
            }

            return;
        }

        if self.full {
            return;
        }

        let mut tmp = [0u8; 4];
        let bytes = param.encode_utf8(&mut tmp).as_bytes();
        self.buffer.extend_from_slice(bytes);

        if self.idx == 0 {
            self.idx = 1;
        }
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.idx = 0;
        self.full = false;
        self.params = Default::default();
    }
}

#[derive(Debug, Default)]
struct Intermediates {
    items: [u8; MAX_INTERMEDIATES],
    idx: usize,
    ignored_excess: bool,
}

impl Intermediates {
    fn get(&self) -> &[u8] {
        &self.items[..self.idx]
    }

    fn reset_index(&mut self) {
        self.idx = 0;
    }

    fn collect(&mut self, byte: u8) {
        if self.idx < MAX_INTERMEDIATES {
            self.items[self.idx] = byte;
            self.idx += 1;
        } else {
            self.ignored_excess = true;
        }
    }
    
    fn clear(&mut self) {
        self.reset_index();
        self.ignored_excess = false;
        self.items = Default::default();
    }
}

#[derive(Default)]
pub struct Parser {
    state: State,
    intermediates: Intermediates,
    params: Params,
    osc: OscState,
    utf8_parser: utf8::Utf8Parser
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance<A: Actor>(&mut self, bytes: &[u8], actor: &mut A) {
        for byte in bytes {
            let byte = *byte;

            if self.state == State::Utf8Sequence {
                self.advance_utf8(actor, byte);
                continue;
            }

            let (next_state, action) = transitions::transit(self.state, byte);

            if self.state == next_state {
                self.perform(action, byte, actor);
                continue;
            }

            if next_state != State::Utf8Sequence {
                self.perform(transitions::exit_action(self.state), 0, actor);
            }

            self.perform(action, byte, actor);
            self.perform(transitions::entry_action(next_state), byte, actor);
            
            self.utf8_parser.set_state(self.state);
            self.state = next_state;
        }
    }

    fn perform<A: Actor>(&mut self, action: Action, byte: u8, actor: &mut A) {
        use Action::*;

        match action {
            Print => actor.print(byte as char),
            Execute => actor.execute(byte),
            Put => actor.put(byte),
            CsiDispatch => self.csi_dispatch(actor, byte),
            EscDispatch => self.esc_dispatch(actor, byte),
            Param => self.handle_param_byte(byte),
            Clear => self.clear(),
            Collect => self.intermediates.collect(byte),
            Hook => self.hook(actor, byte),
            Unhook => actor.unhook(),
            OscStart => self.osc.clear(),
            OscPut => self.osc.put(byte as char),
            OscEnd => self.osc_dispatch(actor),
            Utf8 => self.advance_utf8(actor, byte),
            _ => {},
        }
    }

    // UTF-8 Parsing function (https://github.com/wezterm/wezterm/blob/main/vtparse/src/lib.rs#L669)
    fn advance_utf8<A: Actor>(&mut self, actor: &mut A, byte: u8) {
        let decoder = self.utf8_parser.advance(byte);

        let Some(c) = decoder.get() else {
            return;
        };

        // Slightly gross special cases C1 controls that were
        // encoded as UTF-8 rather than emitted as raw 8-bit.
        // If the decoded value is in the byte range, and that
        // value would cause a state transition, then we process
        // that state transition rather than performing the default
        // string accumulation.
        if c as u32 <= 0xff {
            let byte = c as u8;
            let utf8_parser_state = self.utf8_parser.state();

            let (next_state, action) =
                transitions::transit(utf8_parser_state, byte);

            if action == Action::Execute
                || (next_state != utf8_parser_state
                    && next_state != State::Utf8Sequence)
            {
                self.perform(
                    transitions::exit_action(utf8_parser_state),
                    0,
                    actor,
                );
                self.perform(action, byte, actor);
                self.perform(transitions::entry_action(next_state), 0, actor);

                self.utf8_parser.set_state(self.state);
                self.state = next_state;
                return;
            }
        }

        match self.utf8_parser.state() {
            State::Ground => actor.print(c),
            State::OscString => self.osc.put(c),
            state => panic!("unreachable state {:?}", state),
        };

        self.state = self.utf8_parser.state();
    }

    /// Promote early intermediates to parameters.
    /// This is handle sequences such as DECSET that use `?`
    /// prior to other numeric parameters.
    /// `?` is technically in the intermediate range and shouldn't
    /// appear in the parameter position according to ECMA 48
    fn promote_intermediates_to_params(&mut self) {
        if self.intermediates.idx > 0 {
            for &p in self.intermediates.get() {
                if self.params.full {
                    self.intermediates.ignored_excess = true;
                    break;
                }
                self.params.push(CsiParam::P(p));
            }
            self.intermediates.reset_index();
        }
    }

    fn handle_param_byte(&mut self, byte: u8) {
        if self.params.full {
            return;
        }

        self.promote_intermediates_to_params();

        if (b'0'..=b'9').contains(&byte) {
            let digit = (byte - b'0') as i64;
            match self.params.current.take() {
                Some(CsiParam::Integer(value)) => {
                    let updated = value.saturating_mul(10).saturating_add(digit);
                    self.params.current.replace(CsiParam::Integer(updated));
                }
                Some(param) => panic!("unexpected param: {param:?}"),
                None => {
                    self.params.current.replace(CsiParam::Integer(digit));
                }
            }
        } else {
            self.params.finish();
            self.params.push(CsiParam::P(byte));
        }
    }

    fn hook<A: Actor>(&mut self, actor: &mut A, byte: u8) {
        self.params.finish();
        actor.hook(
            byte,
            &self.params.get_integers(),
            self.intermediates.get(),
            self.intermediates.ignored_excess,
        );
    }

    fn csi_dispatch<A: Actor>(&mut self, actor: &mut A, byte: u8) {
        self.params.finish();
        self.promote_intermediates_to_params();
        actor.csi_dispatch(
            &self.params.get(),
            self.intermediates.ignored_excess,
            byte,
        );
    }

    fn esc_dispatch<A: Actor>(&mut self, actor: &mut A, byte: u8) {
        self.params.finish();
        actor.esc_dispatch(
        &self.params.get_integers(),
            &self.intermediates.get(),
            self.intermediates.ignored_excess,
            byte,
        );
    }

    fn osc_dispatch<A: Actor>(&mut self, actor: &mut A) {
        if self.osc.idx == 0 {
            actor.osc_dispatch(&[]);
            return;
        }

        let mut buffer = self.osc.buffer.as_slice();
        let limit = self.osc.idx.min(MAX_OSC_PARAMS);

        let mut params: Vec<&[u8]> = Vec::with_capacity(MAX_OSC_PARAMS);
        let mut offset = 0usize;

        for &end in &self.osc.params[..limit - 1] {
            let (a, b) = buffer.split_at(end - offset);
            params.push(a);
            buffer = b;
            offset = end;
        }

        params.push(&buffer);
        actor.osc_dispatch(&params[..limit]);
    }

    fn clear(&mut self) {
        self.intermediates.clear();
        self.params.clear();
        self.osc.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    pub enum ActorEvents {
        Print(char),
        Execute(u8),
        Hook {
            params: Vec<i64>,
            intermediates: Vec<u8>,
            ignored_excess_intermediates: bool,
            byte: u8,
        },
        Put(u8),
        Unhook,
        EscDispatch {
            params: Vec<i64>,
            intermediates: Vec<u8>,
            ignored_excess_intermediates: bool,
            byte: u8,
        },
        CsiDispatch {
            params: Vec<CsiParam>,
            parameters_truncated: bool,
            byte: u8,
        },
        OscDispatch(Vec<Vec<u8>>),
    }

    #[derive(Default)]
    struct CollectingActor {
        actions: Vec<ActorEvents>,
    }

    impl Actor for CollectingActor {
        fn print(&mut self, c: char) {
            self.actions.push(ActorEvents::Print(c));
        }

        fn execute(&mut self, byte: u8) {
            self.actions.push(ActorEvents::Execute(byte));
        }

        fn hook(
            &mut self,
            byte: u8,
            params: &[i64],
            intermediates: &[u8],
            ignored_excess_intermediates: bool,
        ) {
            self.actions.push(ActorEvents::Hook {
                params: params.to_vec(),
                intermediates: intermediates.to_vec(),
                ignored_excess_intermediates,
                byte,
            });
        }

        fn put(&mut self, byte: u8) {
            self.actions.push(ActorEvents::Put(byte));
        }

        fn unhook(&mut self) {
            self.actions.push(ActorEvents::Unhook);
        }

        fn esc_dispatch(
            &mut self,
            params: &[i64],
            intermediates: &[u8],
            ignored_excess_intermediates: bool,
            byte: u8,
        ) {
            self.actions.push(ActorEvents::EscDispatch {
                params: params.to_vec(),
                intermediates: intermediates.to_vec(),
                ignored_excess_intermediates,
                byte,
            });
        }

        fn csi_dispatch(
            &mut self,
            params: &[CsiParam],
            parameters_truncated: bool,
            byte: u8,
        ) {
            self.actions.push(ActorEvents::CsiDispatch {
                params: params.to_vec(),
                parameters_truncated,
                byte,
            });
        }

        fn osc_dispatch(&mut self, params: &[&[u8]]) {
            self.actions.push(ActorEvents::OscDispatch(
                params.iter().map(|e| e.to_vec()).collect(),
            ));
        }
    }

    fn parse(bytes: &[u8]) -> Vec<ActorEvents> {
        let mut parser = Parser::new();
        let mut actor = CollectingActor::default();
        parser.advance(bytes, &mut actor);
        actor.actions
    }

    #[test]
    fn parses_printable_ascii() {
        assert_eq!(
            parse(b"test\x07\x1b[32mmy\x1b[0mparser"),
            vec![
                ActorEvents::Print('t'),
                ActorEvents::Print('e'),
                ActorEvents::Print('s'),
                ActorEvents::Print('t'),
                ActorEvents::Execute(0x07),
                ActorEvents::CsiDispatch {
                    params: vec![CsiParam::Integer(32)],
                    parameters_truncated: false,
                    byte: b'm'
                },
                ActorEvents::Print('m'),
                ActorEvents::Print('y'),
                ActorEvents::CsiDispatch {
                    params: vec![CsiParam::Integer(0)],
                    parameters_truncated: false,
                    byte: b'm'
                },
                ActorEvents::Print('p'),
                ActorEvents::Print('a'),
                ActorEvents::Print('r'),
                ActorEvents::Print('s'),
                ActorEvents::Print('e'),
                ActorEvents::Print('r'),
            ]
        );
    }

    #[test]
    fn test_print() {
        assert_eq!(
            parse(b"yo"),
            vec![ActorEvents::Print('y'), ActorEvents::Print('o')]
        );
    }

    #[test]
    fn print_utf8() {
        assert_eq!(
            parse("\u{af}".as_bytes()),
            vec![ActorEvents::Print('\u{af}')]
        );
    }

    #[test]
    fn test_osc_with_c1_st() {
        assert_eq!(
            parse(b"\x1b]0;there\x9c"),
            vec![ActorEvents::OscDispatch(vec![
                b"0".to_vec(),
                b"there".to_vec()
            ])]
        );
    }

    #[test]
    fn test_osc_with_bel_st() {
        assert_eq!(
            parse(b"\x1b]0;hello\x07"),
            vec![ActorEvents::OscDispatch(vec![
                b"0".to_vec(),
                b"hello".to_vec()
            ])]
        );
    }

    #[test]
    fn test_osc_with_no_params() {
        assert_eq!(
            parse(b"\x1b]\x07"),
            vec![ActorEvents::OscDispatch(vec![])]
        );
    }

    #[test]
    fn test_decset() {
        assert_eq!(
            parse(b"\x1b[?1l"),
            vec![ActorEvents::CsiDispatch {
                params: vec![CsiParam::P(b'?'), CsiParam::Integer(1)],
                parameters_truncated: false,
                byte: b'l',
            },]
        );
    }

    #[test]
    fn test_osc_too_many_params() {
        let fields = (0..MAX_OSC_PARAMS + 2)
            .into_iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>();

        let input = format!("\x1b]{}\x07", fields.join(";"));
        let actions = parse(input.as_bytes());
        assert_eq!(actions.len(), 1);

        match &actions[0] {
            ActorEvents::OscDispatch(parsed_fields) => {
                let fields: Vec<_> = fields.into_iter().map(|s| s.as_bytes().to_vec()).collect();
                assert_eq!(parsed_fields.as_slice(), &fields[0..MAX_OSC_PARAMS]);
            }
            other => panic!("Expected OscDispatch but got {:?}", other),
        }
    }

    #[test]
    fn test_osc_with_esc_sequence_st() {
        // This case isn't the same as the other OSC cases; even though
        // `ESC \` is the long form escape sequence for ST, the ESC on its
        // own breaks out of the OSC state and jumps into the ESC state,
        // and that leaves the `\` character to be dispatched there in
        // the calling application.
        assert_eq!(
            parse(b"\x1b]woot\x1b\\"),
            vec![
                ActorEvents::OscDispatch(vec![b"woot".to_vec()]),
                ActorEvents::EscDispatch {
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                    byte: b'\\'
                }
            ]
        );
    }

   #[test]
    fn test_fancy_underline() {
        assert_eq!(
            parse(b"\x1b[4m"),
            vec![ActorEvents::CsiDispatch {
                params: vec![CsiParam::Integer(4)],
                parameters_truncated: false,
                byte: b'm'
            }]
        );

        assert_eq!(
            // This is the kitty curly underline sequence.
            parse(b"\x1b[4:3m"),
            vec![ActorEvents::CsiDispatch {
                params: vec![
                    CsiParam::Integer(4),
                    CsiParam::P(b':'),
                    CsiParam::Integer(3)
                ],
                parameters_truncated: false,
                byte: b'm'
            }]
        );
    }

    #[test]
    fn test_colon_rgb() {
        assert_eq!(
            parse(b"\x1b[38:2::128:64:192m"),
            vec![ActorEvents::CsiDispatch {
                params: vec![
                    CsiParam::Integer(38),
                    CsiParam::P(b':'),
                    CsiParam::Integer(2),
                    CsiParam::P(b':'),
                    CsiParam::P(b':'),
                    CsiParam::Integer(128),
                    CsiParam::P(b':'),
                    CsiParam::Integer(64),
                    CsiParam::P(b':'),
                    CsiParam::Integer(192),
                ],
                parameters_truncated: false,
                byte: b'm'
            }]
        );
    }

        #[test]
    fn test_csi_omitted_param() {
        assert_eq!(
            parse(b"\x1b[;1m"),
            vec![ActorEvents::CsiDispatch {
                params: vec![CsiParam::P(b';'), CsiParam::Integer(1)],
                parameters_truncated: false,
                byte: b'm'
            }]
        );
    }

    #[test]
    fn test_csi_too_many_params() {
        // Due to the much higher CSI element limit,
        // we must construct this test differently.
        let mut input = "\x1b[0".to_string();
        let mut params = vec![CsiParam::default()];

        for n in 1..=127 {
            input.push_str(&format!(";{n}"));
            params.push(CsiParam::P(b';'));
            params.push(CsiParam::Integer(n));
        }
        input.push_str(";128");

        input.push('p');
        params.push(CsiParam::P(b';'));

        assert_eq!(
            parse(input.as_bytes()),
            vec![ActorEvents::CsiDispatch {
                params,
                parameters_truncated: false,
                byte: b'p'
            }]
        );
    }

    #[test]
    fn test_csi_intermediates() {
        assert_eq!(
            parse(b"\x1b[1 p"),
            vec![ActorEvents::CsiDispatch {
                params: vec![CsiParam::Integer(1), CsiParam::P(b' ')],
                parameters_truncated: false,
                byte: b'p'
            }]
        );
        assert_eq!(
            parse(b"\x1b[1 !p"),
            vec![ActorEvents::CsiDispatch {
                params: vec![CsiParam::Integer(1), CsiParam::P(b' '), CsiParam::P(b'!')],
                parameters_truncated: false,
                byte: b'p'
            }]
        );
        assert_eq!(
            parse(b"\x1b[1 !#p"),
            vec![ActorEvents::CsiDispatch {
                // Note that the `#` was discarded
                params: vec![CsiParam::Integer(1), CsiParam::P(b' '), CsiParam::P(b'!')],
                parameters_truncated: true,
                byte: b'p'
            }]
        );
    }

    #[test]
    fn osc_utf8() {
        assert_eq!(
            parse("\x1b]\u{af}\x07".as_bytes()),
            vec![ActorEvents::OscDispatch(vec!["\u{af}".as_bytes().to_vec()])]
        );
    }

    #[test]
    fn osc_fedora_vte() {
        assert_eq!(
            parse("\u{9d}777;preexec\u{9c}".as_bytes()),
            vec![ActorEvents::OscDispatch(vec![
                b"777".to_vec(),
                b"preexec".to_vec(),
            ])]
        );
    }

    #[test]
    fn utf8_control() {
        assert_eq!(
            parse("\u{8d}".as_bytes()),
            vec![ActorEvents::Execute(0x8d)]
        );
    }

    #[test]
    fn tmux_control() {
        assert_eq!(
            parse("\x1bP1000phello\x1b\\".as_bytes()),
            vec![
                ActorEvents::Hook {
                    byte: b'p',
                    params: vec![1000],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                },
                ActorEvents::Put(b'h'),
                ActorEvents::Put(b'e'),
                ActorEvents::Put(b'l'),
                ActorEvents::Put(b'l'),
                ActorEvents::Put(b'o'),
                ActorEvents::Unhook,
                ActorEvents::EscDispatch {
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                    byte: b'\\',
                }
            ]
        );
    }

    #[test]
    fn tmux_passthrugh() {
        // I'm not convinced that we *should* represent this tmux sequence
        // in this way, but it is how it currently maps.
        // It's worth noting that we see this as final byte `t` here, which
        // collides with decVT105G in https://vt100.net/emu/dcsseq_dec.html
        assert_eq!(
            parse("\x1bPtmux;data\x1b\\".as_bytes()),
            vec![
                ActorEvents::Hook {
                    byte: b't',
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                },
                ActorEvents::Put(b'm'),
                ActorEvents::Put(b'u'),
                ActorEvents::Put(b'x'),
                ActorEvents::Put(b';'),
                ActorEvents::Put(b'd'),
                ActorEvents::Put(b'a'),
                ActorEvents::Put(b't'),
                ActorEvents::Put(b'a'),
                ActorEvents::Unhook,
                ActorEvents::EscDispatch {
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                    byte: b'\\',
                }
            ]
        );
    }

    #[test]
    fn sixel() {
        assert_eq!(
            parse("\x1bPqhello\x1b\\".as_bytes()),
            vec![
                ActorEvents::Hook {
                    byte: b'q',
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                },
                ActorEvents::Put(b'h'),
                ActorEvents::Put(b'e'),
                ActorEvents::Put(b'l'),
                ActorEvents::Put(b'l'),
                ActorEvents::Put(b'o'),
                ActorEvents::Unhook,
                ActorEvents::EscDispatch {
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                    byte: b'\\',
                }
            ]
        );
    }

    #[test]
    fn test_ommitted_dcs_param() {
        assert_eq!(
            parse("\x1bP;1q\x1b\\".as_bytes()),
            vec![
                ActorEvents::Hook {
                    byte: b'q',
                    params: vec![0, 1],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                },
                ActorEvents::Unhook,
                ActorEvents::EscDispatch {
                    params: vec![],
                    intermediates: vec![],
                    ignored_excess_intermediates: false,
                    byte: b'\\',
                }
            ]
        );
    }

    // Place tests under this line
}
