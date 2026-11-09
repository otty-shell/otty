//! The create dialogs above the tables (spec Decision 11): Pull image (name
//! and tag, then the pull's progress), Create volume and Create network
//! (name and driver).

use otty_containers::{EngineInfo, PullProgress};

use super::super::super::types::{DEFAULT_TAG, EngineAction, PullHandle};
use super::types::FormField;

/// The driver of a new volume unless another one is typed.
const VOLUME_DRIVER: &str = "local";
/// The driver of a new network unless another one is typed.
const NETWORK_DRIVER: &str = "bridge";

/// The create dialog open above the tables, if any.
#[derive(Debug, Default)]
pub(crate) struct CreateState {
    form: Option<Form>,
    /// The abort handle of the pull dialog's running pull. It lives beside
    /// the dialog, so a dialog opened in place of a pulling one leaves that
    /// pull running, and the next pull's handle replaces it, which stops it.
    pull: Option<PullHandle>,
}

impl CreateState {
    /// Return the open create dialog.
    pub(crate) fn form(&self) -> Option<&Form> {
        self.form.as_ref()
    }

    /// Return whether a create dialog is open.
    pub(crate) fn is_open(&self) -> bool {
        self.form.is_some()
    }

    /// Open a create dialog in place of the one that was open.
    pub(super) fn open_form(&mut self, form: Form) {
        self.form = Some(form);
    }

    /// Return the open create dialog to change it.
    pub(super) fn form_mut(&mut self) -> Option<&mut Form> {
        self.form.as_mut()
    }

    /// Close the create dialog and return it.
    pub(super) fn take_form(&mut self) -> Option<Form> {
        self.form.take()
    }

    /// Keep the abort handle of the pull that just started; the handle of
    /// an earlier pull is dropped, which stops that pull.
    pub(super) fn keep_pull(&mut self, pull: PullHandle) {
        self.pull = Some(pull);
    }

    /// Take the abort handle of the running pull, to stop it.
    pub(super) fn take_pull(&mut self) -> Option<PullHandle> {
        self.pull.take()
    }

    /// Forget the abort handle of the pull that ended.
    pub(super) fn forget_pull(&mut self) {
        self.pull = None;
    }
}

#[cfg(test)]
impl CreateState {
    /// Return the abort handle of the running pull.
    pub(crate) fn pull(&self) -> Option<&PullHandle> {
        self.pull.as_ref()
    }
}

/// Which create dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormKind {
    Pull,
    CreateVolume,
    CreateNetwork,
}

/// An open create dialog: what it creates, on which engine, the typed
/// values and, once a pull runs, its progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Form {
    kind: FormKind,
    engine: EngineInfo,
    name: String,
    tag: String,
    driver: String,
    /// `Some` while the pull runs.
    progress: Option<PullProgress>,
}

impl Form {
    /// Which dialog this is.
    pub(crate) fn kind(&self) -> FormKind {
        self.kind
    }

    /// The engine the dialog creates on.
    pub(crate) fn engine(&self) -> &EngineInfo {
        &self.engine
    }

    /// The fields the dialog shows, top to bottom.
    pub(crate) fn fields(&self) -> &'static [FormField] {
        match self.kind {
            FormKind::Pull => &[FormField::Name, FormField::Tag],
            _ => &[FormField::Name, FormField::Driver],
        }
    }

    /// The typed value of `field`.
    pub(crate) fn value(&self, field: FormField) -> &str {
        match field {
            FormField::Name => &self.name,
            FormField::Tag => &self.tag,
            FormField::Driver => &self.driver,
        }
    }

    /// The image to pull, without blanks around it.
    pub(crate) fn image(&self) -> &str {
        self.name.trim()
    }

    /// The tag to pull, without blanks around it.
    pub(crate) fn tag(&self) -> &str {
        self.tag.trim()
    }

    /// The image and tag as one reference, e.g. `nginx:latest`.
    pub(crate) fn reference(&self) -> String {
        format!("{}:{}", self.image(), self.tag())
    }

    /// Whether a pull runs.
    pub(crate) fn is_pulling(&self) -> bool {
        self.progress.is_some()
    }

    /// How far the running pull is.
    pub(crate) fn progress(&self) -> Option<&PullProgress> {
        self.progress.as_ref()
    }

    /// Whether the action button can be pressed: nothing runs and the
    /// required fields are filled (a pull needs an image and a tag, a
    /// network a name, both creates a driver).
    pub(crate) fn can_submit(&self) -> bool {
        if self.is_pulling() {
            return false;
        }

        match self.kind {
            FormKind::Pull => {
                !self.image().is_empty() && !self.tag().is_empty()
            },
            FormKind::CreateVolume => !self.driver.trim().is_empty(),
            FormKind::CreateNetwork => {
                !self.name.trim().is_empty() && !self.driver.trim().is_empty()
            },
        }
    }

    /// The pull dialog for `engine`, filled with `image` and `tag` (the
    /// default tag when blank).
    pub(super) fn pull(engine: EngineInfo, image: &str, tag: &str) -> Self {
        let tag = if tag.trim().is_empty() {
            DEFAULT_TAG
        } else {
            tag
        };

        Self {
            kind: FormKind::Pull,
            engine,
            name: image.to_string(),
            tag: tag.to_string(),
            driver: String::new(),
            progress: None,
        }
    }

    /// The create volume dialog for `engine`, with the `local` driver.
    pub(super) fn create_volume(engine: EngineInfo) -> Self {
        Self::create(FormKind::CreateVolume, engine, VOLUME_DRIVER)
    }

    /// The create network dialog for `engine`, with the `bridge` driver.
    pub(super) fn create_network(engine: EngineInfo) -> Self {
        Self::create(FormKind::CreateNetwork, engine, NETWORK_DRIVER)
    }

    /// Replace the value of `field`; nothing changes while a pull runs.
    pub(super) fn set_value(&mut self, field: FormField, value: String) {
        if self.is_pulling() {
            return;
        }

        match field {
            FormField::Name => self.name = value,
            FormField::Tag => self.tag = value,
            FormField::Driver => self.driver = value,
        }
    }

    /// Mark the pull as started, with no progress yet.
    pub(super) fn start_pull(&mut self) {
        if self.kind == FormKind::Pull {
            self.progress = Some(PullProgress::default());
        }
    }

    /// Take the newest progress of the running pull.
    pub(super) fn set_progress(&mut self, progress: PullProgress) {
        if self.is_pulling() {
            self.progress = Some(progress);
        }
    }

    /// The engine call of a create dialog; `None` for a pull, which is a
    /// stream of its own.
    pub(crate) fn create_action(&self) -> Option<EngineAction> {
        let name = self.name.trim().to_string();
        let driver = self.driver.trim().to_string();

        match self.kind {
            FormKind::CreateVolume => {
                Some(EngineAction::CreateVolume { name, driver })
            },
            FormKind::CreateNetwork => {
                Some(EngineAction::CreateNetwork { name, driver })
            },
            FormKind::Pull => None,
        }
    }

    fn create(kind: FormKind, engine: EngineInfo, driver: &str) -> Self {
        Self {
            kind,
            engine,
            name: String::new(),
            tag: String::new(),
            driver: driver.to_string(),
            progress: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, PullProgress, fixtures};

    use super::*;

    fn docker() -> EngineInfo {
        fixtures::engine(EngineKind::Docker)
    }

    #[test]
    fn each_form_starts_with_its_defaults() {
        let pull = Form::pull(docker(), "", "");
        let volume = Form::create_volume(docker());
        let network = Form::create_network(docker());

        assert_eq!(pull.value(FormField::Tag), DEFAULT_TAG);
        assert_eq!(pull.value(FormField::Name), "");
        assert_eq!(volume.value(FormField::Driver), "local");
        assert_eq!(network.value(FormField::Driver), "bridge");
        assert_eq!(pull.fields(), &[FormField::Name, FormField::Tag]);
        assert_eq!(volume.fields(), &[FormField::Name, FormField::Driver]);
        assert_eq!(pull.engine(), &docker());
    }

    #[test]
    fn a_pull_needs_an_image_name_and_a_network_needs_a_name() {
        let mut pull = Form::pull(docker(), "", "");
        let volume = Form::create_volume(docker());
        let mut network = Form::create_network(docker());
        assert!(!pull.can_submit());
        assert!(volume.can_submit(), "the engine names the volume");
        assert!(!network.can_submit());

        pull.set_value(FormField::Name, String::from("nginx"));
        network.set_value(FormField::Name, String::from("otty-fx-net2"));

        assert!(pull.can_submit());
        assert!(network.can_submit());
    }

    #[test]
    fn a_blank_driver_or_tag_blocks_the_form() {
        let mut volume = Form::create_volume(docker());
        let mut pull = Form::pull(docker(), "nginx", "");

        volume.set_value(FormField::Driver, String::from(" "));
        pull.set_value(FormField::Tag, String::new());

        assert!(!volume.can_submit());
        assert!(!pull.can_submit());
    }

    #[test]
    fn a_pull_names_its_image_and_tag() {
        let pull = Form::pull(docker(), " nginx ", " 1.27-alpine ");

        assert_eq!(pull.image(), "nginx");
        assert_eq!(pull.tag(), "1.27-alpine");
        assert_eq!(pull.reference(), "nginx:1.27-alpine");
    }

    #[test]
    fn a_running_pull_keeps_its_fields_and_takes_progress() {
        let mut pull = Form::pull(docker(), "nginx", "latest");
        let mut volume = Form::create_volume(docker());

        pull.start_pull();
        pull.set_value(FormField::Name, String::from("redis"));
        pull.set_progress(fixtures::pull_progress());
        volume.set_progress(fixtures::pull_progress());

        assert!(pull.is_pulling());
        assert!(!pull.can_submit());
        assert_eq!(pull.image(), "nginx");
        assert_eq!(pull.progress(), Some(&fixtures::pull_progress()));
        assert!(!volume.is_pulling());
        assert_eq!(volume.progress(), None);
        assert_eq!(
            Form::pull(docker(), "nginx", "latest").progress(),
            None::<&PullProgress>
        );
    }

    #[test]
    fn each_form_names_the_engine_call_it_makes() {
        let mut volume = Form::create_volume(docker());
        volume.set_value(FormField::Name, String::from(" otty-fx-vol "));
        let mut network = Form::create_network(docker());
        network.set_value(FormField::Name, String::from("otty-fx-net2"));

        assert_eq!(
            volume.create_action(),
            Some(EngineAction::CreateVolume {
                name: String::from("otty-fx-vol"),
                driver: String::from("local"),
            })
        );
        assert_eq!(
            network.create_action(),
            Some(EngineAction::CreateNetwork {
                name: String::from("otty-fx-net2"),
                driver: String::from("bridge"),
            })
        );
        assert_eq!(Form::pull(docker(), "nginx", "").create_action(), None);
    }
}
