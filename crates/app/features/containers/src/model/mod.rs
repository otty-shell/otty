mod engine_tree;
mod main_area;
mod no_engine;
mod status_bar;

pub(crate) use engine_tree::{
    EngineTreeRowViewModel, EngineTreeViewModel, TreeNote,
};
pub(crate) use main_area::{MainAreaViewModel, TableViewModel};
pub(crate) use no_engine::{
    NoEngineCommandViewModel, NoEngineLinkViewModel, NoEngineViewModel,
};
pub(crate) use status_bar::StatusBarViewModel;

/// Fixtures that the tests of several view model files share.
#[cfg(test)]
mod test_support {
    use otty_app_i18n::{self as i18n, Key};
    use otty_containers::{EngineAddress, EngineInfo, EngineKind, fixtures};
    use otty_ui_kit::components::tag_search::TagSearchAction;

    use super::{EngineTreeViewModel, MainAreaViewModel, TableViewModel};
    use crate::event::ContainersIntent;
    use crate::reducer::reduce;
    use crate::state::ContainersState;
    use crate::types::{LoadedLists, Place, ResourceKind};

    pub(super) const NOW: i64 = 1_793_557_960;

    /// The engine tree and the main area of Containers mode at [`NOW`]. Issue
    /// 14 adds the inspector.
    pub(super) struct Vm<'a> {
        pub(super) tree: EngineTreeViewModel,
        pub(super) main: MainAreaViewModel<'a>,
    }

    pub(super) fn build(state: &ContainersState) -> Vm<'_> {
        Vm {
            tree: EngineTreeViewModel::from_state(state),
            main: MainAreaViewModel::from_state(state, NOW),
        }
    }

    pub(super) fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    pub(super) fn podman() -> EngineInfo {
        fixtures::engine(EngineKind::Podman)
    }

    pub(super) fn lists(kind: EngineKind) -> LoadedLists {
        let resources = fixtures::resources(kind);

        LoadedLists {
            containers: Ok(resources.containers().to_vec()),
            images: Ok(resources.images().to_vec()),
            volumes: Ok(resources.volumes().to_vec()),
            networks: Ok(resources.networks().to_vec()),
        }
    }

    pub(super) fn found(kinds: &[EngineKind]) -> ContainersState {
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(kinds)),
        );
        state
    }

    pub(super) fn load(
        state: &mut ContainersState,
        info: &EngineInfo,
        lists: LoadedLists,
    ) {
        let _ = reduce(
            state,
            ContainersIntent::ListsLoaded {
                engine: info.address().clone(),
                lists,
            },
        );
    }

    pub(super) fn open(state: &mut ContainersState, resource: ResourceKind) {
        let place = Place::new(docker().address().clone(), resource);
        let _ = reduce(state, ContainersIntent::Open(place));
    }

    pub(super) fn table<'a>(vm: &'a Vm<'_>) -> &'a TableViewModel<'a> {
        match &vm.main {
            MainAreaViewModel::Table(table) => table,
            other => panic!("expected a table, got {other:?}"),
        }
    }

    /// The labels of the tree rows, top to bottom.
    pub(super) fn labels(state: &ContainersState) -> Vec<String> {
        EngineTreeViewModel::from_state(state)
            .rows
            .into_iter()
            .map(|row| row.label)
            .collect()
    }

    /// The labels of the highlighted tree rows with the engine of each.
    pub(super) fn highlighted(
        state: &ContainersState,
    ) -> Vec<(String, EngineAddress)> {
        EngineTreeViewModel::from_state(state)
            .rows
            .into_iter()
            .filter(|row| row.is_selected)
            .map(|row| (row.label, row.place.engine().clone()))
            .collect()
    }

    pub(super) fn press_root(state: &mut ContainersState, info: &EngineInfo) {
        let _ = reduce(
            state,
            ContainersIntent::ToggleEngine(info.address().clone()),
        );
    }

    /// The labels of an open engine: its root and its four lists.
    pub(super) fn open_labels(root: &str) -> Vec<String> {
        [
            root,
            i18n::t(Key::ContainersContainers),
            i18n::t(Key::ContainersImages),
            i18n::t(Key::ContainersVolumes),
            i18n::t(Key::ContainersNetworks),
        ]
        .map(str::to_string)
        .to_vec()
    }

    pub(super) fn search_does(
        state: &mut ContainersState,
        action: TagSearchAction,
    ) {
        let _ = reduce(state, ContainersIntent::TagSearch(action));
    }

    pub(super) fn type_text(state: &mut ContainersState, text: &str) {
        search_does(state, TagSearchAction::InputChanged(text.to_string()));
    }

    /// Adds the typed `key:text` as it is, as Enter does with no suggestion.
    pub(super) fn add_tag(state: &mut ContainersState, text: &str) {
        type_text(state, text);
        search_does(state, TagSearchAction::Close);
        search_does(state, TagSearchAction::Apply);
    }

    /// Docker with one image of two tags, its images open.
    pub(super) fn two_tags_of_one_image() -> ContainersState {
        let mut state = found(&[EngineKind::Docker]);
        let mut loaded = lists(EngineKind::Docker);
        loaded.images = Ok(fixtures::image_rows(
            "sha256:app",
            &["app:1", "app:2"],
            &[],
            &[],
        ));
        load(&mut state, &docker(), loaded);
        open(&mut state, ResourceKind::Images);
        state
    }
}

#[cfg(test)]
mod tests {
    use otty_app_i18n::{self as i18n, Key};
    use otty_containers::{CandidateSource, Discovery, EngineKind, fixtures};

    use super::test_support::{
        add_tag, build, docker, found, highlighted, labels, lists, load,
        open_labels, podman, press_root, table, two_tags_of_one_image,
        type_text,
    };
    use super::*;
    use crate::event::{ContainersIntent, ContainersOutcome};
    use crate::reducer::reduce;
    use crate::state::ContainersState;
    use crate::types::{ContainersError, NoEngineCase, ResourceKind};

    // From losing the only engine to the card that says what to do.

    const DOCKER_PATH: &str = "/usr/bin/docker";
    const PODMAN_PATH: &str = "/usr/bin/podman";

    /// What a search says after the only engine stopped, and the card it must
    /// show: the case, the commands, and how many install buttons.
    fn lost_engine_cases()
    -> [(Discovery, NoEngineCase, &'static [&'static str], usize); 6] {
        [
            (
                fixtures::discovery_with_cli(&[], Some(DOCKER_PATH), None),
                NoEngineCase::DockerNotRunning,
                &["sudo systemctl start docker"],
                0,
            ),
            (
                fixtures::discovery_with_cli(&[], None, Some(PODMAN_PATH)),
                NoEngineCase::PodmanApiOff,
                &["systemctl --user enable --now podman.socket"],
                0,
            ),
            (
                fixtures::discovery_with_cli(
                    &[],
                    Some(DOCKER_PATH),
                    Some(PODMAN_PATH),
                ),
                NoEngineCase::DockerAndPodmanOff,
                &[
                    "sudo systemctl start docker",
                    "systemctl --user enable --now podman.socket",
                ],
                0,
            ),
            (
                fixtures::denied_discovery_with_cli(
                    fixtures::DOCKER_SOCKET,
                    CandidateSource::Docker,
                    None,
                    Some(PODMAN_PATH),
                ),
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Docker),
                &[
                    "sudo usermod -aG docker $USER",
                    "systemctl --user enable --now podman.socket",
                ],
                0,
            ),
            (
                fixtures::denied_discovery_with_cli(
                    fixtures::PODMAN_SOCKET,
                    CandidateSource::Podman,
                    Some(DOCKER_PATH),
                    None,
                ),
                NoEngineCase::NoPermissionAndOtherOff(EngineKind::Podman),
                &[
                    "systemctl --user enable --now podman.socket",
                    "sudo systemctl start docker",
                ],
                0,
            ),
            (fixtures::discovery(&[]), NoEngineCase::NotInstalled, &[], 2),
        ]
    }

    /// The engine of `kind` is the only one and its lists are loaded.
    fn only_engine(kind: EngineKind) -> ContainersState {
        let mut state = found(&[kind]);
        load(&mut state, &fixtures::engine(kind), lists(kind));

        state
    }

    /// The only engine stops answering and the search that follows says
    /// `found`, as the app does it.
    fn lose_engine(
        state: &mut ContainersState,
        kind: EngineKind,
        found: Discovery,
    ) {
        let info = fixtures::engine(kind);
        let sent = reduce(
            state,
            ContainersIntent::EngineUnreachable(info.address().clone()),
        );
        assert_eq!(
            sent,
            vec![ContainersOutcome::Discover],
            "a lost engine starts a search"
        );

        let _ = reduce(state, ContainersIntent::DiscoveryFinished(found));
    }

    /// Checks that everything says what `case` says and nothing of the engine
    /// is left.
    fn assert_only_the_card(
        state: &ContainersState,
        case: NoEngineCase,
        commands: &[&str],
        installs: usize,
    ) {
        let vm = build(state);
        let MainAreaViewModel::NoEngine(card) = &vm.main else {
            panic!("expected the no engine card, got {:?}", vm.main);
        };

        assert_eq!(card.case, case);
        let shown: Vec<&str> =
            card.commands.iter().map(|command| command.text).collect();
        assert_eq!(shown, commands);
        assert_eq!(card.installs.len(), installs);
        assert!(vm.tree.rows.is_empty(), "stale rows: {:?}", vm.tree.rows);
        assert_eq!(vm.tree.note, Some(card.panel_text));
        assert_eq!(state.place(), None);

        let bar = StatusBarViewModel::from_state(state);
        assert_eq!(bar.engine, None);
        assert_eq!(bar.info, card.status_text);
    }

    #[test]
    fn losing_the_only_engine_shows_the_card_of_the_search() {
        for kind in [EngineKind::Docker, EngineKind::Podman] {
            for (found, case, commands, installs) in lost_engine_cases() {
                let mut state = only_engine(kind);

                lose_engine(&mut state, kind, found);

                assert_only_the_card(&state, case, commands, installs);
            }
        }
    }

    #[test]
    fn losing_the_only_engine_while_it_is_closed_shows_the_same_card() {
        for (found, case, commands, installs) in lost_engine_cases() {
            let mut state = only_engine(EngineKind::Docker);
            press_root(&mut state, &docker());

            lose_engine(&mut state, EngineKind::Docker, found);

            assert_only_the_card(&state, case, commands, installs);
            assert!(state.is_engine_closed(docker().address()));
        }
    }

    #[test]
    fn the_table_stays_until_the_search_after_a_lost_engine_finishes() {
        let mut state = only_engine(EngineKind::Docker);

        let _ = reduce(
            &mut state,
            ContainersIntent::EngineUnreachable(docker().address().clone()),
        );

        assert!(matches!(build(&state).main, MainAreaViewModel::Table(_)));
        assert!(!EngineTreeViewModel::from_state(&state).rows.is_empty());
    }

    #[test]
    fn losing_one_of_two_engines_shows_no_card_and_no_row_of_the_lost_one() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        load(&mut state, &podman(), lists(EngineKind::Podman));

        lose_engine(
            &mut state,
            EngineKind::Docker,
            fixtures::discovery(&[EngineKind::Podman]),
        );

        let vm = build(&state);
        assert!(matches!(vm.main, MainAreaViewModel::Table(_)));
        assert_eq!(labels(&state), open_labels("Podman 5.8.8"));
        assert_eq!(EngineTreeViewModel::from_state(&state).note, None);
    }

    #[test]
    fn a_closed_engine_that_comes_back_shows_its_table_under_a_closed_root() {
        let mut state = only_engine(EngineKind::Docker);
        press_root(&mut state, &docker());
        lose_engine(
            &mut state,
            EngineKind::Docker,
            fixtures::discovery_with_cli(&[], Some(DOCKER_PATH), None),
        );

        let _ = reduce(&mut state, ContainersIntent::CheckAgain);
        assert_eq!(build(&state).main, MainAreaViewModel::Searching);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        load(&mut state, &docker(), lists(EngineKind::Docker));

        let vm = build(&state);
        assert_eq!(table(&vm).rows.len(), 6);
        assert_eq!(labels(&state), vec![String::from("Docker 29.9.1")]);
        assert!(!vm.tree.rows[0].is_open);
        assert!(highlighted(&state).is_empty());
        assert_eq!(EngineTreeViewModel::from_state(&state).note, None);
    }

    #[test]
    fn an_open_engine_that_comes_back_shows_its_table_under_an_open_root() {
        let mut state = only_engine(EngineKind::Docker);
        lose_engine(
            &mut state,
            EngineKind::Docker,
            fixtures::discovery_with_cli(&[], Some(DOCKER_PATH), None),
        );

        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );

        assert_eq!(labels(&state), open_labels("Docker 29.9.1"));
        assert_eq!(highlighted(&state).len(), 1);
    }

    #[test]
    fn a_search_that_found_nothing_shows_the_no_engine_card_and_panel_text() {
        let state = found(&[]);

        let vm = build(&state);

        let MainAreaViewModel::NoEngine(card) = vm.main else {
            panic!("expected the no engine card");
        };
        assert_eq!(card.title, i18n::t(Key::ContainersNotInstalledTitle));
        assert_eq!(
            vm.tree.note,
            Some(i18n::t(Key::ContainersPanelNotInstalled))
        );
        assert!(vm.tree.rows.is_empty());
    }

    #[test]
    fn before_any_engine_is_known_the_main_area_says_it_searches() {
        let state = ContainersState::default();

        let vm = build(&state);

        assert_eq!(vm.main, MainAreaViewModel::Searching);
        assert!(vm.tree.rows.is_empty());
    }

    #[test]
    fn closing_the_shown_engine_keeps_its_table_and_highlights_no_row() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let before = build(&state);
        let rows_before = table(&before).rows.clone();

        press_root(&mut state, &docker());

        let vm = build(&state);
        assert_eq!(table(&vm).resource, ResourceKind::Containers);
        assert_eq!(table(&vm).rows, rows_before);
        assert_eq!(highlighted(&state), Vec::new());
    }

    #[test]
    fn a_compose_tag_keeps_the_containers_row_selected_and_limits_the_rows() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        add_tag(&mut state, "compose:otty-fx");

        let vm = build(&state);
        let table = table(&vm);

        let selected: Vec<&str> = vm
            .tree
            .rows
            .iter()
            .filter(|row| row.is_selected)
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(selected, vec!["Containers"]);
        assert_eq!(table.resource, ResourceKind::Containers);
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.search.tags().len(), 1);
        assert_eq!(table.empty_message, None);
    }

    #[test]
    fn a_selected_row_outside_the_filter_is_not_highlighted() {
        let mut state = found(&[EngineKind::Docker]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        type_text(&mut state, "paused");

        let vm = build(&state);
        let table = table(&vm);

        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.selected, None);
    }

    #[test]
    fn an_empty_or_loading_list_says_so() {
        let mut state = found(&[EngineKind::Docker]);
        let loading = build(&state);
        let loading_message = table(&loading).empty_message.clone();
        let mut empty = lists(EngineKind::Docker);
        empty.containers = Ok(Vec::new());
        load(&mut state, &docker(), empty);

        let vm = build(&state);

        assert_eq!(
            loading_message,
            Some(i18n::t(Key::ContainersLoading).to_string())
        );
        assert_eq!(
            table(&vm).empty_message,
            Some(i18n::t(Key::ContainersNoContainers).to_string())
        );
    }

    #[test]
    fn the_tree_counts_the_rows_of_the_images_table() {
        let state = two_tags_of_one_image();

        let vm = build(&state);

        assert_eq!(vm.tree.rows[2].note, TreeNote::Count(2));
        assert_eq!(table(&vm).rows.len(), 2);
    }

    #[test]
    fn a_failed_list_shows_the_engine_message_and_no_rows() {
        let mut state = found(&[EngineKind::Docker]);
        let mut failed = lists(EngineKind::Docker);
        failed.containers = Err(ContainersError::new("engine said no"));
        load(&mut state, &docker(), failed);

        let vm = build(&state);

        assert_eq!(table(&vm).error, Some("engine said no"));
        assert!(table(&vm).rows.is_empty());
        assert_eq!(table(&vm).empty_message, None);
    }
}
