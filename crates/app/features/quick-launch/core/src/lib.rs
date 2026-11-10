//! Quick launch of the OTTY app: the tree of saved launches in the Launches
//! section of the left panel, the launch editor in a tab of the main area,
//! the context menu of the tree and the prompt that asks an SSH launch for
//! its password or key passphrase.
//!
//! The feature keeps the tree and decides everything about it. What needs
//! the outside world comes back as a [`QuickLaunchOutcome`]:
//! `otty-app-quick-launch-io` reads and writes the launches file, checks a
//! launch before it runs and builds its terminal settings, and the binary
//! hands a ready launch to the tabs. The
//! answers come back as [`QuickLaunchIntent`]s. Which machine a launch runs
//! on is decided here: a ready launch reports its [`TabMachine`].
//!
//! # Example
//!
//! A click on a launch asks the binary to check it; the answer opens its
//! tab on the SSH host it connects to.
//!
//! ```
//! use iced::{Point, Size};
//! use otty_app_domain::tab::TabMachine;
//! use otty_app_quick_launch::{
//!     QuickLaunchCtx, QuickLaunchFile, QuickLaunchIntent, QuickLaunchOutcome,
//!     QuickLaunchSetupOutcome, QuickLaunches,
//! };
//! use otty_app_slots::Feature;
//! use otty_ui_term::settings::Settings;
//!
//! let saved: QuickLaunchFile = serde_json::from_str(
//!     r#"{"version": 1, "root": {"title": "Root", "expanded": true,
//!         "children": [{"kind": "Command", "title": "laptop",
//!           "spec": {"type": "Ssh", "ssh": {"host": "192.168.1.20",
//!             "port": 22, "user": null, "identity_file": null,
//!             "extra_args": []}}}]}}"#,
//! )
//! .expect("the launches should parse");
//! let ctx = QuickLaunchCtx {
//!     cursor: Point::ORIGIN,
//!     is_panel_resizing: false,
//!     screen_size: Size::new(1024.0, 768.0),
//! };
//! let mut launches = QuickLaunches::new(saved);
//!
//! let asked = launches.reduce(
//!     QuickLaunchIntent::LaunchNode {
//!         path: vec![String::from("laptop")],
//!     },
//!     ctx,
//! );
//! let [QuickLaunchOutcome::PrepareLaunch(request)] = asked.as_slice() else {
//!     panic!("the launch should be checked first");
//! };
//!
//! let ready = launches.reduce(
//!     QuickLaunchIntent::SetupCompleted(QuickLaunchSetupOutcome::Prepared {
//!         path: request.path().to_vec(),
//!         launch_id: request.launch_id(),
//!         command: Box::new(request.command().clone()),
//!         settings: Box::new(Settings::default()),
//!     }),
//!     ctx,
//! );
//! assert!(matches!(
//!     ready.as_slice(),
//!     [QuickLaunchOutcome::LaunchReady { machine, .. }]
//!         if *machine == TabMachine::Ssh(String::from("192.168.1.20"))
//! ));
//! ```

mod constants;
mod ctx;
mod errors;
mod event;
mod model;
mod reducer;
mod state;
mod types;
mod view;
mod wizard_draft;

use std::time::Duration;

use constants::QUICK_LAUNCHES_TICK_MS;
pub use ctx::QuickLaunchCtx;
pub use event::{LaunchRequest, QuickLaunchIntent, QuickLaunchOutcome};
use iced::{Element, Subscription};
use model::{
    ContextMenuViewModel, SecretPromptViewModel, SidebarTreeViewModel,
    WizardFormViewModel,
};
#[cfg(doc)]
use otty_app_domain::tab::TabMachine;
use otty_app_i18n::{self as i18n, Key};
use otty_app_slots::{
    Feature, LeftPanel, LeftPanelSection, MainArea, MainContent, Overlay,
    Overlays, SectionState,
};
use otty_app_theme::theme::ThemeProps;
use state::QuickLaunchState;
pub use types::{
    ContextMenuAction, LaunchSummary, QuickLaunchFile, QuickLaunchSetupOutcome,
    QuickLaunchType, SshSecretKind, SshSecretRequest,
};
use view::{
    context_menu, new_launch_menu, secret_prompt, sidebar_panel, wizard_form,
};

/// The screens quick launch shows in the main area: the launch editor of a
/// tab. Two editors never share their scrolling, and an editor drawn again
/// keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickLaunchScreen {
    /// The launch editor in the tab with this id.
    Editor(u64),
    /// The tab in front has no editor: a line says so.
    MissingEditor,
}

/// The quick launch feature: the tree of saved launches, the launch editors
/// of the open editor tabs, the launches being prepared, and the open
/// context menu, name field or SSH secret prompt.
///
/// # Example
///
/// ```
/// use otty_app_quick_launch::{QuickLaunchFile, QuickLaunches};
/// use otty_app_slots::Feature;
///
/// let launches = QuickLaunches::new(QuickLaunchFile::default());
///
/// assert!(launches.launches().is_empty());
/// // Nothing is checked or waits to be saved, so no tick runs.
/// assert_eq!(launches.subscription().units(), 0);
/// ```
pub struct QuickLaunches {
    state: QuickLaunchState,
}

impl QuickLaunches {
    /// Every launch in the tree, folders left out, in tree order, each with
    /// the path that runs it. The palette lists them.
    pub fn launches(&self) -> Vec<LaunchSummary<'_>> {
        self.state.data().launches()
    }

    /// The launches at the top level of the tree, in tree order, each with
    /// the path that runs it. The New session menu lists them.
    pub fn top_level_launches(&self) -> Vec<LaunchSummary<'_>> {
        self.state.data().top_level_launches()
    }

    /// Return whether the context menu of the tree is open.
    pub fn has_context_menu(&self) -> bool {
        self.state.context_menu().is_some()
    }

    /// Return whether the SSH secret prompt is open.
    pub fn has_secret_prompt(&self) -> bool {
        self.state.secret_prompt().is_some()
    }

    /// Return whether a name is being typed in the tree: a new folder or a
    /// rename.
    pub fn has_inline_edit(&self) -> bool {
        self.state.inline_edit().is_some()
    }

    /// Height of the Launches menu, so the frame keeps it inside the window.
    pub fn launches_menu_height(&self) -> f32 {
        new_launch_menu::menu_height()
    }

    /// Create the feature with the launches read at start.
    pub fn new(saved: QuickLaunchFile) -> Self {
        Self {
            state: QuickLaunchState::with_data(saved),
        }
    }

    /// The Launches menu under the chevron of the section header: New
    /// launch and New folder. The frame keeps it open and anchors it.
    pub fn launches_menu<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Element<'a, QuickLaunchIntent> {
        new_launch_menu::view(theme)
    }

    /// The SSH secret prompt as a dialog, while a launch waits for its
    /// password or key passphrase. It is the top layer of the window, over
    /// Settings and the palette, so it is not the feature's
    /// [`Overlays::overlay`], which lies among the overlays of Terminal
    /// mode.
    pub fn secret_prompt<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, QuickLaunchIntent>> {
        let vm = SecretPromptViewModel::from_state(&self.state)?;

        Some(Overlay::dialog(secret_prompt::view(vm, theme)))
    }

    /// How often the feature wants [`QuickLaunchIntent::Tick`]: while a
    /// launch is being prepared (its row blinks) or the tree waits to be
    /// saved. `None` when it needs no tick.
    fn tick_period(&self) -> Option<Duration> {
        let needs_tick = self.state.has_active_launches()
            || self.state.is_dirty()
            || self.state.is_persist_in_flight();

        needs_tick.then(|| Duration::from_millis(QUICK_LAUNCHES_TICK_MS))
    }
}

impl Feature for QuickLaunches {
    type Intent = QuickLaunchIntent;
    type Outcome = QuickLaunchOutcome;
    type Ctx<'a> = QuickLaunchCtx;

    fn reduce(
        &mut self,
        intent: QuickLaunchIntent,
        ctx: QuickLaunchCtx,
    ) -> Vec<QuickLaunchOutcome> {
        reducer::reduce(&mut self.state, intent, &ctx)
    }

    /// The tick while a launch is being prepared or the tree waits to be
    /// saved: each tick is a [`QuickLaunchIntent::Tick`].
    fn subscription(&self) -> Subscription<QuickLaunchIntent> {
        let Some(every) = self.tick_period() else {
            return Subscription::none();
        };

        iced::time::every(every).map(|_| QuickLaunchIntent::Tick)
    }
}

impl LeftPanel for QuickLaunches {
    /// The Launches section: its header with the New launch button, and the
    /// tree of launches with a name field while one is typed.
    fn left_panel<'a>(
        &'a self,
        theme: ThemeProps<'a>,
        section: SectionState,
    ) -> LeftPanelSection<'a, QuickLaunchIntent> {
        let vm = SidebarTreeViewModel::from_state(&self.state);
        let content_height = sidebar_panel::content_height(&vm);

        let content = sidebar_panel::view(sidebar_panel::SidebarPanelProps {
            vm,
            theme,
            section,
        });

        LeftPanelSection::new(content, content_height)
    }
}

impl MainArea for QuickLaunches {
    type Screen = QuickLaunchScreen;

    /// The launch editor of the tab in front, or a line that says it is
    /// missing.
    fn main_area<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> MainContent<'a, QuickLaunchIntent, QuickLaunchScreen> {
        let editor = self.state.wizard().front_tab_id().and_then(|tab_id| {
            WizardFormViewModel::from_state(&self.state, tab_id)
        });

        match editor {
            Some(vm) => MainContent::Screen {
                id: QuickLaunchScreen::Editor(vm.tab_id),
                content: wizard_form::view(vm, theme),
            },
            None => MainContent::Screen {
                id: QuickLaunchScreen::MissingEditor,
                content: wizard_form::missing_editor(i18n::t(
                    Key::QuickLaunchEditorNotInitialized,
                )),
            },
        }
    }
}

impl Overlays for QuickLaunches {
    /// The context menu of the tree, as a menu among the overlays of
    /// Terminal mode. The SSH secret prompt is drawn on top of every layer
    /// instead (see [`QuickLaunches::secret_prompt`]).
    fn overlay<'a>(
        &'a self,
        theme: ThemeProps<'a>,
    ) -> Option<Overlay<'a, QuickLaunchIntent>> {
        let vm = ContextMenuViewModel::from_state(&self.state)?;

        Some(Overlay::menu(context_menu::view(vm, theme)))
    }
}

#[cfg(test)]
mod tests {
    use iced::{Point, Size};
    use iced_test::simulator;
    use otty_app_slots::{OverlayKind, SectionMessage};
    use otty_app_theme::theme::ThemeManager;

    use super::*;

    fn ctx() -> QuickLaunchCtx {
        QuickLaunchCtx {
            cursor: Point::new(20.0, 20.0),
            is_panel_resizing: false,
            screen_size: Size::new(1024.0, 768.0),
        }
    }

    /// A "servers" folder with an SSH launch, then a local launch at the
    /// top.
    fn saved() -> QuickLaunchFile {
        serde_json::from_str(
            r#"{"version": 1, "root": {"title": "Root", "expanded": true,
                "children": [
                  {"kind": "Folder", "title": "servers", "expanded": true,
                   "children": [{"kind": "Command", "title": "laptop",
                     "spec": {"type": "Ssh", "ssh": {"host": "192.168.1.20",
                       "port": 22, "user": "ubuntu", "identity_file": null,
                       "extra_args": []}}}]},
                  {"kind": "Command", "title": "htop",
                   "spec": {"type": "Custom", "custom": {"program": "htop",
                     "args": [], "env": [], "working_directory": null}}}
                ]}}"#,
        )
        .expect("the launches should parse")
    }

    fn laptop() -> Vec<String> {
        vec![String::from("servers"), String::from("laptop")]
    }

    /// Run the launch `laptop`; the check answers that it needs a password.
    fn ask_for_a_password(launches: &mut QuickLaunches) {
        let asked = launches
            .reduce(QuickLaunchIntent::LaunchNode { path: laptop() }, ctx());
        let [QuickLaunchOutcome::PrepareLaunch(request), ..] = asked.as_slice()
        else {
            panic!("the launch should be checked first");
        };
        let answer = QuickLaunchSetupOutcome::SecretRequired {
            path: laptop(),
            launch_id: request.launch_id(),
            command: Box::new(request.command().clone()),
            request: SshSecretRequest::new(
                SshSecretKind::Password,
                String::from("ubuntu@192.168.1.20:22"),
                false,
            ),
        };

        let _focus =
            launches.reduce(QuickLaunchIntent::SetupCompleted(answer), ctx());
    }

    #[test]
    fn the_launch_lists_follow_the_tree() {
        let launches = QuickLaunches::new(saved());

        let all: Vec<(Vec<String>, &str, bool)> = launches
            .launches()
            .into_iter()
            .map(|launch| {
                (launch.path().to_vec(), launch.title(), launch.is_ssh())
            })
            .collect();
        let top: Vec<Vec<String>> = launches
            .top_level_launches()
            .into_iter()
            .map(LaunchSummary::into_path)
            .collect();

        assert_eq!(
            all,
            vec![
                (laptop(), "laptop", true),
                (vec![String::from("htop")], "htop", false),
            ]
        );
        assert_eq!(top, vec![vec![String::from("htop")]]);
    }

    /// The timer runs while a launch is checked and while the tree waits to
    /// be saved, and stops after the save.
    #[test]
    fn the_tick_runs_while_a_launch_is_checked_or_a_save_waits() {
        let mut launches = QuickLaunches::new(saved());
        let idle = launches.tick_period();
        let timers_idle = launches.subscription().units();

        let _check = launches.reduce(
            QuickLaunchIntent::LaunchNode {
                path: vec![String::from("htop")],
            },
            ctx(),
        );
        let checking = launches.tick_period();
        let timers_checking = launches.subscription().units();
        let _folder = launches.reduce(
            QuickLaunchIntent::LaunchNode {
                path: vec![String::from("servers")],
            },
            ctx(),
        );
        let _save = launches.reduce(QuickLaunchIntent::Tick, ctx());
        let _saved =
            launches.reduce(QuickLaunchIntent::PersistCompleted, ctx());
        let _answer = launches.reduce(
            QuickLaunchIntent::SetupCompleted(
                QuickLaunchSetupOutcome::Canceled {
                    path: vec![String::from("htop")],
                    launch_id: 1,
                },
            ),
            ctx(),
        );

        let tick = Some(Duration::from_millis(QUICK_LAUNCHES_TICK_MS));
        assert_eq!(idle, None);
        assert_eq!(timers_idle, 0);
        assert_eq!(checking, tick);
        assert_eq!(timers_checking, 1);
        assert_eq!(launches.tick_period(), None);
        assert_eq!(launches.subscription().units(), 0);
    }

    /// The section draws from the feature's own state; the frame's section
    /// state hides the rows of a closed section, and its title belongs to
    /// the frame.
    #[test]
    fn the_left_panel_is_the_launches_section() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let launches = QuickLaunches::new(saved());

        let open = launches.left_panel(theme, SectionState::new(true, false));
        let height = open.content_height();
        let mut ui = simulator(open.into_content());

        assert_eq!(height, 3.0 * otty_ui_kit::size::ROW);
        assert!(ui.find("laptop").is_ok());
        ui.click(i18n::t(Key::LaunchesPanelTitle))
            .expect("the title should show");
        let pressed: Vec<SectionMessage<QuickLaunchIntent>> =
            ui.into_messages().collect();
        assert!(matches!(pressed.as_slice(), [SectionMessage::Toggle]));

        let closed =
            launches.left_panel(theme, SectionState::new(false, false));
        let mut ui = simulator(closed.into_content());
        assert!(ui.find("laptop").is_err());
    }

    /// The main area shows the editor of the tab in front; a tab without
    /// one says so.
    #[test]
    fn the_main_area_is_the_editor_of_the_tab_in_front() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut launches = QuickLaunches::new(saved());
        let is_missing = matches!(
            launches.main_area(theme),
            MainContent::Screen {
                id: QuickLaunchScreen::MissingEditor,
                ..
            }
        );
        let _editor = launches.reduce(
            QuickLaunchIntent::WizardInitializeCreate {
                tab_id: 4,
                parent_path: Vec::new(),
            },
            ctx(),
        );

        let MainContent::Screen { id, content } = launches.main_area(theme)
        else {
            panic!("the editor is a screen");
        };
        let mut ui = simulator(content);

        assert!(is_missing);
        assert_eq!(id, QuickLaunchScreen::Editor(4));
        assert!(ui.find(i18n::t(Key::WizardTitleNew)).is_ok());
    }

    /// The context menu is a menu among the mode overlays; the SSH prompt
    /// is a dialog the frame puts on top of every layer.
    #[test]
    fn the_context_menu_is_a_menu_and_the_secret_prompt_a_dialog() {
        let themes = ThemeManager::new();
        let theme = ThemeProps::new(themes.current());
        let mut launches = QuickLaunches::new(saved());
        let nothing = (
            launches.overlay(theme).is_none(),
            launches.secret_prompt(theme).is_none(),
        );

        let _keyboard =
            launches.reduce(QuickLaunchIntent::BackgroundRightClicked, ctx());
        let menu = launches.overlay(theme).map(|menu| menu.kind());
        let has_menu = launches.has_context_menu();
        ask_for_a_password(&mut launches);
        let prompt = launches.secret_prompt(theme).map(|prompt| prompt.kind());

        assert_eq!(nothing, (true, true));
        assert_eq!(menu, Some(OverlayKind::Menu));
        assert_eq!(prompt, Some(OverlayKind::Dialog));
        assert!(has_menu);
        assert!(launches.has_secret_prompt());
    }

    #[test]
    fn the_launches_menu_offers_new_launch_and_new_folder() {
        let themes = ThemeManager::new();
        let launches = QuickLaunches::new(saved());
        let mut ui = simulator(
            launches.launches_menu(ThemeProps::new(themes.current())),
        );

        ui.click(i18n::t(Key::MenuNewFolder))
            .expect("New folder should show");

        let pressed: Vec<QuickLaunchIntent> = ui.into_messages().collect();
        assert!(matches!(
            pressed.as_slice(),
            [QuickLaunchIntent::HeaderCreateFolder]
        ));
        assert!(launches.launches_menu_height() > 0.0);
    }

    #[test]
    fn a_name_typed_in_the_tree_is_an_inline_edit() {
        let mut launches = QuickLaunches::new(saved());

        let _focus =
            launches.reduce(QuickLaunchIntent::HeaderCreateFolder, ctx());

        assert!(launches.has_inline_edit());
    }
}
