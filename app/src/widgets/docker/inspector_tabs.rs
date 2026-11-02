//! View model of the inspector tabs: the tabs of the selected row, the open
//! one and what it shows.

use std::collections::BTreeSet;

use otty_ui_kit::components::log_view::LogViewState;

use super::inspect_links::InspectLinks;
use super::inspect_tree::{InspectTree, inspect_tree};
use super::inspector::{InspectLoad, InspectTarget, InspectorTab};
use super::overview::{
    self, Field, container_fields, network_fields, volume_fields,
};
use super::state::{DockerState, EngineState, SelectedRow};

/// The tab row and the body of the open tab.
pub(crate) struct InspectorTabsViewModel<'a> {
    /// The tabs of the selected row: containers have Logs.
    pub(crate) tabs: &'static [InspectorTab],
    pub(crate) selected: InspectorTab,
    pub(crate) content: TabContent<'a>,
}

/// What the open tab shows.
pub(crate) enum TabContent<'a> {
    /// Key facts; empty ones are left out.
    Overview(Vec<Field>),
    /// The followed lines (`None` before the logs start) and the engine's
    /// message when they could not be read.
    Logs {
        lines: Option<&'a LogViewState>,
        error: Option<&'a str>,
    },
    /// The inspect answer of the selected row.
    Inspect(InspectContent<'a>),
}

/// The Inspect tab body.
pub(crate) enum InspectContent<'a> {
    /// The JSON is on its way.
    Loading,
    /// The answer, as a tree or as JSON.
    Loaded(InspectLoadedViewModel<'a>),
    /// The engine's message.
    Failed(&'a str),
}

/// A loaded Inspect tab.
pub(crate) struct InspectLoadedViewModel<'a> {
    /// The pretty-printed JSON.
    pub(crate) json: &'a LogViewState,
    /// The rows to show; `None` shows the JSON instead: the Raw JSON switch
    /// is on, or the answer is not a JSON object.
    pub(crate) tree: Option<InspectTree>,
    /// The titles of the open sections of the tree.
    pub(crate) open_sections: &'a BTreeSet<String>,
    /// Whether the tree shows empty values.
    pub(crate) shows_empty: bool,
}

/// The tabs of the selected row of the open list; `None` when nothing is
/// selected.
pub(crate) fn inspector_tabs(
    state: &DockerState,
) -> Option<InspectorTabsViewModel<'_>> {
    let (engine, row) = state.selected_row()?;
    let selected = state.inspector().tab();

    let content = match selected {
        InspectorTab::Logs => {
            let logs = state.inspector().logs();
            TabContent::Logs {
                lines: logs.map(|logs| logs.lines()),
                error: logs.and_then(|logs| logs.error()),
            }
        },
        InspectorTab::Inspect => {
            TabContent::Inspect(inspect_content(state, engine, &row))
        },
        _ => TabContent::Overview(overview_fields(engine, &row)),
    };

    Some(InspectorTabsViewModel {
        tabs: InspectorTab::of(row.resource()),
        selected,
        content,
    })
}

/// The Overview fields of `row`; their links point into the engine's
/// lists, and an image counts its users in the engine's containers.
fn overview_fields(engine: &EngineState, row: &SelectedRow<'_>) -> Vec<Field> {
    let containers = engine.containers().items();

    match row {
        SelectedRow::Container(container) => container_fields(
            container,
            engine.images().items(),
            engine.volumes().items(),
            engine.networks().items(),
        ),
        SelectedRow::Image(image) => overview::image_fields(image, containers),
        SelectedRow::Volume(volume) => volume_fields(volume, containers),
        SelectedRow::Network(network) => network_fields(network, containers),
    }
}

/// The inspect JSON of `row`; loading until it is the one that arrived. The
/// links of its tree point into the engine's lists.
fn inspect_content<'a>(
    state: &'a DockerState,
    engine: &EngineState,
    row: &SelectedRow<'_>,
) -> InspectContent<'a> {
    let target = InspectTarget::new(
        engine.info().address().clone(),
        row.resource(),
        row.id(),
    );
    let Some(inspect) = state
        .inspector()
        .inspect()
        .filter(|inspect| *inspect.target() == target)
    else {
        return InspectContent::Loading;
    };

    let inspector = state.inspector();

    match inspect.load() {
        InspectLoad::Loaded(loaded) => {
            let shows_empty = inspector.shows_empty();
            let links = InspectLinks {
                resource: row.resource(),
                containers: engine.containers().items(),
                images: engine.images().items(),
                volumes: engine.volumes().items(),
                networks: engine.networks().items(),
            };
            let tree = if inspector.is_raw_json() {
                None
            } else {
                inspect_tree(loaded.value(), shows_empty, &links)
            };

            InspectContent::Loaded(InspectLoadedViewModel {
                json: loaded.json(),
                tree,
                open_sections: loaded.open_sections(),
                shows_empty,
            })
        },
        InspectLoad::Failed(message) => InspectContent::Failed(message),
        _ => InspectContent::Loading,
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineInfo, EngineKind, fixtures};
    use otty_ui_kit::components::data_tree::DataRow;
    use otty_ui_kit::components::key_value_list::ValuePart;

    use super::*;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::inspector::{InspectTarget, LogTarget};
    use crate::widgets::docker::links::LinkTarget;
    use crate::widgets::docker::overview;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    fn loaded() -> DockerState {
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = DockerState::default();
        let _ = reduce(&mut state, DockerIntent::Enter);
        let _ = reduce(
            &mut state,
            DockerIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            DockerIntent::ListsLoaded {
                engine: docker().address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        state
    }

    fn select_tab(state: &mut DockerState, tab: InspectorTab) {
        let _ = reduce(state, DockerIntent::SelectInspectorTab(tab));
    }

    #[test]
    fn nothing_selected_has_no_tabs() {
        let state = DockerState::default();

        assert!(inspector_tabs(&state).is_none());
    }

    #[test]
    fn a_container_has_three_tabs_and_opens_on_its_overview() {
        let state = loaded();
        let resources = fixtures::resources(EngineKind::Docker);

        let vm = inspector_tabs(&state).expect("a selected container");

        assert_eq!(
            vm.tabs,
            [
                InspectorTab::Overview,
                InspectorTab::Logs,
                InspectorTab::Inspect
            ]
        );
        assert_eq!(vm.selected, InspectorTab::Overview);
        let TabContent::Overview(fields) = vm.content else {
            panic!("expected the overview");
        };
        assert_eq!(
            fields,
            overview::container_fields(
                &resources.containers()[0],
                resources.images(),
                resources.volumes(),
                resources.networks(),
            )
        );
    }

    #[test]
    fn other_rows_have_overview_and_inspect_with_their_own_fields() {
        let mut state = loaded();
        let resources = fixtures::resources(EngineKind::Docker);
        let mut shown = Vec::new();

        for resource in [
            ResourceKind::Images,
            ResourceKind::Volumes,
            ResourceKind::Networks,
        ] {
            let place = Place::new(docker().address().clone(), resource);
            let _ = reduce(&mut state, DockerIntent::Open(place));
            let vm = inspector_tabs(&state).expect("a selected row");
            assert_eq!(
                vm.tabs,
                [InspectorTab::Overview, InspectorTab::Inspect]
            );
            let TabContent::Overview(fields) = vm.content else {
                panic!("expected the overview");
            };
            shown.push(fields);
        }

        assert_eq!(
            shown,
            vec![
                overview::image_fields(
                    &resources.images()[0],
                    resources.containers()
                ),
                overview::volume_fields(
                    &resources.volumes()[0],
                    resources.containers()
                ),
                overview::network_fields(
                    &resources.networks()[0],
                    resources.containers()
                ),
            ]
        );
    }

    #[test]
    fn the_logs_tab_shows_the_followed_lines_and_their_error() {
        let mut state = loaded();
        let resources = fixtures::resources(EngineKind::Docker);
        let target = LogTarget::new(docker(), resources.containers()[0].id());
        select_tab(&mut state, InspectorTab::Logs);
        let _ = reduce(
            &mut state,
            DockerIntent::LogLines {
                target: target.clone(),
                lines: vec![String::from("ready")],
            },
        );
        let _ = reduce(
            &mut state,
            DockerIntent::LogsFailed {
                target,
                message: String::from("stream closed"),
            },
        );

        let vm = inspector_tabs(&state).expect("a selected container");

        assert_eq!(vm.selected, InspectorTab::Logs);
        let TabContent::Logs { lines, error } = vm.content else {
            panic!("expected the logs");
        };
        assert_eq!(lines.map(LogViewState::text), Some(String::from("ready")));
        assert_eq!(error, Some("stream closed"));
    }

    /// The Inspect tab of the first container with `json` loaded.
    fn with_inspect(json: &str) -> DockerState {
        let mut state = loaded();
        let resources = fixtures::resources(EngineKind::Docker);
        select_tab(&mut state, InspectorTab::Inspect);
        let _ = reduce(
            &mut state,
            DockerIntent::InspectLoaded {
                target: InspectTarget::new(
                    docker().address().clone(),
                    ResourceKind::Containers,
                    resources.containers()[0].id(),
                ),
                result: Ok(json.to_string()),
            },
        );
        state
    }

    /// Runs `check` on the loaded Inspect tab of `state`.
    fn check_loaded(
        state: &DockerState,
        check: impl FnOnce(InspectLoadedViewModel<'_>),
    ) {
        let vm = inspector_tabs(state).expect("a selected container");
        let TabContent::Inspect(InspectContent::Loaded(loaded)) = vm.content
        else {
            panic!("expected the loaded inspect");
        };

        check(loaded);
    }

    #[test]
    fn the_inspect_tab_shows_loading_then_the_tree_and_the_json() {
        let mut state = loaded();
        select_tab(&mut state, InspectorTab::Inspect);

        let loading = matches!(
            inspector_tabs(&state).map(|vm| vm.content),
            Some(TabContent::Inspect(InspectContent::Loading))
        );
        let state = with_inspect(
            "{\n  \"Id\": \"850b\",\n  \"State\": {\n    \"Status\": \"up\"\n  }\n}",
        );

        assert!(loading);
        check_loaded(&state, |loaded| {
            let tree = loaded.tree.expect("the tree");
            assert_eq!(tree.general, vec![DataRow::new(0, "Id", "850b")]);
            assert_eq!(tree.sections[0].title, "State");
            assert_eq!(
                loaded.open_sections.iter().collect::<Vec<_>>(),
                vec!["State"]
            );
            assert!(loaded.json.text().starts_with("{\n  \"Id\""));
            assert!(!loaded.shows_empty);
        });
    }

    #[test]
    fn the_links_of_the_inspect_tree_point_into_the_loaded_lists() {
        let resources = fixtures::resources(EngineKind::Docker);
        let image_id = resources.containers()[0].image_id();
        let state = with_inspect(&format!(
            "{{ \"Image\": \"{image_id}\", \"Mounts\": [{{ \"Name\": \
             \"otty-fx-data\" }}] }}"
        ));

        check_loaded(&state, |loaded| {
            let tree = loaded.tree.expect("the tree");
            assert_eq!(
                tree.general,
                vec![DataRow::with_parts(
                    0,
                    "Image",
                    [ValuePart::link(
                        image_id,
                        LinkTarget::Item(
                            ResourceKind::Images,
                            image_id.to_string()
                        )
                    )]
                )]
            );
            assert_eq!(
                tree.sections[0].rows,
                vec![
                    DataRow::heading(0, "1"),
                    DataRow::with_parts(
                        1,
                        "Name",
                        [ValuePart::link(
                            "otty-fx-data",
                            LinkTarget::Item(
                                ResourceKind::Volumes,
                                String::from("otty-fx-data")
                            )
                        )]
                    ),
                ]
            );
        });
    }

    #[test]
    fn the_links_of_the_inspect_tree_follow_the_kind_of_the_selected_row() {
        let resources = fixtures::resources(EngineKind::Docker);
        let web = resources
            .containers()
            .iter()
            .find(|container| container.name() == "otty-fx-web-1")
            .expect("web container");
        let network = resources.networks()[0].id();
        let mut state = loaded();
        let place =
            Place::new(docker().address().clone(), ResourceKind::Networks);
        let _ = reduce(&mut state, DockerIntent::Open(place));
        select_tab(&mut state, InspectorTab::Inspect);

        let _ = reduce(
            &mut state,
            DockerIntent::InspectLoaded {
                target: InspectTarget::new(
                    docker().address().clone(),
                    ResourceKind::Networks,
                    network,
                ),
                result: Ok(String::from(
                    "{ \"Containers\": { \"a\": { \"Name\": \"otty-fx-web-1\" \
                     } } }",
                )),
            },
        );

        check_loaded(&state, |loaded| {
            let tree = loaded.tree.expect("the tree");
            assert_eq!(
                tree.sections[0].rows,
                vec![
                    DataRow::heading(0, "a"),
                    DataRow::with_parts(
                        1,
                        "Name",
                        [ValuePart::link(
                            "otty-fx-web-1",
                            LinkTarget::Item(
                                ResourceKind::Containers,
                                web.id().to_string()
                            )
                        )]
                    ),
                ]
            );
        });
    }

    #[test]
    fn the_raw_json_switch_shows_the_json_instead_of_the_tree() {
        let mut state = with_inspect("{ \"Id\": \"850b\" }");

        let _ = reduce(&mut state, DockerIntent::InspectRawJsonToggled(true));

        check_loaded(&state, |loaded| {
            assert!(loaded.tree.is_none());
            assert_eq!(loaded.json.text(), "{ \"Id\": \"850b\" }");
        });
    }

    #[test]
    fn the_show_empty_switch_brings_the_empty_values_into_the_tree() {
        let mut state = with_inspect("{ \"Id\": \"850b\", \"Name\": \"\" }");
        check_loaded(&state, |loaded| {
            let tree = loaded.tree.expect("the tree");
            assert_eq!(tree.general, vec![DataRow::new(0, "Id", "850b")]);
        });

        let _ = reduce(&mut state, DockerIntent::InspectShowEmptyToggled(true));

        check_loaded(&state, |loaded| {
            let tree = loaded.tree.expect("the tree");
            assert_eq!(
                tree.general,
                vec![
                    DataRow::new(0, "Id", "850b"),
                    DataRow::new(0, "Name", "\"\""),
                ]
            );
            assert!(loaded.shows_empty);
        });
    }

    #[test]
    fn an_answer_that_is_not_an_object_shows_as_json() {
        for answer in ["not json", "[1, 2]"] {
            let state = with_inspect(answer);

            check_loaded(&state, |loaded| {
                assert!(loaded.tree.is_none());
                assert_eq!(loaded.json.text(), answer);
            });
        }
    }

    #[test]
    fn a_failed_inspect_shows_the_engine_message() {
        let mut state = loaded();
        let resources = fixtures::resources(EngineKind::Docker);
        select_tab(&mut state, InspectorTab::Inspect);
        let _ = reduce(
            &mut state,
            DockerIntent::InspectLoaded {
                target: InspectTarget::new(
                    docker().address().clone(),
                    ResourceKind::Containers,
                    resources.containers()[0].id(),
                ),
                result: Err(String::from("engine said no")),
            },
        );

        let vm = inspector_tabs(&state).expect("a selected container");

        assert!(matches!(
            vm.content,
            TabContent::Inspect(InspectContent::Failed("engine said no"))
        ));
    }
}
