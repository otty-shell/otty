mod engine_tree;
mod inspector;
mod main_area;
mod no_engine;
mod status_bar;

pub(crate) use engine_tree::{
    EngineTreeRowViewModel, EngineTreeViewModel, TreeNote,
};
pub(crate) use inspector::InspectorViewModel;
pub(crate) use main_area::{MainAreaViewModel, TableViewModel};
pub(crate) use no_engine::{
    NoEngineCommandViewModel, NoEngineLinkViewModel, NoEngineViewModel,
};
pub(crate) use status_bar::StatusBarViewModel;

#[cfg(test)]
mod tests {
    use otty_app_i18n::{self as i18n, Key};
    use otty_containers::{CandidateSource, Discovery, EngineKind, fixtures};

    use super::*;
    use crate::event::{ContainersIntent, ContainersOutcome};
    use crate::reducer::reduce;
    use crate::run::event::RunIntent;
    use crate::run::types::RunEdit;
    use crate::run::{RunFormViewModel, RunPanelViewModel};
    use crate::state::ContainersState;
    use crate::testing::{
        add_tag, build, docker, found, highlighted, labels, lists, load,
        open_labels, podman, press_root, table, two_tags_of_one_image,
        type_text,
    };
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
        assert_eq!(vm.inspector, None);
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
    fn the_card_replaces_an_open_run_form_and_the_form_waits_for_the_engine() {
        for (found, case, commands, installs) in lost_engine_cases() {
            let mut state = only_engine(EngineKind::Docker);
            let _ = reduce(&mut state, ContainersIntent::OpenCreate);
            let _ = reduce(
                &mut state,
                ContainersIntent::Run(RunIntent::Edited(RunEdit::Image(
                    String::from("nginx"),
                ))),
            );

            lose_engine(&mut state, EngineKind::Docker, found);

            let ctx = state.run_ctx();
            let form = RunFormViewModel::from_state(state.run(), &ctx);
            let panel = RunPanelViewModel::from_state(state.run(), &ctx);
            assert_eq!(form.map(|_| ()), None, "{case:?}");
            assert_eq!(panel.map(|_| ()), None, "{case:?}");
            assert_only_the_card(&state, case, commands, installs);

            let _ = reduce(
                &mut state,
                ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                    EngineKind::Docker,
                ])),
            );

            let form = state.run().form().expect("the form was kept");
            assert_eq!(form.reference(), "nginx");
            let vm =
                RunFormViewModel::from_state(state.run(), &state.run_ctx());
            assert!(vm.is_some(), "{case:?}");
        }
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
        assert_eq!(vm.inspector, None);
    }

    #[test]
    fn before_any_engine_is_known_the_main_area_says_it_searches() {
        let state = ContainersState::default();

        let vm = build(&state);

        assert_eq!(vm.main, MainAreaViewModel::Searching);
        assert!(vm.tree.rows.is_empty());
        assert_eq!(vm.inspector, None);
    }

    #[test]
    fn closing_the_shown_engine_keeps_its_table_and_highlights_no_row() {
        let mut state = found(&[EngineKind::Docker, EngineKind::Podman]);
        load(&mut state, &docker(), lists(EngineKind::Docker));
        let before = build(&state);
        let rows_before = table(&before).rows.clone();
        let inspector_before = before.inspector.clone();

        press_root(&mut state, &docker());

        let vm = build(&state);
        assert_eq!(table(&vm).resource, ResourceKind::Containers);
        assert_eq!(table(&vm).rows, rows_before);
        assert_eq!(vm.inspector, inspector_before);
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
        assert_eq!(
            vm.inspector.map(|inspector| inspector.title),
            Some(String::from("otty-fx-web-1"))
        );
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
        assert_eq!(vm.inspector, None);
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
        assert_eq!(vm.inspector, None);
    }
}
