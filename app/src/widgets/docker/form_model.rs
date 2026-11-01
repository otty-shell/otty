//! View model of the open create dialog: its fields, its action button and
//! the progress line of a running pull.

use otty_docker::PullProgress;

use super::format::size_text;
use super::forms::{FormField, FormKind};
use super::state::DockerState;
use crate::i18n::{self, Key};

/// One text field of the create dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldViewModel<'a> {
    pub(crate) field: FormField,
    pub(crate) label: &'static str,
    pub(crate) value: &'a str,
    /// Shown while the field is empty; empty for none.
    pub(crate) placeholder: &'static str,
}

/// The progress line of a running pull.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PullLineViewModel {
    /// "Pulling nginx:latest".
    pub(crate) text: String,
    /// How far the bar is filled, 0 to 1.
    pub(crate) fraction: f32,
    /// "15.0 MB of 47.0 MB", once the engine sent sizes.
    pub(crate) amount: Option<String>,
}

/// The open create dialog.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FormViewModel<'a> {
    pub(crate) title: &'static str,
    pub(crate) fields: Vec<FieldViewModel<'a>>,
    /// `Some` while a pull runs.
    pub(crate) progress: Option<PullLineViewModel>,
    pub(crate) submit_label: &'static str,
    /// Whether the action button can be pressed.
    pub(crate) can_submit: bool,
    /// `false` while a pull runs: the fields do not take typing.
    pub(crate) is_editable: bool,
}

/// The open create dialog; `None` when none is open.
pub(crate) fn form_dialog(state: &DockerState) -> Option<FormViewModel<'_>> {
    let form = state.form()?;

    let (title, submit) = match form.kind() {
        FormKind::Pull => (Key::DockerPullImage, Key::DockerActionPull),
        FormKind::CreateVolume => {
            (Key::DockerCreateVolume, Key::DockerActionCreate)
        },
        FormKind::CreateNetwork => {
            (Key::DockerCreateNetwork, Key::DockerActionCreate)
        },
    };
    let fields = form
        .fields()
        .iter()
        .map(|&field| {
            let (label, placeholder) = field_texts(form.kind(), field);
            FieldViewModel {
                field,
                label: i18n::t(label),
                value: form.value(field),
                placeholder: placeholder.map_or("", i18n::t),
            }
        })
        .collect();
    let progress = form.progress().map(|progress| PullLineViewModel {
        text: i18n::docker_pulling(&form.reference()),
        fraction: progress.fraction(),
        amount: pull_amount(progress),
    });

    Some(FormViewModel {
        title: i18n::t(title),
        fields,
        progress,
        submit_label: i18n::t(submit),
        can_submit: form.can_submit(),
        is_editable: !form.is_pulling(),
    })
}

/// The bytes of a pull, e.g. "15.0 MB of 47.0 MB"; `None` until the
/// engine sent a layer size.
pub(crate) fn pull_amount(progress: &PullProgress) -> Option<String> {
    if progress.total() == 0 {
        return None;
    }

    Some(i18n::docker_pull_amount(
        &size_text(progress.downloaded()),
        &size_text(progress.total()),
    ))
}

/// The label and placeholder keys of `field` in a dialog of `kind`.
fn field_texts(kind: FormKind, field: FormField) -> (Key, Option<Key>) {
    match (kind, field) {
        (FormKind::Pull, FormField::Name) => {
            (Key::DockerFieldImage, Some(Key::DockerImagePlaceholder))
        },
        (FormKind::CreateVolume, FormField::Name) => {
            (Key::DockerFieldName, Some(Key::DockerVolumeNamePlaceholder))
        },
        (_, FormField::Name) => (Key::DockerFieldName, None),
        (_, FormField::Tag) => (Key::DockerFieldTag, None),
        (_, FormField::Driver) => (Key::DockerFieldDriver, None),
    }
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::docker::event::DockerIntent;
    use crate::widgets::docker::reducer::reduce;
    use crate::widgets::docker::types::{LoadedLists, Place, ResourceKind};

    fn state_with_form(resource: ResourceKind) -> DockerState {
        let docker = fixtures::engine(EngineKind::Docker);
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
                engine: docker.address().clone(),
                lists: LoadedLists {
                    containers: Ok(resources.containers().to_vec()),
                    images: Ok(resources.images().to_vec()),
                    volumes: Ok(resources.volumes().to_vec()),
                    networks: Ok(resources.networks().to_vec()),
                },
            },
        );
        let _ = reduce(
            &mut state,
            DockerIntent::Open(Place::new(docker.address().clone(), resource)),
        );
        let _ = reduce(&mut state, DockerIntent::OpenCreate);
        state
    }

    fn type_in(state: &mut DockerState, field: FormField, value: &str) {
        let _ = reduce(state, DockerIntent::FormChanged(field, value.into()));
    }

    #[test]
    fn the_pull_dialog_asks_for_an_image_and_a_tag() {
        let state = state_with_form(ResourceKind::Images);

        let vm = form_dialog(&state).expect("an open dialog");

        assert_eq!(vm.title, i18n::t(Key::DockerPullImage));
        assert_eq!(
            vm.fields,
            vec![
                FieldViewModel {
                    field: FormField::Name,
                    label: i18n::t(Key::DockerFieldImage),
                    value: "",
                    placeholder: i18n::t(Key::DockerImagePlaceholder),
                },
                FieldViewModel {
                    field: FormField::Tag,
                    label: i18n::t(Key::DockerFieldTag),
                    value: "latest",
                    placeholder: "",
                },
            ]
        );
        assert_eq!(vm.submit_label, i18n::t(Key::DockerActionPull));
        assert!(!vm.can_submit);
        assert!(vm.is_editable);
        assert_eq!(vm.progress, None);
    }

    #[test]
    fn a_named_image_can_be_pulled() {
        let mut state = state_with_form(ResourceKind::Images);

        type_in(&mut state, FormField::Name, "nginx");

        let vm = form_dialog(&state).expect("an open dialog");
        assert!(vm.can_submit);
    }

    #[test]
    fn a_running_pull_shows_its_progress_and_locks_the_fields() {
        let mut state = state_with_form(ResourceKind::Images);
        type_in(&mut state, FormField::Name, "nginx");
        type_in(&mut state, FormField::Tag, "1.27-alpine");
        let _ = reduce(&mut state, DockerIntent::FormSubmitted);
        let started = form_dialog(&state).and_then(|vm| vm.progress);
        let _ = reduce(
            &mut state,
            DockerIntent::PullProgressed(fixtures::pull_progress()),
        );

        let vm = form_dialog(&state).expect("an open dialog");

        assert_eq!(
            started,
            Some(PullLineViewModel {
                text: i18n::docker_pulling("nginx:1.27-alpine"),
                fraction: 0.0,
                amount: None,
            })
        );
        let progress = vm.progress.expect("a progress line");
        assert_eq!(progress.text, i18n::docker_pulling("nginx:1.27-alpine"));
        assert!((progress.fraction - 15.0 / 47.0).abs() < 1e-6);
        assert_eq!(
            progress.amount,
            Some(i18n::docker_pull_amount("15 MB", "47 MB"))
        );
        assert!(!vm.is_editable);
        assert!(!vm.can_submit);
    }

    #[test]
    fn the_create_dialogs_ask_for_a_name_and_a_driver() {
        let volume_state = state_with_form(ResourceKind::Volumes);
        let mut network_state = state_with_form(ResourceKind::Networks);
        let volume = form_dialog(&volume_state).expect("volume dialog");
        let empty_network_can_submit =
            form_dialog(&network_state).is_some_and(|vm| vm.can_submit);
        type_in(&mut network_state, FormField::Name, "otty-fx-net2");
        let network = form_dialog(&network_state).expect("network dialog");

        assert_eq!(volume.title, i18n::t(Key::DockerCreateVolume));
        assert_eq!(
            volume.fields[0].placeholder,
            i18n::t(Key::DockerVolumeNamePlaceholder)
        );
        assert_eq!(volume.fields[1].label, i18n::t(Key::DockerFieldDriver));
        assert_eq!(volume.fields[1].value, "local");
        assert_eq!(volume.submit_label, i18n::t(Key::DockerActionCreate));
        assert!(volume.can_submit);
        assert_eq!(network.title, i18n::t(Key::DockerCreateNetwork));
        assert_eq!(network.fields[0].label, i18n::t(Key::DockerFieldName));
        assert_eq!(network.fields[1].value, "bridge");
        assert!(!empty_network_can_submit);
        assert!(network.can_submit);
    }

    #[test]
    fn the_amount_shows_once_sizes_are_known() {
        let progress = otty_docker::PullProgress::default();

        assert_eq!(pull_amount(&progress), None);
        assert_eq!(
            pull_amount(&fixtures::pull_progress()),
            Some(i18n::docker_pull_amount("15 MB", "47 MB"))
        );
    }

    #[test]
    fn no_form_has_no_dialog() {
        assert_eq!(form_dialog(&DockerState::default()), None);
    }
}
