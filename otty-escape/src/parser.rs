use crate::{
    NamedPrivateMode,
    actor::Actor,
    control, csi, esc, osc,
    sync::{SYNC_UPDATE_TIMEOUT, SyncHandler, Timeout},
};
use log::debug;
use otty_vte::{Actor as VTActor, CsiParam, Parser as VTParser};
use std::mem;

/// Maximum number of bytes read in one synchronized update (2MiB).
const SYNC_BUFFER_SIZE: usize = 0x20_0000;

/// Highest fill level allowed before the synchronization buffer is flushed.
const SYNC_BUFFER_LIMIT: usize = SYNC_BUFFER_SIZE - 1;

/// Number of bytes in the BSU/ESU CSI sequences.
const SYNC_ESCAPE_LEN: usize = 8;

/// BSU CSI sequence for beginning or extending synchronized updates.
const BSU_CSI: [u8; SYNC_ESCAPE_LEN] = *b"\x1b[?2026h";

/// ESU CSI sequence for terminating synchronized updates.
const ESU_CSI: [u8; SYNC_ESCAPE_LEN] = *b"\x1b[?2026l";

/// ESC byte introducer for CSI/OSC sequences.
const ESC_BYTE: u8 = 0x1B;

struct Performer<'a, A: Actor, T: Timeout> {
    actor: &'a mut A,
    state: &'a mut ParserState<T>,
}

impl<'a, A: Actor, T: Timeout> VTActor for Performer<'a, A, T> {
    fn print(&mut self, c: char) {
        self.actor.print(c);
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

    fn osc_dispatch(&mut self, params: &[&[u8]], byte: u8) {
        osc::perform(self.actor, params, byte);
    }

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        intermediates: &[u8],
        has_ignored_intermediates: bool,
        byte: u8,
    ) {
        csi::perform(
            self.actor,
            self.state,
            params,
            intermediates,
            has_ignored_intermediates,
            byte,
        )
    }

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        intermediates: &[u8],
        _ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        esc::perform(self.actor, intermediates, byte);
    }

    #[inline]
    fn terminated(&self) -> bool {
        self.state.terminated
    }
}

impl<'a, A: Actor, T: Timeout> Performer<'a, A, T> {
    #[must_use]
    fn new(state: &'a mut ParserState<T>, actor: &'a mut A) -> Self {
        Self { actor, state }
    }
}

#[derive(Default)]
pub(crate) struct ParserState<T: Timeout> {
    pub last_preceding_char: Option<char>,
    pub terminated: bool,
    /// Handler for synchronized updates.
    pub timeout: T,
    /// Bytes read during the synchronized update.
    buffer: Vec<u8>,
}

/// High-level escape sequence parser that forwards semantic events to an
/// [`Actor`](crate::actor::Actor).
#[derive(Default)]
pub struct Parser<T: Timeout = SyncHandler> {
    vt: VTParser,
    state: ParserState<T>,
}

impl<T: Timeout> Parser<T> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Synchronized update timeout.
    #[inline]
    pub fn sync_timeout(&self) -> &T {
        &self.state.timeout
    }

    /// End a synchronized update.
    pub fn stop_sync<A: Actor>(&mut self, handler: &mut A) {
        self.stop_sync_internal(handler, None);
    }

    /// Number of bytes in the synchronization buffer.
    #[inline]
    pub fn sync_bytes_count(&self) -> usize {
        self.state.buffer.len()
    }

    /// Advance the parser with a new chunk of bytes.
    pub fn advance<A: Actor>(&mut self, bytes: &[u8], actor: &mut A) {
        let mut processed = 0;

        while processed != bytes.len() {
            if self.state.timeout.pending_timeout() {
                processed += self.advance_sync(actor, &bytes[processed..]);
                continue;
            }

            let mut performer = Performer::new(&mut self.state, actor);
            processed += self
                .vt
                .advance_until_terminated(&bytes[processed..], &mut performer);
        }
    }

    /// End a synchronized update.
    ///
    /// The `bsu_offset` parameter should be passed if the sync buffer contains
    /// a new BSU escape that is not part of the current synchronized
    /// update.
    fn stop_sync_internal<A: Actor>(
        &mut self,
        actor: &mut A,
        bsu_offset: Option<usize>,
    ) {
        // Process all synchronized bytes.
        //
        // NOTE: We do not use `advance_until_terminated` here since BSU sequences are
        // processed automatically during the synchronized update.
        let buffer = mem::take(&mut self.state.buffer);
        let offset = bsu_offset.unwrap_or(buffer.len());

        {
            let mut performer = Performer::new(&mut self.state, actor);
            self.vt.advance(&buffer[..offset], &mut performer);
        }

        self.state.buffer = buffer;

        match bsu_offset {
            // Just clear processed bytes if there is a new BSU.
            //
            // NOTE: We do not need to re-process for a new ESU since the `advance_sync`
            // function checks for BSUs in reverse.
            Some(bsu_offset) => {
                let new_len = self.state.buffer.len() - bsu_offset;
                self.state.buffer.copy_within(bsu_offset.., 0);
                self.state.buffer.truncate(new_len);
            },
            // Report mode and clear state if no new BSU is present.
            None => {
                actor.unset_private_mode(NamedPrivateMode::SyncUpdate.into());
                self.state.timeout.clear_timeout();
                self.state.buffer.clear();
            },
        }
    }

    /// Process a new byte during a synchronized update.
    ///
    /// Returns the number of bytes processed.
    #[cold]
    fn advance_sync<A: Actor>(&mut self, actor: &mut A, bytes: &[u8]) -> usize {
        // Advance sync parser or stop sync if we'd exceed the maximum buffer size.
        let projected_len = self.state.buffer.len() + bytes.len();
        if projected_len >= SYNC_BUFFER_LIMIT {
            // Terminate the synchronized update.
            self.stop_sync_internal(actor, None);

            // Just parse the bytes normally.
            let mut performer = Performer {
                actor,
                state: &mut self.state,
            };
            self.vt.advance_until_terminated(bytes, &mut performer)
        } else {
            self.state.buffer.extend_from_slice(bytes);
            self.advance_sync_csi(actor, bytes.len());
            bytes.len()
        }
    }

    /// Handle BSU/ESU CSI sequences during synchronized update.
    fn advance_sync_csi<A: Actor>(&mut self, actor: &mut A, new_bytes: usize) {
        // Get constraints within which a new escape character might be relevant.
        let buffer_len = self.state.buffer.len();
        let start_offset =
            (buffer_len - new_bytes).saturating_sub(SYNC_ESCAPE_LEN - 1);
        let end_offset = buffer_len.saturating_sub(SYNC_ESCAPE_LEN - 1);

        let mut bsu_offset = None;
        let mut should_stop = false;

        {
            let search_buffer = &self.state.buffer[start_offset..end_offset];

            // Search for termination/extension escapes in the added bytes.
            //
            // NOTE: It is technically legal to specify multiple private modes in the same
            // escape, but we only allow EXACTLY `\e[?2026h`/`\e[?2026l` to keep the parser
            // more simple.
            for index in memchr::memchr_iter(ESC_BYTE, search_buffer).rev() {
                let offset = start_offset + index;
                let escape =
                    &self.state.buffer[offset..offset + SYNC_ESCAPE_LEN];

                if escape == BSU_CSI {
                    self.state.timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
                    bsu_offset = Some(offset);
                } else if escape == ESU_CSI {
                    should_stop = true;
                    break;
                }
            }
        }

        if should_stop {
            self.stop_sync_internal(actor, bsu_offset);
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
