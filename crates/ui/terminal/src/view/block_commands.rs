//! Running the block commands a host queued in a terminal view (see
//! [`TerminalView::command`]): selecting, copying, folding, running a
//! block again, bringing it into view, and pasting.

use iced_core::clipboard::Kind as ClipboardKind;
use otty_libterm::surface::BlockKey;

use super::{TerminalView, TerminalViewState};
use crate::block_scroll::{ScrollPlace, place_showing_block};
use crate::paste::{paste_input, run_again_input};
use crate::term::{BlockCommand, Event};

impl<T: Clone + PartialEq> TerminalView<'_, T> {
    /// Run the block commands queued in `view_state`, oldest first.
    pub(super) fn process_pending_commands(
        &self,
        view_state: &mut TerminalViewState,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        while let Some(command) = view_state.next_block_command() {
            self.apply_block_command(command, view_state, clipboard, shell);
        }
    }

    fn apply_block_command(
        &self,
        command: BlockCommand,
        view_state: &mut TerminalViewState,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        if view.is_full_screen() {
            return;
        }

        match command {
            BlockCommand::Select(block_key) => {
                self.select_block(block_key, view_state, shell);
            },
            BlockCommand::SelectHovered => {
                if let Some(block_key) = view_state.hovered_block_key {
                    self.select_block(block_key, view_state, shell);
                }
            },
            BlockCommand::CopySelection => {
                let content = view.selectable_content();
                if !content.is_empty() {
                    clipboard.write(ClipboardKind::Standard, content);
                }
            },
            BlockCommand::ClearSelection => {
                self.input_manager
                    .clear_block_selection(view_state, &mut |event| {
                        shell.publish(event)
                    });
            },
            BlockCommand::CopyContent(block_key) => {
                let text = view.block_output_text(block_key);
                self.copy_block(block_key, text, view_state, clipboard, shell);
            },
            BlockCommand::CopyCommand(block_key) => {
                let text = view
                    .block(block_key)
                    .and_then(|block| block.meta.cmd.clone())
                    .filter(|cmd| !cmd.is_empty());
                self.copy_block(block_key, text, view_state, clipboard, shell);
            },
            BlockCommand::ScrollTo(block_key) => {
                self.bring_block_into_view(block_key, shell);
            },
            BlockCommand::ToggleFold(block_key) => {
                self.term.engine.toggle_block_fold(block_key);
            },
            BlockCommand::RunAgain(block_key) => {
                if let Some(command) = view.run_again_command(block_key) {
                    shell.publish(Event::Write {
                        id: self.term.id,
                        data: run_again_input(command, view.mode),
                    });
                }
            },
            BlockCommand::PasteClipboard => {
                if let Some(data) = clipboard.read(ClipboardKind::Standard)
                    && !data.is_empty()
                {
                    shell.publish(Event::Write {
                        id: self.term.id,
                        data: paste_input(&data, view.mode),
                    });
                }
            },
        }
    }

    /// Select the block `block_key` and tell the host, with a redraw.
    /// Nothing happens for a block that is not of a command or is selected
    /// already.
    fn select_block(
        &self,
        block_key: BlockKey,
        view_state: &mut TerminalViewState,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        let snapshot = self.term.engine.snapshot();
        let view = snapshot.view();
        let Some(block) = view.block(block_key) else {
            return;
        };

        // Only command blocks can be selected: not the prompt, and not the
        // block a terminal starts with.
        if !block.is_command() {
            return;
        }

        if view_state.selected_block_key == Some(block_key) {
            return;
        }

        view_state.selected_block_key = Some(block_key);
        shell.publish(Event::BlockSelected {
            id: self.term.id,
            block_key,
        });
        shell.publish(Event::Redraw { id: self.term.id });
    }

    /// Bring the block `block_key` into view: ask the host to scroll the
    /// history to the place that puts its start at the top edge of the
    /// pane, worked out from the snapshot shown now, or past the top of
    /// the history when its start shows only there. Nothing moves while
    /// the pane shows all of the block.
    fn bring_block_into_view(
        &self,
        block_key: BlockKey,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        let snapshot = self.term.engine.snapshot();
        let rows = self.term.engine.row_layout();
        let place = place_showing_block(&snapshot.view(), rows, block_key);

        match place {
            Some(ScrollPlace::Offset(offset)) => {
                shell.publish(Event::ScrollToOffset {
                    id: self.term.id,
                    offset,
                });
            },
            Some(ScrollPlace::PastTheTop) => {
                shell.publish(Event::ScrollPastTheTop { id: self.term.id });
            },
            None => {},
        }
    }

    /// Run a copy command of the block `block_key`: select the block and
    /// put `text` on the clipboard, and tell the host of both. Nothing is
    /// copied without a text; the selection still changes.
    fn copy_block(
        &self,
        block_key: BlockKey,
        text: Option<String>,
        view_state: &mut TerminalViewState,
        clipboard: &mut dyn iced_graphics::core::Clipboard,
        shell: &mut iced_graphics::core::Shell<'_, Event<T>>,
    ) {
        self.select_block(block_key, view_state, shell);

        if let Some(text) = text {
            clipboard.write(ClipboardKind::Standard, text);
            shell.publish(Event::BlockCopied {
                id: self.term.id,
                block_key,
            });
            shell.publish(Event::Redraw { id: self.term.id });
        }
    }
}
