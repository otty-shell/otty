//! Containers mode of the OTTY app: the engines found on this computer in the
//! left panel, the containers, images, volumes and networks of each engine in
//! the main area, with the sort and the tag search of each page, the cards
//! shown while no engine is usable, and the left side of the status bar.
//!
//! The feature finds no engine and calls none itself: it has the data types
//! of `otty-containers` only. It reports the engine work it needs as a
//! [`ContainersOutcome`]; the binary runs it and gives the answer back as a
//! [`ContainersIntent`]. What to watch is decided here: the binary watches
//! the events of [`Containers::watched_engines`] and runs the timer of
//! [`Containers::engine_search_timer`].
//!
//! The inspector in the right panel, the Run container form, the create
//! dialogs and the actions with their row menu and confirm dialogs come with
//! issue 14, as inner modules of this crate.
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

mod event;
mod format;
mod links;
mod model;
mod no_engine;
mod overview;
mod reducer;
mod resource_rows;
mod sort;
mod state;
mod table_search;
mod types;
mod view;

use std::time::Duration;

pub use event::{ContainersIntent, ContainersOutcome};
use iced::Element;
use model::{EngineTreeViewModel, MainAreaViewModel, StatusBarViewModel};
use otty_app_slots::{Feature, MainArea, MainContent, StatusBar};
use otty_app_theme::theme::ThemeProps;
use otty_containers::EngineInfo;
use otty_ui_kit::components::status_bar::StatusItem;
use state::ContainersState;
pub use types::{
    ContainersError, LoadedList, LoadedLists, NoEngineCase, Place, ResourceKind,
};
use view::{engine_tree, main_area, status_bar};

/// How often the engines are looked for while the no-engine card shows.
const ENGINE_SEARCH_PERIOD: Duration = Duration::from_secs(5);

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
}

impl MainArea for Containers {
    type Screen = ContainersScreen;

    /// The open list of an engine, or a card while there is none; ages are
    /// counted up to the current time.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, ContainersIntent, ContainersScreen> {
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

impl StatusBar for Containers {
    /// The engine on screen and the counts of the open list, or the mode
    /// and the reason there is no engine.
    fn status_bar(&self) -> Vec<StatusItem<ContainersIntent>> {
        status_bar::items(&StatusBarViewModel::from_state(&self.state))
    }
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
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
}
