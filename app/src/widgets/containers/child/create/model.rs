//! View model of the open create dialog: its fields, its action button and
//! the progress line of a running pull.

use super::super::super::format::pull_amount;
use super::state::{CreateState, FormKind};
use super::types::FormField;
use crate::components::pull_line::PullLineViewModel;
use crate::i18n::{self, Key};

/// One text field of the create dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FormDialogFieldViewModel<'a> {
    pub(crate) field: FormField,
    pub(crate) label: &'static str,
    pub(crate) value: &'a str,
    /// Shown while the field is empty; empty for none.
    pub(crate) placeholder: &'static str,
}

/// The open create dialog.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FormDialogViewModel<'a> {
    pub(crate) title: &'static str,
    pub(crate) fields: Vec<FormDialogFieldViewModel<'a>>,
    /// `Some` while a pull runs.
    pub(crate) progress: Option<PullLineViewModel>,
    pub(crate) submit_label: &'static str,
    /// Whether the action button can be pressed.
    pub(crate) can_submit: bool,
    /// `false` while a pull runs: the fields do not take typing.
    pub(crate) is_editable: bool,
}

impl<'a> FormDialogViewModel<'a> {
    /// Build the open create dialog; `None` when none is open.
    pub(crate) fn from_state(state: &'a CreateState) -> Option<Self> {
        let form = state.form()?;

        let (title, submit) = match form.kind() {
            FormKind::Pull => {
                (Key::ContainersPullImage, Key::ContainersActionPull)
            },
            FormKind::CreateVolume => {
                (Key::ContainersCreateVolume, Key::ContainersActionCreate)
            },
            FormKind::CreateNetwork => {
                (Key::ContainersCreateNetwork, Key::ContainersActionCreate)
            },
        };
        let fields = form
            .fields()
            .iter()
            .map(|&field| {
                let (label, placeholder) = field_texts(form.kind(), field);
                FormDialogFieldViewModel {
                    field,
                    label: i18n::t(label),
                    value: form.value(field),
                    placeholder: placeholder.map_or("", i18n::t),
                }
            })
            .collect();
        let progress = form.progress().map(|progress| PullLineViewModel {
            text: i18n::containers_pulling(&form.reference()),
            fraction: progress.fraction(),
            amount: pull_amount(progress),
        });

        Some(Self {
            title: i18n::t(title),
            fields,
            progress,
            submit_label: i18n::t(submit),
            can_submit: form.can_submit(),
            is_editable: !form.is_pulling(),
        })
    }
}

/// The label and placeholder keys of `field` in a dialog of `kind`.
fn field_texts(kind: FormKind, field: FormField) -> (Key, Option<Key>) {
    match (kind, field) {
        (FormKind::Pull, FormField::Name) => (
            Key::ContainersFieldImage,
            Some(Key::ContainersImagePlaceholder),
        ),
        (FormKind::CreateVolume, FormField::Name) => (
            Key::ContainersFieldName,
            Some(Key::ContainersVolumeNamePlaceholder),
        ),
        (_, FormField::Name) => (Key::ContainersFieldName, None),
        (_, FormField::Tag) => (Key::ContainersFieldTag, None),
        (_, FormField::Driver) => (Key::ContainersFieldDriver, None),
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};

    use super::*;
    use crate::widgets::containers::child::create::event::CreateIntent;
    use crate::widgets::containers::event::ContainersIntent;
    use crate::widgets::containers::reducer::reduce;
    use crate::widgets::containers::state::ContainersState;
    use crate::widgets::containers::types::{LoadedLists, Place, ResourceKind};

    fn state_with_form(resource: ResourceKind) -> ContainersState {
        let docker = fixtures::engine(EngineKind::Docker);
        let resources = fixtures::resources(EngineKind::Docker);
        let mut state = ContainersState::default();
        let _ = reduce(&mut state, ContainersIntent::Enter);
        let _ = reduce(
            &mut state,
            ContainersIntent::DiscoveryFinished(fixtures::discovery(&[
                EngineKind::Docker,
            ])),
        );
        let _ = reduce(
            &mut state,
            ContainersIntent::ListsLoaded {
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
            ContainersIntent::Open(Place::new(
                docker.address().clone(),
                resource,
            )),
        );
        let _ = reduce(&mut state, ContainersIntent::OpenCreate);
        state
    }

    fn form_dialog(state: &ContainersState) -> Option<FormDialogViewModel<'_>> {
        FormDialogViewModel::from_state(state.create())
    }

    fn type_in(state: &mut ContainersState, field: FormField, value: &str) {
        let _ = reduce(
            state,
            ContainersIntent::Create(CreateIntent::Changed(
                field,
                value.into(),
            )),
        );
    }

    #[test]
    fn the_pull_dialog_asks_for_an_image_and_a_tag() {
        let state = state_with_form(ResourceKind::Images);

        let vm = form_dialog(&state).expect("an open dialog");

        assert_eq!(vm.title, i18n::t(Key::ContainersPullImage));
        assert_eq!(
            vm.fields,
            vec![
                FormDialogFieldViewModel {
                    field: FormField::Name,
                    label: i18n::t(Key::ContainersFieldImage),
                    value: "",
                    placeholder: i18n::t(Key::ContainersImagePlaceholder),
                },
                FormDialogFieldViewModel {
                    field: FormField::Tag,
                    label: i18n::t(Key::ContainersFieldTag),
                    value: "latest",
                    placeholder: "",
                },
            ]
        );
        assert_eq!(vm.submit_label, i18n::t(Key::ContainersActionPull));
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
        let _ = reduce(
            &mut state,
            ContainersIntent::Create(CreateIntent::Submitted),
        );
        let started = form_dialog(&state).and_then(|vm| vm.progress);
        let _ = reduce(
            &mut state,
            ContainersIntent::Create(CreateIntent::PullProgressed(
                fixtures::pull_progress(),
            )),
        );

        let vm = form_dialog(&state).expect("an open dialog");

        assert_eq!(
            started,
            Some(PullLineViewModel {
                text: i18n::containers_pulling("nginx:1.27-alpine"),
                fraction: 0.0,
                amount: None,
            })
        );
        let progress = vm.progress.expect("a progress line");
        assert_eq!(
            progress.text,
            i18n::containers_pulling("nginx:1.27-alpine")
        );
        assert!((progress.fraction - 15.0 / 47.0).abs() < 1e-6);
        assert_eq!(
            progress.amount,
            Some(i18n::containers_pull_amount("15 MB", "47 MB"))
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

        assert_eq!(volume.title, i18n::t(Key::ContainersCreateVolume));
        assert_eq!(
            volume.fields[0].placeholder,
            i18n::t(Key::ContainersVolumeNamePlaceholder)
        );
        assert_eq!(volume.fields[1].label, i18n::t(Key::ContainersFieldDriver));
        assert_eq!(volume.fields[1].value, "local");
        assert_eq!(volume.submit_label, i18n::t(Key::ContainersActionCreate));
        assert!(volume.can_submit);
        assert_eq!(network.title, i18n::t(Key::ContainersCreateNetwork));
        assert_eq!(network.fields[0].label, i18n::t(Key::ContainersFieldName));
        assert_eq!(network.fields[1].value, "bridge");
        assert!(!empty_network_can_submit);
        assert!(network.can_submit);
    }

    #[test]
    fn no_form_has_no_dialog() {
        assert_eq!(
            FormDialogViewModel::from_state(&CreateState::default()),
            None
        );
    }
}
