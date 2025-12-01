use log::debug;

use crate::{Action, BlockEvent, EscapeActor};

/// Internal state for handling DCS passthrough sequences.
#[derive(Default)]
pub(crate) struct DcsState {
    buffer: Vec<u8>,
    overflow: bool,
}

impl DcsState {
    /// Reset DCS state and start a new payload with the given final byte.
    pub(crate) fn hook(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        debug!(
            "[dcs hook] params: {:?}, intermediates: {:?}, ignore: {:?}, final: {:?}",
            params, intermediates, ignored_excess_intermediates, byte
        );

        self.buffer.clear();
        self.overflow = false;
        self.buffer.push(byte);
    }

    /// Append a byte to the current DCS payload if within limits.
    pub(crate) fn put(&mut self, byte: u8) {
        if self.overflow {
            return;
        }

        if self.buffer.len() < crate::block::max_block_dcs_buffer_len() {
            self.buffer.push(byte);
        } else {
            self.overflow = true;
        }
    }

    /// Finalize the current DCS payload and emit any resulting actions.
    pub(crate) fn unhook<A: EscapeActor>(&mut self, actor: &mut A) {
        if self.overflow {
            debug!("[dcs unhook] payload exceeded buffer limit, ignoring");
        } else if let Some(event) =
            crate::block::parse_block_dcs(self.buffer.as_slice())
        {
            self.handle_block_event(actor, event);
        }

        self.buffer.clear();
        self.overflow = false;
    }

    fn handle_block_event<A: EscapeActor>(
        &mut self,
        actor: &mut A,
        event: BlockEvent,
    ) {
        actor.handle(Action::BlockEvent(event));
    }
}
