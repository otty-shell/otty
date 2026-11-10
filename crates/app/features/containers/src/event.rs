use otty_app_domain::mode::Mode;
use otty_containers::{Container, Discovery, EngineAddress, EngineInfo};
use otty_ui_kit::components::tag_search::TagSearchAction;

use super::types::{LoadedList, LoadedLists, Place, ResourceKind};

/// What the containers feature is asked to do: a press in its views, a tick
/// of its timer, an engine event, or the answer of the engine work the
/// binary ran for it.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainersIntent {
    /// What the open page sent: a press on a row, a header or a link, the
    /// search field and the header buttons. A position in a filtered table
    /// only means something on the page that drew it, and a search can move
    /// the place in the same update as a press, so `intent` is taken only
    /// while `page` is still open.
    FromPage {
        /// The page that drew what was pressed.
        page: Place,
        /// What was pressed.
        intent: Box<ContainersIntent>,
    },
    /// The active mode changed: Containers mode enters, any other mode
    /// leaves it.
    ModeChanged(Mode),
    /// Containers mode became active: search for engines, then load their
    /// lists.
    Enter,
    /// Another mode became active: start no more engine work.
    Leave,
    /// The right panel went off the screen or came back. The binary sends it
    /// after every update in which that changed.
    RightPanelHidden(bool),
    /// The engine search finished.
    DiscoveryFinished(Discovery),
    /// The timer of the no-engine card ticked: look for engines behind the
    /// card, unless a search is on its way.
    EngineSearchTick,
    /// The search behind the no-engine card finished.
    BackgroundSearchFinished(Discovery),
    /// One load of an engine's four lists finished.
    ListsLoaded {
        /// The engine the lists belong to.
        engine: EngineAddress,
        /// Its four lists, each loaded or failed on its own.
        lists: LoadedLists,
    },
    /// A list row of the tree was pressed: show this list of this engine.
    Open(Place),
    /// An engine root of the tree was pressed: open or close it. What the
    /// main area shows does not change.
    ToggleEngine(EngineAddress),
    /// The Compose link of the row at this position of the shown (filtered)
    /// containers table was pressed: add the `compose` tag of that project.
    OpenComposeProject(usize),
    /// A port link of the container at this position of the shown (filtered)
    /// containers table was pressed: open the page of that link in the
    /// browser. `link` counts the port links of the row from 0. The row is
    /// not selected.
    OpenPortLink {
        /// The position of the row in the shown table.
        row: usize,
        /// The position of the link among the port links of the row.
        link: usize,
    },
    /// The title of the sortable column at this position of the open table
    /// was pressed.
    SortBy(usize),
    /// The row at this position of the shown (filtered) table was pressed.
    SelectRow(usize),
    /// The user did something in the tag search of the open table.
    TagSearch(TagSearchAction),
    /// Load every list of every engine again (the Refresh button).
    Refresh,
    /// Search for engines again (the Check again button).
    CheckAgain,
    /// Copy this command to the clipboard (the Copy button).
    CopyCommand(String),
    /// Open this page in the browser (the Install and How to fix buttons).
    OpenLink(String),
    /// The browser was asked to open a page; the error is why it could not
    /// be started.
    LinkOpened(Result<(), String>),
    /// An engine event changed this list of this engine (the events watch
    /// sends it at most once per 500 ms per list).
    ListChanged {
        /// The engine whose list changed.
        engine: EngineAddress,
        /// The list that changed.
        resource: ResourceKind,
    },
    /// The events stream of this engine broke and is open again; changes in
    /// between were missed.
    EngineReconnected(EngineAddress),
    /// This engine stopped answering.
    EngineUnreachable(EngineAddress),
    /// One list of an engine was loaded again.
    ListLoaded {
        /// The engine the list belongs to.
        engine: EngineAddress,
        /// The list, loaded or failed.
        list: LoadedList,
    },
}

/// What the containers feature reports after an intent: the engine work the
/// binary runs for it, and the news for the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainersOutcome {
    /// Search for the local engines; answer
    /// [`ContainersIntent::DiscoveryFinished`].
    Discover,
    /// The same search behind the no-engine card, which stays on screen;
    /// answer [`ContainersIntent::BackgroundSearchFinished`].
    DiscoverInBackground,
    /// Load the four lists of this engine; answer
    /// [`ContainersIntent::ListsLoaded`].
    LoadLists(EngineInfo),
    /// Load one list of this engine again; answer
    /// [`ContainersIntent::ListLoaded`]. "Used by" of images, volumes and
    /// networks is read from `containers`; a container load does not use it.
    LoadList {
        /// The engine to ask.
        info: EngineInfo,
        /// The list to load.
        resource: ResourceKind,
        /// The containers shown now, for "used by".
        containers: Vec<Container>,
    },
    /// Put this text on the clipboard.
    CopyToClipboard(String),
    /// Open this page in the default browser.
    OpenUrl(String),
    /// A row press selected a row: make the right panel visible, so the
    /// inspector shows it.
    RevealInspector,
    /// Tell the user about an error; the text is ready to show.
    Failed(String),
}
