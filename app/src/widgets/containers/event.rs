use iced::Point;
use otty_containers::{Container, Discovery, EngineAddress, EngineInfo};
use otty_ui_kit::components::tag_search::TagSearchAction;

use super::child::actions::event::{ActionsEffect, ActionsIntent};
use super::child::create::event::{CreateEffect, CreateIntent};
use super::child::inspector::event::{InspectorEffect, InspectorIntent};
use super::child::run::event::{RunEffect, RunIntent};
use super::types::{
    InspectorTab, LoadedList, LoadedLists, Place, ResourceKind,
};

/// Intent events handled by the Containers widget.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ContainersIntent {
    /// What the open page sent: a press on a row, a header or a link, the
    /// search field and the header buttons. A position in a filtered table
    /// only means something on the page that drew it, and a search can move
    /// the place in the same update as a press, so `intent` is taken only
    /// while `page` is still open.
    FromPage {
        page: Place,
        intent: Box<ContainersIntent>,
    },
    /// Containers mode became active: search for engines, then load their
    /// lists, and ask again about the folders of an open Run form.
    Enter,
    /// Another mode became active: start no more engine work.
    Leave,
    /// The right panel went off the screen or came back. Only the app's update
    /// path sends this; see `update` in `update.rs`.
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
        engine: EngineAddress,
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
    OpenPortLink { row: usize, link: usize },
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
    /// An inspector tab was pressed.
    SelectInspectorTab(InspectorTab),
    // Live updates from engine events.
    /// An engine event changed this list of this engine (the events watch
    /// sends it at most once per 500 ms per list).
    ListChanged {
        engine: EngineAddress,
        resource: ResourceKind,
    },
    /// The events stream of this engine broke and is open again; changes in
    /// between were missed.
    EngineReconnected(EngineAddress),
    /// This engine stopped answering.
    EngineUnreachable(EngineAddress),
    /// One list of an engine was loaded again.
    ListLoaded {
        engine: EngineAddress,
        list: LoadedList,
    },
    /// A row was right-pressed: select it and open its menu at `position`
    /// (window coordinates).
    OpenRowMenu { index: usize, position: Point },
    /// The "⋯" button of the row at this position of the shown table was
    /// pressed: open that row's menu at `position` (window coordinates; the
    /// button's bottom-right corner) without selecting it, so the inspector
    /// keeps its row.
    RowMenuPressed { index: usize, position: Point },
    /// The create button above the table was pressed: the open list opens
    /// Pull image, Create volume, Create network or the Run container form.
    OpenCreate,
    /// An intent of the create dialogs.
    Create(CreateIntent),
    /// An intent of the Run container form.
    Run(RunIntent),
    /// An intent of the actions: the row menu, the confirmation dialog,
    /// Prune and the engine calls that run.
    Actions(ActionsIntent),
    /// An intent of the inspector: its followed logs, its inspect answer and
    /// the presses in the body of its open tab.
    Inspector(InspectorIntent),
}

/// Effect events produced by the Containers reducer. The app runs them off the
/// reducer, because they talk to the engines or the disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContainersEffect {
    /// Search for the local engines (`otty_containers::discover`).
    Discover,
    /// The same search behind the no-engine card, which stays on screen; its
    /// result comes as [`ContainersIntent::BackgroundSearchFinished`].
    DiscoverInBackground,
    /// Load the four lists of this engine.
    LoadLists(EngineInfo),
    /// Put this text on the clipboard.
    CopyToClipboard(String),
    /// Open this page in the default browser.
    OpenUrl(String),
    /// Load one list of this engine again. "Used by" of images, volumes and
    /// networks is read from `containers`; a container load does not use
    /// it.
    LoadList {
        info: EngineInfo,
        resource: ResourceKind,
        containers: Vec<Container>,
    },
    /// An effect of the create dialogs.
    Create(CreateEffect),
    /// An effect of the Run container form.
    Run(RunEffect),
    /// An effect of the actions.
    Actions(ActionsEffect),
    /// An effect of the inspector.
    Inspector(InspectorEffect),
    /// A row press selected a row: make the right panel visible, so the
    /// inspector shows it.
    RevealInspector,
    /// Tell the user about an error, such as a failed action, pull or run;
    /// carries the text in their language.
    Failed(String),
}

/// Containers event stream routed through the app update loop.
#[derive(Debug, Clone)]
pub(crate) enum ContainersEvent {
    /// Intent event reduced by the Containers widget.
    Intent(ContainersIntent),
    /// External effect orchestrated by app-level routing.
    Effect(ContainersEffect),
}
