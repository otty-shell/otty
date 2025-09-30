use crate::actor::Actor;
use crate::types::{Action, State};
use crate::{transitions, utf8};

const MAX_INTERMEDIATES: usize = 2;
const MAX_OSC_PARAMS: usize = 16;
const MAX_OSC_RAW: usize = 1024;
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
    current_param: Option<CsiParam>,
    full: bool,
    idx: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            items: [CsiParam::default(); MAX_PARAMS],
            current_param: None,
            full: false,
            idx: 0,
        }
    }
}

impl Params {
    fn clear(&mut self) {
        self.current_param.take();
        self.full = false;
        self.idx = 0;
        self.items = [CsiParam::default(); MAX_PARAMS];
    }

    fn push_param(&mut self, param: CsiParam) {
        if self.idx >= MAX_PARAMS {
            self.full = true;
            return;
        }

        self.items[self.idx] = param;
        self.idx += 1;
    }

    fn finish_param(&mut self) {
        if let Some(val) = self.current_param.take() {
            if self.idx < MAX_PARAMS {
                self.items[self.idx] = val;
                self.idx += 1;
            }
        }
    }
}

#[derive(Debug, Default)]
struct OscState {
    buffer: Vec<u8>,
    params: [usize; MAX_OSC_PARAMS],
    num_params: usize,
    full: bool,
}

impl OscState {
    fn clear(&mut self) {
        self.buffer.clear();
        self.buffer.shrink_to_fit();
        self.num_params = 0;
        self.full = false;
        self.params = Default::default();
    }
}

#[derive(Debug, Default)]
struct Intermediates {
    items: [u8; MAX_INTERMEDIATES],
    idx: usize,
    ignore: bool,
}

impl Intermediates {
    fn clear(&mut self) {
        self.idx = 0;
        self.ignore = false;
        self.items = Default::default();
    }
}

#[derive(Debug, Default)]
pub struct Parser {
    state: State,
    intermediates: Intermediates,
    params: Params,
    osc: OscState,
    utf8_parser: utf8parse::Parser,
    utf8_return_state: State,
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
            
            self.utf8_return_state = self.state;
            self.state = next_state;
        }
    }

    fn perform<A: Actor>(&mut self, action: Action, byte: u8, actor: &mut A) {
        use Action::*;

        match action {
            Print => actor.print(byte as char),
            Execute => actor.execute(byte),
            Unhook => actor.unhook(),
            Put => actor.put(byte),
            CsiDispatch => actor.csi_dispatch(&[], false, byte),
            Param => self.handle_param_byte(byte),
            Clear => self.clear(),
            Collect => self.collect_intermediate(byte),
            Utf8 => self.advance_utf8(actor, byte),
            _ => {},
        }
    }

    // UTF-8 Parsing function (https://github.com/wezterm/wezterm/blob/main/vtparse/src/lib.rs#L669)
    fn advance_utf8<A: Actor>(&mut self, actor: &mut A, byte: u8) {
        let mut decoder = utf8::Decoder::new();

        self.utf8_parser.advance(&mut decoder, byte);

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
            let (next_state, action) =
                transitions::transit(self.utf8_return_state, byte);

            if action == Action::Execute
                || (next_state != self.utf8_return_state
                    && next_state != State::Utf8Sequence)
            {
                self.perform(
                    transitions::exit_action(self.utf8_return_state),
                    0,
                    actor,
                );
                self.perform(action, byte, actor);
                self.perform(transitions::entry_action(next_state), 0, actor);

                self.utf8_return_state = self.state;
                self.state = next_state;
                return;
            }
        }

        match self.utf8_return_state {
            State::Ground => actor.print(c),
            // State::OscString => self.osc.put(c),
            state => panic!("unreachable state {:?}", state),
        };

        self.state = self.utf8_return_state;
    }

    /// Promote early intermediates to parameters.
    /// This is handle sequences such as DECSET that use `?`
    /// prior to other numeric parameters.
    /// `?` is technically in the intermediate range and shouldn't
    /// appear in the parameter position according to ECMA 48
    fn promote_intermediates_to_params(&mut self) {
        if self.intermediates.idx > 0 {
            for &p in &self.intermediates.items[..self.intermediates.idx] {
                if self.params.idx >= MAX_PARAMS {
                    self.intermediates.ignore = true;
                    break;
                }
                self.params.items[self.params.idx] = CsiParam::P(p);
                self.params.idx += 1;
            }
            self.intermediates.idx = 0;
        }
    }

    fn handle_param_byte(&mut self, byte: u8) {
        if self.params.full {
            return;
        }

        self.promote_intermediates_to_params();

        if (b'0'..=b'9').contains(&byte) {
            let digit = (byte - b'0') as i64;
            match self.params.current_param.take() {
                Some(CsiParam::Integer(value)) => {
                    let updated = value.saturating_mul(10).saturating_add(digit);
                    self.params.current_param.replace(CsiParam::Integer(updated));
                }
                Some(_) => unreachable!(),
                None => {
                    self.params.current_param.replace(CsiParam::Integer(digit));
                }
            }
        } else {
            self.params.finish_param();
            self.params.push_param(CsiParam::P(byte));
        }
    }

    fn collect_intermediate(&mut self, byte: u8) {
        if self.intermediates.idx < MAX_INTERMEDIATES {
            self.intermediates.items[self.intermediates.idx] = byte;
            self.intermediates.idx += 1;
        } else {
            self.intermediates.ignore = true;
        }
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
            params: Vec<u8>,
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
            params: &[u8],
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
                    params: vec![],
                    parameters_truncated: false,
                    byte: b'm'
                },
                ActorEvents::Print('m'),
                ActorEvents::Print('y'),
                ActorEvents::CsiDispatch {
                    params: vec![],
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
}
