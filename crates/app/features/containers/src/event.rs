use std::path::PathBuf;

use iced::{Point, Size};
use otty_app_domain::mode::Mode;
use otty_containers::{
    Container, Discovery, EngineAddress, EngineInfo, RunRequest,
};
use otty_ui_kit::components::tag_search::TagSearchAction;

use super::actions::event::ActionsIntent;
use super::create::event::CreateIntent;
use super::inspector::event::InspectorIntent;
use super::inspector::types::{InspectTarget, LogTarget};
use super::run::event::RunIntent;
use super::types::{
    ActionRequest, InspectorTab, LoadedList, LoadedLists, Place, PullHandle,
    ResourceKind,
};

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
    /// lists, and ask again about the folders of an open Run form.
    Enter,
    /// Another mode became active: start no more engine work.
    Leave,
    /// The right panel went off the screen or came back. The binary sends it
    /// after every update in which that changed.
    RightPanelHidden(bool),
    /// The window got this size. The row menu opens at the pointer in window
    /// coordinates and is kept inside the window.
    WindowResized(Size),
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
    /// An inspector tab was pressed.
    SelectInspectorTab(InspectorTab),
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
    /// A row was right-pressed: select it and open its menu at `position`.
    OpenRowMenu {
        /// The position of the row in the shown table.
        index: usize,
        /// Where the menu opens, in window coordinates.
        position: Point,
    },
    /// The "⋯" button of the row at this position of the shown table was
    /// pressed: open that row's menu at `position` without selecting it, so
    /// the inspector keeps its row.
    RowMenuPressed {
        /// The position of the row in the shown table.
        index: usize,
        /// Where the menu opens, in window coordinates: the button's
        /// bottom-right corner.
        position: Point,
    },
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
    /// Tell the user about an error, such as a failed action, pull or run;
    /// the text is ready to show.
    Failed(String),
    /// A menu opened: take the keyboard from every widget, so typed keys
    /// reach no field or shell under it and Esc reaches the frame.
    TakeKeyboard,
    /// Run this action on its engine; answer
    /// [`ActionsIntent::Finished`] in [`ContainersIntent::Actions`].
    RunAction(ActionRequest),
    /// Open a terminal tab named `title` on this computer running `program`
    /// with `args`, a shell in a container on its own engine, then show
    /// Terminal mode.
    OpenShell {
        /// The title of the tab.
        title: String,
        /// The engine's command line tool, a full path.
        program: String,
        /// The arguments that open the shell on the chosen engine.
        args: Vec<String>,
    },
    /// Pull `image` at `tag` on `engine` for the pull dialog; answer with
    /// the pull's abort handle, its progress and its end in
    /// [`ContainersIntent::Create`].
    PullImage {
        /// The engine to pull on.
        engine: EngineInfo,
        /// The image name.
        image: String,
        /// The tag.
        tag: String,
    },
    /// Pull `image` at `tag` on `engine` for the Run container form; answer
    /// with the pull's abort handle, its progress and its end in
    /// [`ContainersIntent::Run`].
    PullRunImage {
        /// The engine to pull on.
        engine: EngineInfo,
        /// The image name.
        image: String,
        /// The tag.
        tag: String,
    },
    /// Stop a running pull through its abort handle.
    CancelPull(PullHandle),
    /// Ask `engine` what the image `reference` sets on its own; answer
    /// [`RunIntent::ImageDefaultsLoaded`] in [`ContainersIntent::Run`].
    LoadImageDefaults {
        /// The engine to ask.
        engine: EngineInfo,
        /// The image reference typed in the Run form.
        reference: String,
    },
    /// Create and start a container on `engine`; answer
    /// [`RunIntent::Finished`] in [`ContainersIntent::Run`].
    RunContainer {
        /// The engine to run on.
        engine: EngineInfo,
        /// What the Run form asks for.
        request: RunRequest,
    },
    /// Ask the disk whether this folder of the Run form exists; answer
    /// [`RunIntent::FolderChecked`] in [`ContainersIntent::Run`].
    CheckFolder(PathBuf),
    /// Wait for a pause in typing, then send
    /// [`RunIntent::FolderTypingPaused`] in [`ContainersIntent::Run`].
    WaitForTypingPause(PathBuf),
    /// Give the disk time to answer, then send
    /// [`RunIntent::FolderAnswerLate`] in [`ContainersIntent::Run`].
    WaitForLateAnswer(PathBuf),
    /// Load the inspect JSON of this row from this engine; answer
    /// [`InspectorIntent::InspectLoaded`] in [`ContainersIntent::Inspector`].
    LoadInspect {
        /// The engine to ask.
        engine: EngineInfo,
        /// The row to inspect.
        target: InspectTarget,
    },
    /// Ask what became of the container of these logs, after its engine
    /// reconnected; answer [`InspectorIntent::LogRunChecked`] in
    /// [`ContainersIntent::Inspector`].
    CheckLogRun(LogTarget),
}
