use gpui::{Pixels, Point, SharedString};

use crate::{BackendGeneration, BackendState};

/// Stable block identifier reported by the terminal protocol.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockId(SharedString);

impl BlockId {
    /// Borrow the protocol identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create an identifier from owned or shared text.
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for BlockId {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl From<String> for BlockId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Portion of a block copied by [`crate::Terminal::copy_block`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlockTextPart {
    /// Prompt/input line followed by command output.
    All,
    /// Full block contents.
    Content,
    /// Prompt/input line only.
    Prompt,
    /// Parsed command metadata only.
    Command,
}

/// Origin of text written to the native clipboard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CopySource {
    /// Current grid selection.
    Selection,
    /// A named block and requested part.
    Block {
        /// Copied block.
        block_id: BlockId,
        /// Copied portion.
        part: BlockTextPart,
    },
}

/// Semantic item under a context-menu or pointer request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HitTarget {
    /// Ordinary terminal grid content.
    Terminal,
    /// Clickable hyperlink URI.
    Link(SharedString),
    /// Terminal block metadata or content.
    Block(BlockId),
}

/// Coarse host-facing signals emitted by the terminal widget.
#[derive(Clone, Debug)]
pub enum TerminalEvent {
    /// Active backend lifecycle changed.
    BackendStateChanged {
        /// Backend generation owning this state.
        generation: BackendGeneration,
        /// New state.
        state: BackendState,
    },
    /// Window/tab title changed; `None` restores the host default.
    TitleChanged(Option<SharedString>),
    /// The terminal requested an audible alert.
    Bell,
    /// A link should be routed or opened by the host.
    OpenLinkRequested {
        /// Link URI.
        uri: SharedString,
    },
    /// The grid selection became empty or non-empty.
    SelectionChanged {
        /// Whether a non-empty selection exists.
        has_selection: bool,
    },
    /// The selected block changed.
    BlockSelectionChanged {
        /// Selected block, or `None` when cleared.
        block_id: Option<BlockId>,
    },
    /// Text was written to the native clipboard.
    Copied {
        /// Clipboard content origin.
        source: CopySource,
    },
    /// The host should present its terminal context menu.
    ContextMenuRequested {
        /// Position in window coordinates.
        position: Point<Pixels>,
        /// Semantic item under the pointer.
        target: HitTarget,
    },
}
