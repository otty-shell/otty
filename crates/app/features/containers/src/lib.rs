//! Containers mode of the OTTY app: the engines found on this computer in the
//! left panel, the containers, images, volumes and networks of each engine in
//! the main area, with the sort and the tag search of each page, the cards
//! shown while no engine is usable, the Run container form, the inspector of
//! the selected row in the right panel, the row menu, the confirmation and
//! create dialogs, and the left side of the status bar.
//!
//! The feature finds no engine and calls none itself: it has the data types
//! of `otty-containers` only. It reports the engine and disk work it needs
//! as a [`ContainersOutcome`]; the binary runs it and gives the answer back
//! as a [`ContainersIntent`]. What to watch is decided here: the binary
//! watches the events of [`Containers::watched_engines`], follows the logs
//! of [`Containers::followed_logs`] and runs the timers of
//! [`Containers::engine_search_timer`] and
//! [`Containers::folder_recheck_timer`].
//!
//! The inspector, the Run form, the create dialogs and the actions are inner
//! modules: each reduces its own intents and reports outcomes that the
//! parent reducer turns into [`ContainersOutcome`]s in the same update.
//!
//! # Example
//!
//! Entering Containers mode looks for engines; a found engine has its lists
//! loaded and its events watched.
//!
//! ```
//! use otty_app_containers::{
//!     Containers, ContainersIntent, ContainersOutcome,
//! };
//! use otty_app_slots::Feature;
//! use otty_containers::{EngineKind, fixtures};
//!
//! let mut containers = Containers::new();
//!
//! let entered = containers.reduce(ContainersIntent::Enter, ());
//! assert_eq!(entered, vec![ContainersOutcome::Discover]);
//!
//! // The binary searched and gives the result back.
//! let docker = fixtures::engine(EngineKind::Docker);
//! let found = containers.reduce(
//!     ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
//!         EngineKind::Docker,
//!     ])),
//!     (),
//! );
//! assert_eq!(found, vec![ContainersOutcome::LoadLists(docker.clone())]);
//! assert_eq!(containers.watched_engines(), vec![&docker]);
//! ```

mod actions;
mod create;
mod event;
mod format;
mod inspector;
mod links;
mod model;
mod no_engine;
mod overview;
mod reducer;
mod resource_rows;
mod run;
mod sort;
mod state;
mod table_search;
mod types;
mod view;

use std::time::Duration;

use actions::view::{confirm_dialog, row_menu};
use actions::{ConfirmDialogViewModel, RowMenuViewModel};
use create::FormDialogViewModel;
use create::view::form_dialog;
pub use event::{ContainersIntent, ContainersOutcome};
use iced::Element;
use model::{
    EngineTreeViewModel, InspectorViewModel, MainAreaViewModel,
    StatusBarViewModel,
};
use otty_app_slots::{
    Feature, MainArea, MainContent, Overlay, Overlays, RightPanel, StatusBar,
};
use otty_app_theme::theme::ThemeProps;
use otty_containers::EngineInfo;
use otty_ui_kit::components::status_bar::StatusItem;
use run::view::{run_form, run_panel};
use run::{RunFormViewModel, RunPanelViewModel};
use state::ContainersState;
use view::{engine_tree, inspector as inspector_view, main_area, status_bar};

pub use self::actions::event::ActionsIntent;
pub use self::actions::types::RowAction;
pub use self::create::event::CreateIntent;
pub use self::create::types::FormField;
pub use self::inspector::event::InspectorIntent;
pub use self::inspector::types::{InspectTarget, LogRead, LogTarget};
pub use self::inspector::{LOG_TAIL, MAX_LOG_LINES};
pub use self::links::LinkTarget;
pub use self::run::event::RunIntent;
pub use self::run::types::RunEdit;
pub use self::types::{
    ActionRequest, ContainersError, EngineAction, InspectorTab, LoadedList,
    LoadedLists, NoEngineCase, Place, PullHandle, ResourceKind,
};

/// How often the engines are looked for while the no-engine card shows.
const ENGINE_SEARCH_PERIOD: Duration = Duration::from_secs(5);

/// How often the folders of the Run form are asked about again.
const FOLDER_RECHECK_PERIOD: Duration = Duration::from_secs(2);

/// The containers feature: the engines found on this computer, their
/// containers, images, volumes and networks, the open list, the selected
/// rows and the tag search of each page (spec Decisions 5, 8, 10 and 21). It
/// asks for engine work only while Containers mode is active.
///
/// # Example
///
/// ```
/// use otty_app_containers::Containers;
///
/// let containers = Containers::new();
///
/// assert!(!containers.is_active());
/// assert!(containers.watched_engines().is_empty());
/// ```
#[derive(Debug, Default)]
pub struct Containers {
    state: ContainersState,
}

impl Containers {
    /// Return whether Containers mode is the active mode.
    pub fn is_active(&self) -> bool {
        self.state.is_active()
    }

    /// Return whether the feature counts the right panel as off the screen.
    /// The binary compares it with the frame after every update and sends
    /// [`ContainersIntent::RightPanelHidden`] when they differ.
    pub fn is_right_panel_hidden(&self) -> bool {
        self.state.is_right_panel_hidden()
    }

    /// Return how often to tick [`ContainersIntent::EngineSearchTick`]:
    /// while the no-engine card is on screen in active Containers mode, so an
    /// engine that starts takes over by itself. `None` otherwise.
    ///
    /// The binary runs the timer: it needs the async runtime of the binary,
    /// which a feature crate does not take.
    pub fn engine_search_timer(&self) -> Option<Duration> {
        if !self.state.is_active() || !self.state.shows_no_engine_card() {
            return None;
        }

        Some(ENGINE_SEARCH_PERIOD)
    }

    /// Return the engines whose events to watch: every reached engine while
    /// Containers mode is active, nothing in another mode (spec Decision
    /// 14).
    ///
    /// The binary runs one watch per engine, keyed by what the engine said
    /// about itself, so a watch keeps running across searches that find the
    /// engine again and stops when a search no longer finds it. Its events
    /// come back as [`ContainersIntent::ListChanged`],
    /// [`ContainersIntent::EngineReconnected`] and
    /// [`ContainersIntent::EngineUnreachable`].
    pub fn watched_engines(&self) -> Vec<&EngineInfo> {
        if !self.state.is_active() {
            return Vec::new();
        }

        self.state
            .engines()
            .iter()
            .map(|engine| engine.info())
            .collect()
    }

    /// Return how often to send [`RunIntent::RecheckFolders`] in
    /// [`ContainersIntent::Run`]: while Containers mode is active and the
    /// open Run form has a folder row, so a folder created elsewhere clears
    /// its error by itself. `None` otherwise.
    pub fn folder_recheck_timer(&self) -> Option<Duration> {
        if !self.state.is_active() || !self.state.run().has_folder_rows() {
            return None;
        }

        Some(FOLDER_RECHECK_PERIOD)
    }

    /// Return the logs to follow: the selected container's while the Logs
    /// tab of the inspector is on screen in Containers mode; `None`
    /// otherwise (spec Decision 12).
    ///
    /// The binary runs one logs stream keyed by the target, so another
    /// container, another engine or the container starting again stops the
    /// stream and starts a new one. Its lines come back as
    /// [`InspectorIntent`]s in [`ContainersIntent::Inspector`].
    pub fn followed_logs(&self) -> Option<&LogTarget> {
        self.state.inspector().logs().map(|logs| logs.target())
    }

    /// Return whether a create dialog, such as Pull image, is open. It is a
    /// dialog over the window.
    pub fn has_create_dialog(&self) -> bool {
        self.state.create().is_open()
    }

    /// Return whether a destructive action waits for the user to confirm
    /// it. The confirmation is a dialog over the window.
    pub fn has_confirm(&self) -> bool {
        self.state.actions().confirm().is_some()
    }

    /// Return whether the menu of a row is open. It is a menu over the
    /// window.
    pub fn has_row_menu(&self) -> bool {
        self.state.actions().row_menu().is_some()
    }

    /// Create the feature before any engine search.
    pub fn new() -> Self {
        Self::default()
    }

    /// The engine tree that fills the left panel of Containers mode: its rows
    /// and the short text under them while no engine is usable.
    ///
    /// It is a method of its own and not [`otty_app_slots::LeftPanel`]: that
    /// trait draws one of the stacked sections of Terminal mode, with a
    /// header the frame opens and closes and a height it shares. The tree
    /// fills the whole panel and has no part the frame owns.
    pub fn engine_tree<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, ContainersIntent> {
        engine_tree::view(engine_tree::EngineTreeProps {
            vm: EngineTreeViewModel::from_state(&self.state),
            theme,
        })
    }
}

impl Feature for Containers {
    type Intent = ContainersIntent;
    type Outcome = ContainersOutcome;
    type Ctx<'a> = ();

    /// Reduce an intent into state changes and outcomes. The clock of the
    /// tag search moves after the intent, so the rows the next view shows
    /// and the rows the next press is looked up in are counted at one time.
    fn reduce(
        &mut self,
        intent: ContainersIntent,
        _ctx: (),
    ) -> Vec<ContainersOutcome> {
        let outcomes = reducer::reduce(&mut self.state, intent);

        self.state.set_age_clock(format::unix_now());

        outcomes
    }
}

/// Which screen fills the main area of Containers mode. The table of an
/// open list is no screen: it scrolls on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainersScreen {
    /// The card while the engine search runs.
    Searching,
    /// The no-engine card of one diagnosis.
    NoEngine(NoEngineCase),
    /// The Run container form, in place of the table.
    RunForm,
}

impl MainArea for Containers {
    type Screen = ContainersScreen;

    /// The Run container form while it is open, else the open list of an
    /// engine or a card while there is none; ages are counted up to the
    /// current time.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, ContainersIntent, ContainersScreen> {
        let run_ctx = self.state.run_ctx();
        if let Some(vm) =
            RunFormViewModel::from_state(self.state.run(), &run_ctx)
        {
            let content = run_form::view(run_form::RunFormProps { vm, theme })
                .map(ContainersIntent::Run);

            return MainContent::Screen {
                id: ContainersScreen::RunForm,
                content,
            };
        }

        let vm = MainAreaViewModel::from_state(&self.state, format::unix_now());
        let id = match &vm {
            MainAreaViewModel::Table(_) => None,
            MainAreaViewModel::Searching => Some(ContainersScreen::Searching),
            MainAreaViewModel::NoEngine(card) => {
                Some(ContainersScreen::NoEngine(card.case))
            },
        };

        let content = main_area::view(main_area::MainAreaProps { vm, theme });

        match id {
            Some(id) => MainContent::Screen { id, content },
            None => MainContent::ScrollsOnItsOwn(content),
        }
    }
}

impl RightPanel for Containers {
    /// What the image sets while the Run form is open, else the inspector of
    /// the selected row: its header with the action buttons, its tabs and
    /// the open tab.
    fn right_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, ContainersIntent> {
        let run_ctx = self.state.run_ctx();
        if let Some(vm) =
            RunPanelViewModel::from_state(self.state.run(), &run_ctx)
        {
            return run_panel::view(run_panel::RunPanelProps { vm, theme })
                .map(ContainersIntent::Run);
        }

        let actions_ctx = self.state.actions_ctx();
        inspector_view::view(inspector_view::InspectorProps {
            vm: InspectorViewModel::from_state(&self.state),
            tabs: inspector::InspectorTabsViewModel::from_state(
                self.state.inspector(),
                &self.state.inspector_ctx(),
            ),
            actions: actions::ActionsViewModel::from_state(
                self.state.actions(),
                &actions_ctx,
            ),
            theme,
        })
    }
}

impl Overlays for Containers {
    /// The overlay on top: the create dialog, then the confirmation dialog,
    /// then the row menu. The row menu opens at the pointer in window
    /// coordinates and is kept inside the window, so every overlay lies over
    /// the whole window.
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, ContainersIntent>> {
        if let Some(vm) = FormDialogViewModel::from_state(self.state.create()) {
            let dialog =
                form_dialog::view(form_dialog::FormDialogProps { vm, theme })
                    .map(ContainersIntent::Create);

            return Some(Overlay::dialog(dialog));
        }

        if let Some(vm) =
            ConfirmDialogViewModel::from_state(self.state.actions())
        {
            let dialog =
                confirm_dialog::view(confirm_dialog::ConfirmDialogProps {
                    vm,
                    theme,
                })
                .map(ContainersIntent::Actions);

            return Some(Overlay::dialog(dialog));
        }

        let actions_ctx = self.state.actions_ctx();
        let vm =
            RowMenuViewModel::from_state(self.state.actions(), &actions_ctx)?;
        let menu = row_menu::view(row_menu::RowMenuProps {
            vm,
            area: self.state.window_size(),
            theme,
        })
        .map(ContainersIntent::Actions);

        Some(Overlay::menu(menu))
    }
}

impl StatusBar for Containers {
    /// The engine on screen and the counts of the open list, or the mode
    /// and the reason there is no engine.
    fn status_bar(&self) -> Vec<StatusItem<ContainersIntent>> {
        status_bar::items(&StatusBarViewModel::from_state(&self.state))
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_slots::OverlayKind;
    use otty_app_theme::theme::ThemeManager;
    use otty_containers::{CandidateSource, EngineKind, fixtures};

    use super::*;

    /// Containers mode active with the fixture engines of `kinds` found.
    fn found(kinds: &[EngineKind]) -> Containers {
        let mut containers = Containers::new();
        let _ = containers.reduce(ContainersIntent::Enter, ());
        let _ = containers.reduce(
            ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
            (),
        );

        containers
    }

    /// Containers mode active with the fixture Docker engine and its lists.
    fn loaded() -> Containers {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut containers = found(&[EngineKind::Docker]);
        let _ = containers.reduce(
            ContainersIntent::ListsLoaded {
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
            (),
        );

        containers
    }

    #[test]
    fn each_intent_moves_the_clock_of_the_tag_search() {
        let mut containers = Containers::new();
        assert_eq!(containers.state.age_clock(), 0);

        let _ = containers.reduce(ContainersIntent::Leave, ());

        assert!(containers.state.age_clock() > 0);
    }

    #[test]
    fn containers_mode_watches_every_reached_engine() {
        let one = found(&[EngineKind::Docker]);
        let two = found(&[EngineKind::Docker, EngineKind::Podman]);

        assert_eq!(one.watched_engines().len(), 1);
        assert_eq!(two.watched_engines().len(), 2);
    }

    #[test]
    fn nothing_is_watched_without_an_engine() {
        let containers = found(&[]);

        assert!(containers.watched_engines().is_empty());
    }

    #[test]
    fn engines_are_watched_only_in_containers_mode() {
        let mut containers = Containers::new();
        assert!(containers.watched_engines().is_empty());

        let _ = containers.reduce(ContainersIntent::Enter, ());
        let _ = containers.reduce(
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
                EngineKind::Podman,
            ])),
            (),
        );
        assert_eq!(containers.watched_engines().len(), 2);

        let _ = containers.reduce(ContainersIntent::Leave, ());
        assert!(containers.watched_engines().is_empty());
    }

    #[test]
    fn the_engine_timer_runs_while_the_card_shows() {
        let containers = found(&[]);

        assert_eq!(
            containers.engine_search_timer(),
            Some(ENGINE_SEARCH_PERIOD)
        );
    }

    #[test]
    fn the_engine_timer_runs_for_every_card() {
        for discovery in [
            fixtures::discovery_with_cli(&[], Some("/bin/docker"), None),
            fixtures::discovery_with_cli(&[], None, Some("/bin/podman")),
            fixtures::discovery_with_cli(
                &[],
                Some("/bin/docker"),
                Some("/bin/podman"),
            ),
            fixtures::denied_discovery(
                fixtures::DOCKER_SOCKET,
                CandidateSource::Docker,
            ),
        ] {
            let mut containers = found(&[]);

            let _ = containers
                .reduce(ContainersIntent::DiscoveryFinished(discovery), ());

            assert!(containers.engine_search_timer().is_some());
        }
    }

    #[test]
    fn the_engine_timer_does_not_run_while_engines_are_shown() {
        let containers = found(&[EngineKind::Docker]);

        assert_eq!(containers.engine_search_timer(), None);
    }

    #[test]
    fn the_engine_timer_does_not_run_before_the_first_search_finishes() {
        let mut containers = Containers::new();
        let _ = containers.reduce(ContainersIntent::Enter, ());

        assert_eq!(containers.engine_search_timer(), None);
    }

    #[test]
    fn the_engine_timer_stops_when_containers_mode_is_left() {
        let mut containers = found(&[]);

        let _ = containers.reduce(ContainersIntent::Leave, ());

        assert_eq!(containers.engine_search_timer(), None);
    }

    #[test]
    fn the_engine_timer_stops_as_soon_as_the_card_goes() {
        let mut containers = found(&[]);

        let _ = containers.reduce(
            ContainersIntent::BackgroundSearchFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
            (),
        );

        assert_eq!(containers.engine_search_timer(), None);
    }

    /// While Check again runs, the card is replaced by "searching".
    #[test]
    fn the_engine_timer_waits_while_check_again_runs() {
        let mut containers = found(&[]);

        let _ = containers.reduce(ContainersIntent::CheckAgain, ());

        assert_eq!(containers.engine_search_timer(), None);
    }

    #[test]
    fn the_engine_tree_lists_the_engines_and_a_press_opens_a_list() {
        let themes = ThemeManager::new();
        let containers = loaded();
        let mut ui = simulator(
            containers.engine_tree(ThemeProps::new(themes.current())),
        );

        ui.click(otty_app_i18n::t(otty_app_i18n::Key::ContainersImages))
            .expect("the tree should list the images");

        let intents: Vec<ContainersIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [ContainersIntent::Open(place)]
                if place.resource() == ResourceKind::Images
        ));
    }

    #[test]
    fn the_main_area_is_a_table_that_scrolls_on_its_own() {
        let themes = ThemeManager::new();
        let containers = loaded();

        let content = containers.main_area(ThemeProps::new(themes.current()));

        assert!(matches!(content, MainContent::ScrollsOnItsOwn(_)));
    }

    #[test]
    fn the_cards_are_screens_named_by_what_they_show() {
        let themes = ThemeManager::new();
        let mut containers = Containers::new();
        let _ = containers.reduce(ContainersIntent::Enter, ());
        let searching = containers.main_area(ThemeProps::new(themes.current()));
        let searching = matches!(
            searching,
            MainContent::Screen {
                id: ContainersScreen::Searching,
                ..
            }
        );

        let card = found(&[]);
        let card = card.main_area(ThemeProps::new(themes.current()));

        assert!(searching);
        assert!(matches!(
            card,
            MainContent::Screen {
                id: ContainersScreen::NoEngine(NoEngineCase::NotInstalled),
                ..
            }
        ));
    }

    #[test]
    fn a_row_press_in_the_main_area_comes_from_its_page() {
        let themes = ThemeManager::new();
        let containers = loaded();
        let content = containers
            .main_area(ThemeProps::new(themes.current()))
            .into_element(ThemeProps::new(themes.current()));
        let mut ui = simulator(content);

        ui.click("otty-fx-exit3")
            .expect("the table should list containers");

        let intents: Vec<ContainersIntent> = ui.into_messages().collect();
        assert!(matches!(
            intents.as_slice(),
            [ContainersIntent::FromPage { intent, .. }]
                if matches!(**intent, ContainersIntent::SelectRow(_))
        ));
    }

    #[test]
    fn the_status_bar_names_the_engine_and_counts_the_open_list() {
        let containers = loaded();

        let items = containers.status_bar();

        assert!(matches!(
            items.as_slice(),
            [
                StatusItem::Target(_, engine, true),
                StatusItem::Text(counts),
            ] if engine == "Docker 29.9.1" && counts.contains("containers")
        ));
    }

    /// Reduce each of `intents` in order.
    fn send(containers: &mut Containers, intents: Vec<ContainersIntent>) {
        for intent in intents {
            let _ = containers.reduce(intent, ());
        }
    }

    /// The fixture lists with the Logs tab of the first container open.
    fn on_logs() -> Containers {
        let mut containers = loaded();
        send(
            &mut containers,
            vec![ContainersIntent::SelectInspectorTab(InspectorTab::Logs)],
        );

        containers
    }

    /// The fixture lists with the Run form open and one volume row from
    /// `source`.
    fn form_with_source(source: &str) -> Containers {
        let mut containers = loaded();
        send(
            &mut containers,
            vec![
                ContainersIntent::OpenCreate,
                ContainersIntent::Run(RunIntent::Edited(RunEdit::AddVolume)),
                ContainersIntent::Run(RunIntent::Edited(
                    RunEdit::VolumeSource(0, source.to_string()),
                )),
            ],
        );

        containers
    }

    #[test]
    fn the_logs_are_followed_only_on_the_logs_tab_of_a_container() {
        let on_overview = loaded();
        let on_logs = on_logs();

        assert_eq!(on_overview.followed_logs(), None);
        assert!(on_logs.followed_logs().is_some());
    }

    #[test]
    fn the_logs_stop_when_another_tab_opens() {
        let mut containers = on_logs();

        send(
            &mut containers,
            vec![ContainersIntent::SelectInspectorTab(InspectorTab::Inspect)],
        );

        assert_eq!(containers.followed_logs(), None);
    }

    #[test]
    fn the_logs_follow_another_container_when_the_selection_changes() {
        let mut containers = on_logs();
        let first = containers.followed_logs().cloned();

        send(&mut containers, vec![ContainersIntent::SelectRow(0)]);
        let second = containers.followed_logs().cloned();
        send(&mut containers, vec![ContainersIntent::SelectRow(0)]);

        assert!(first.is_some());
        assert!(second.is_some());
        assert_ne!(first, second);
        assert_eq!(containers.followed_logs().cloned(), second);
    }

    /// A finished stream keeps its place in iced as long as its key stays,
    /// so only a new key can start the logs again after a restart.
    #[test]
    fn the_logs_follow_a_new_stream_when_the_container_starts_again() {
        let mut containers = on_logs();
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let first = containers.followed_logs().cloned().expect("logs");
        let other = resources
            .containers()
            .iter()
            .map(|container| container.id())
            .find(|id| *id != first.container_id())
            .expect("another container");

        send(
            &mut containers,
            vec![ContainersIntent::Inspector(
                InspectorIntent::ContainerStarted {
                    engine: docker.address().clone(),
                    id: other.to_string(),
                },
            )],
        );
        let after_other = containers.followed_logs().cloned();
        send(
            &mut containers,
            vec![ContainersIntent::Inspector(
                InspectorIntent::ContainerStarted {
                    engine: docker.address().clone(),
                    id: first.container_id().to_string(),
                },
            )],
        );
        let after_own = containers.followed_logs().cloned().expect("logs");

        assert_eq!(after_other, Some(first.clone()));
        assert_eq!(after_own.container_id(), first.container_id());
        assert_ne!(after_own, first);
    }

    #[test]
    fn the_logs_stop_when_containers_mode_is_left() {
        let mut containers = on_logs();

        send(&mut containers, vec![ContainersIntent::Leave]);

        assert_eq!(containers.followed_logs(), None);
    }

    #[test]
    fn the_logs_stop_while_the_run_form_is_open() {
        let mut containers = on_logs();

        send(&mut containers, vec![ContainersIntent::OpenCreate]);
        let while_open = containers.followed_logs().cloned();
        send(
            &mut containers,
            vec![ContainersIntent::Run(RunIntent::Close)],
        );

        assert_eq!(while_open, None);
        assert!(containers.followed_logs().is_some());
    }

    #[test]
    fn the_logs_stop_while_the_right_panel_is_hidden() {
        let mut containers = on_logs();

        send(
            &mut containers,
            vec![ContainersIntent::RightPanelHidden(true)],
        );
        let while_hidden = containers.followed_logs().cloned();
        send(
            &mut containers,
            vec![ContainersIntent::RightPanelHidden(false)],
        );

        assert_eq!(while_hidden, None);
        assert!(containers.followed_logs().is_some());
    }

    #[test]
    fn the_folder_timer_runs_with_a_folder_row_in_containers_mode() {
        let containers = form_with_source("/srv/site");

        assert_eq!(
            containers.folder_recheck_timer(),
            Some(FOLDER_RECHECK_PERIOD)
        );
    }

    #[test]
    fn the_folder_timer_needs_an_open_form() {
        let mut containers = form_with_source("/srv/site");

        send(
            &mut containers,
            vec![ContainersIntent::Run(RunIntent::Close)],
        );

        assert_eq!(containers.folder_recheck_timer(), None);
    }

    #[test]
    fn the_folder_timer_needs_a_folder_row() {
        let mut no_rows = loaded();
        send(&mut no_rows, vec![ContainersIntent::OpenCreate]);

        assert_eq!(no_rows.folder_recheck_timer(), None);
        assert_eq!(form_with_source("otty-data").folder_recheck_timer(), None);
        assert_eq!(form_with_source("  ").folder_recheck_timer(), None);
    }

    #[test]
    fn the_folder_timer_stops_when_containers_mode_is_left() {
        let mut containers = form_with_source("/srv/site");

        send(&mut containers, vec![ContainersIntent::Leave]);

        assert!(containers.state.run().is_open());
        assert_eq!(containers.folder_recheck_timer(), None);
    }

    #[test]
    fn the_run_form_fills_the_main_area_as_its_own_screen() {
        let themes = ThemeManager::new();
        let mut containers = loaded();
        send(&mut containers, vec![ContainersIntent::OpenCreate]);

        let content = containers.main_area(ThemeProps::new(themes.current()));

        assert!(matches!(
            content,
            MainContent::Screen {
                id: ContainersScreen::RunForm,
                ..
            }
        ));
    }

    #[test]
    fn the_right_panel_shows_the_inspector_of_the_selected_row() {
        let themes = ThemeManager::new();
        let containers = loaded();
        let mut ui = simulator(
            containers.right_panel(ThemeProps::new(themes.current())),
        );

        ui.click(otty_app_i18n::t(otty_app_i18n::Key::ContainersTabLogs))
            .expect("the inspector should show its tabs");

        let intents: Vec<ContainersIntent> = ui.into_messages().collect();
        assert_eq!(
            intents,
            vec![ContainersIntent::SelectInspectorTab(InspectorTab::Logs)]
        );
    }

    #[test]
    fn while_the_run_form_is_open_the_right_panel_shows_the_image_panel() {
        let themes = ThemeManager::new();
        let mut containers = loaded();
        send(&mut containers, vec![ContainersIntent::OpenCreate]);
        let mut ui = simulator(
            containers.right_panel(ThemeProps::new(themes.current())),
        );

        assert!(
            ui.find(otty_app_i18n::t(otty_app_i18n::Key::ContainersRunNoImage))
                .is_ok()
        );
    }

    /// The kind of the overlay on top, if one is open.
    fn top_kind(containers: &Containers) -> Option<OverlayKind> {
        let themes = ThemeManager::new();

        containers
            .overlay(ThemeProps::new(themes.current()))
            .map(|overlay| overlay.kind())
    }

    #[test]
    fn the_row_menu_is_a_menu_and_the_dialogs_are_dialogs() {
        let mut menu = loaded();
        send(
            &mut menu,
            vec![ContainersIntent::OpenRowMenu {
                index: 0,
                position: Point::new(40.0, 40.0),
            }],
        );
        let mut confirm = loaded();
        send(
            &mut confirm,
            vec![ContainersIntent::Actions(ActionsIntent::Prune)],
        );
        let mut create = loaded();
        send(
            &mut create,
            vec![
                ContainersIntent::Open(Place::new(
                    fixtures::engine(EngineKind::Docker).address().clone(),
                    ResourceKind::Volumes,
                )),
                ContainersIntent::OpenCreate,
            ],
        );

        assert_eq!(top_kind(&loaded()), None);
        assert_eq!(top_kind(&menu), Some(OverlayKind::Menu));
        assert!(menu.has_row_menu());
        assert_eq!(top_kind(&confirm), Some(OverlayKind::Dialog));
        assert!(confirm.has_confirm());
        assert_eq!(top_kind(&create), Some(OverlayKind::Dialog));
        assert!(create.has_create_dialog());
    }

    /// The row menu is kept inside the window the binary told about.
    #[test]
    fn the_row_menu_opens_inside_the_window() {
        let themes = ThemeManager::new();
        let window = Size::new(600.0, 400.0);
        let mut containers = loaded();
        send(
            &mut containers,
            vec![
                ContainersIntent::WindowResized(window),
                ContainersIntent::OpenRowMenu {
                    index: 0,
                    position: Point::new(590.0, 390.0),
                },
            ],
        );
        let menu = containers
            .overlay(ThemeProps::new(themes.current()))
            .expect("the row menu")
            .into_content();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            window,
            menu,
        );

        let item = ui
            .find(otty_app_i18n::t(otty_app_i18n::Key::ContainersActionRemove))
            .expect("the menu should offer Remove")
            .bounds();

        assert!(item.x + item.width <= window.width, "{item:?}");
        assert!(item.y + item.height <= window.height, "{item:?}");
    }
}
