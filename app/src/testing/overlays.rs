//! Open each overlay of the window the way the app does, for tests that need
//! one open. An opener leaves the other overlays as they are, so a test can
//! open several and see which one lies on top.

use iced::Point;

use crate::app::App;
use crate::dispatch::{self, AppEvent};
use crate::testing::shell_tab::open_shell_tab;
use crate::testing::window::reduce_quick_launch;
use crate::widgets::containers::child::actions::event::ActionsIntent;
use crate::widgets::containers::child::actions::types::RowAction;
use crate::widgets::containers::event::ContainersIntent;
use crate::widgets::containers::types::{Place, ResourceKind};
use crate::widgets::modes::event::ModesIntent;
use crate::widgets::palette::event::{PaletteEvent, PaletteIntent};
use crate::widgets::quick_launch::event::QuickLaunchIntent;
use crate::widgets::settings::event::SettingsIntent;
use crate::widgets::sidebar::event::SidebarIntent;
use crate::widgets::sidebar::types::HeaderMenu;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

/// Opens the password prompt of an SSH launch, as the launch does when the
/// server asks for one.
pub(crate) fn open_secret_prompt(app: &mut App) {
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::domain::quick_launch::{CommandSpec, QuickLaunch, SshCommand};
    use crate::widgets::quick_launch::types::{
        QuickLaunchSetupOutcome, SshSecretKind, SshSecretRequest,
    };

    let launch = QuickLaunch::new(
        String::from("laptop"),
        CommandSpec::Ssh {
            ssh: SshCommand::new(
                String::from("192.168.1.20"),
                22,
                Some(String::from("ubuntu")),
                None,
                Vec::new(),
            ),
        },
        ExitBehavior::default(),
    );

    reduce_quick_launch(
        app,
        QuickLaunchIntent::SetupCompleted(
            QuickLaunchSetupOutcome::SecretRequired {
                path: vec![String::from("laptop")],
                launch_id: 0,
                command: Box::new(launch),
                request: SshSecretRequest::new(
                    SshSecretKind::Password,
                    String::from("ubuntu@192.168.1.20:22"),
                    false,
                ),
            },
        ),
    );
}

/// Opens the Settings popup, as the gear does. The file read it asks for
/// is not run.
pub(crate) fn open_settings(app: &mut App) {
    let _ = app.widgets.settings.reduce(SettingsIntent::Open);
}

/// Opens the notifications list, as the bell does.
pub(crate) fn open_notification_list(app: &mut App) {
    use crate::widgets::notifications::event::{
        NotificationsEvent, NotificationsIntent,
    };

    let _ = dispatch::dispatch(
        app,
        AppEvent::Notifications(NotificationsEvent::Intent(
            NotificationsIntent::OpenList,
        )),
    );
}

/// Opens the command palette.
pub(crate) fn open_palette(app: &mut App) {
    let _ = dispatch::dispatch(
        app,
        AppEvent::Palette(PaletteEvent::Intent(PaletteIntent::Open)),
    );
}

/// Opens the All modes menu.
pub(crate) fn open_modes_menu(app: &mut App) {
    let _ = app.widgets.modes.reduce(ModesIntent::ToggleMenu);
}

/// Opens the New session menu of the Sessions header.
pub(crate) fn open_header_menu(app: &mut App) {
    let _ = app
        .widgets
        .sidebar
        .reduce(SidebarIntent::HeaderMenuOpen(HeaderMenu::Sessions));
}

/// Opens the Pull image dialog over the images of the fixture Docker engine.
/// `app` is in Containers mode with the fixture lists loaded, as
/// [`crate::testing::window::containers_app_with_lists`] makes it.
pub(crate) fn open_create_dialog(app: &mut App) {
    let engine =
        otty_containers::fixtures::engine(otty_containers::EngineKind::Docker);
    let _ = app
        .widgets
        .containers
        .reduce(ContainersIntent::Open(Place::new(
            engine.address().clone(),
            ResourceKind::Images,
        )));
    let _ = app.widgets.containers.reduce(ContainersIntent::OpenCreate);
}

/// Opens the menu of a container row, at a point inside the table. `app` is
/// as for [`open_create_dialog`].
pub(crate) fn open_row_menu(app: &mut App) {
    let _ = app
        .widgets
        .containers
        .reduce(ContainersIntent::OpenRowMenu {
            index: 5,
            position: Point::new(400.0, 200.0),
        });
}

/// Asks to confirm the removal of a container, as Remove in its row menu
/// does. `app` is as for [`open_create_dialog`].
pub(crate) fn open_confirm(app: &mut App) {
    open_row_menu(app);
    let _ = app.widgets.containers.reduce(ContainersIntent::Actions(
        ActionsIntent::Act(RowAction::Remove),
    ));
}

/// Opens a shell tab and the context menu of its only pane.
pub(crate) fn open_pane_menu(app: &mut App) {
    let tab_id = open_shell_tab(app);
    let (_, pane) = app
        .widgets
        .terminal_workspace
        .focused_pane(Some(tab_id))
        .expect("the shell tab has a pane");
    let terminal_id = app
        .widgets
        .terminal_workspace
        .tab(tab_id)
        .and_then(|tab| tab.terminals().keys().next().copied())
        .expect("the pane has a terminal");

    let _ = dispatch::dispatch(
        app,
        AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
            TerminalWorkspaceIntent::OpenContextMenu {
                tab_id,
                pane,
                terminal_id,
            },
        )),
    );
}

/// Opens the context menu of the Launches tree, on its background.
pub(crate) fn open_launch_menu(app: &mut App) {
    reduce_quick_launch(app, QuickLaunchIntent::BackgroundRightClicked);
}

/// Starts naming a new folder in the Launches tree.
pub(crate) fn start_inline_edit(app: &mut App) {
    reduce_quick_launch(app, QuickLaunchIntent::HeaderCreateFolder);
}
