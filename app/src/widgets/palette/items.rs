use otty_ui_kit::components::palette_list::PaletteItem;
use otty_ui_kit::components::target_dot::TargetKind;

use super::reducer::{PaletteCtx, PaletteLaunch, PaletteSession};
use super::types::PaletteTarget;
use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;
use crate::widgets::modes::types::Mode;

/// Build every palette entry: open sessions, then launches, then the app
/// actions. Split pane is listed only when there is a focused pane to
/// split, Close session only when a tab is active, and both only in Terminal
/// mode: elsewhere the tab is hidden and they do nothing. New folder is listed
/// only in Terminal mode too, because its name field lives in the Launches
/// panel. Every mode can be switched to, pinned or not.
pub(super) fn palette_items(
    ctx: &PaletteCtx<'_>,
) -> Vec<PaletteItem<PaletteTarget>> {
    let mut items = session_items(&ctx.sessions);
    items.extend(launch_items(&ctx.launches));
    items.extend(action_items(ctx));

    items
}

/// One entry per open tab, the machine name as detail.
fn session_items(
    sessions: &[PaletteSession<'_>],
) -> Vec<PaletteItem<PaletteTarget>> {
    let group = i18n::t(Key::SessionsPanelTitle);

    sessions
        .iter()
        .map(|session| {
            PaletteItem::new(
                PaletteTarget::Session {
                    tab_id: session.tab_id,
                },
                group,
                session.title,
            )
            .with_detail(session.machine)
            .with_target(session.target)
        })
        .collect()
}

/// One entry per launch, its folder path as detail (none at the top of
/// the tree).
fn launch_items(
    launches: &[PaletteLaunch<'_>],
) -> Vec<PaletteItem<PaletteTarget>> {
    let group = i18n::t(Key::LaunchesPanelTitle);
    let mut items = Vec::new();

    for launch in launches {
        let target = if launch.is_ssh {
            TargetKind::Ssh
        } else {
            TargetKind::Local
        };
        let folders = &launch.path[..launch.path.len().saturating_sub(1)];

        let mut item = PaletteItem::new(
            PaletteTarget::Launch {
                path: launch.path.clone(),
            },
            group,
            launch.title,
        )
        .with_target(target);
        if !folders.is_empty() {
            item = item.with_detail(folders.join("/"));
        }

        items.push(item);
    }

    items
}

/// The app actions, each with its shortcut when it has one.
fn action_items(ctx: &PaletteCtx<'_>) -> Vec<PaletteItem<PaletteTarget>> {
    let is_terminal_mode = ctx.active_mode == Mode::Terminal;
    let has_active_session =
        ctx.sessions.iter().any(|session| session.is_active);

    let mut actions = vec![
        (
            PaletteTarget::NewTerminal,
            Key::PaletteNewTerminal,
            Some(AppShortcut::NewTerminal),
        ),
        (PaletteTarget::NewLaunch, Key::PaletteNewLaunch, None),
    ];
    if is_terminal_mode {
        actions.push((PaletteTarget::NewFolder, Key::PaletteNewFolder, None));
    }
    actions.push((
        PaletteTarget::Settings,
        Key::SettingsTabTitle,
        Some(AppShortcut::Settings),
    ));
    if is_terminal_mode && ctx.can_split_pane {
        actions.push((
            PaletteTarget::SplitPane,
            Key::PaletteSplitPane,
            Some(AppShortcut::SplitPane),
        ));
    }
    if is_terminal_mode && has_active_session {
        actions.push((
            PaletteTarget::CloseSession,
            Key::CloseSession,
            Some(AppShortcut::CloseSession),
        ));
    }
    actions.extend([
        (
            PaletteTarget::ToggleLeftPanel,
            Key::PaletteToggleLeftPanel,
            Some(AppShortcut::LeftPanel),
        ),
        (
            PaletteTarget::ToggleFilesPanel,
            Key::PaletteToggleFilesPanel,
            Some(AppShortcut::FilesPanel),
        ),
        (
            PaletteTarget::ToggleFocusMode,
            Key::WindowFocusMode,
            Some(AppShortcut::FocusMode),
        ),
    ]);
    actions.extend(Mode::ALL.into_iter().map(|mode| {
        let shortcut = ctx
            .pinned_modes
            .iter()
            .position(|pinned| *pinned == mode)
            .and_then(AppShortcut::for_mode_position);

        (
            PaletteTarget::SwitchMode(mode),
            switch_mode_title(mode),
            shortcut,
        )
    }));

    let group = i18n::t(Key::PaletteGroupActions);
    actions
        .into_iter()
        .map(|(target, title, shortcut)| {
            let item = PaletteItem::new(target, group, i18n::t(title));
            match shortcut {
                Some(shortcut) => item.with_shortcut(shortcut.label()),
                None => item,
            }
        })
        .collect()
}

/// Title of the palette entry that switches to `mode`.
fn switch_mode_title(mode: Mode) -> Key {
    match mode {
        Mode::Terminal => Key::PaletteSwitchToTerminalMode,
        Mode::Docker => Key::PaletteSwitchToDockerMode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::modes::types::Mode;

    fn session(
        tab_id: u64,
        title: &'static str,
        machine: &'static str,
        target: TargetKind,
        is_active: bool,
    ) -> PaletteSession<'static> {
        PaletteSession {
            tab_id,
            title,
            machine,
            target,
            is_active,
        }
    }

    fn launch(
        path: &[&str],
        title: &'static str,
        is_ssh: bool,
    ) -> PaletteLaunch<'static> {
        PaletteLaunch {
            path: path.iter().map(|segment| String::from(*segment)).collect(),
            title,
            is_ssh,
        }
    }

    fn empty_ctx() -> PaletteCtx<'static> {
        PaletteCtx {
            sessions: Vec::new(),
            launches: Vec::new(),
            can_split_pane: false,
            pinned_modes: vec![Mode::Terminal, Mode::Docker],
            active_mode: Mode::Terminal,
        }
    }

    fn targets(items: &[PaletteItem<PaletteTarget>]) -> Vec<PaletteTarget> {
        items.iter().map(|item| item.key().clone()).collect()
    }

    #[test]
    fn sessions_name_their_machine_and_switch_to_their_tab() {
        let ctx = PaletteCtx {
            sessions: vec![
                session(1, "zsh  ~/otty", "fedora", TargetKind::Local, true),
                session(4, "htop", "laptop", TargetKind::Ssh, false),
            ],
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        let group = i18n::t(Key::SessionsPanelTitle);
        assert_eq!(
            items[0],
            PaletteItem::new(
                PaletteTarget::Session { tab_id: 1 },
                group,
                "zsh  ~/otty",
            )
            .with_detail("fedora")
            .with_target(TargetKind::Local)
        );
        assert_eq!(
            items[1],
            PaletteItem::new(
                PaletteTarget::Session { tab_id: 4 },
                group,
                "htop"
            )
            .with_detail("laptop")
            .with_target(TargetKind::Ssh)
        );
    }

    #[test]
    fn launches_show_their_folder_path_and_run_by_tree_path() {
        let ctx = PaletteCtx {
            launches: vec![
                launch(&["servers", "prod", "laptop"], "laptop", true),
                launch(&["htop"], "htop", false),
            ],
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        let group = i18n::t(Key::LaunchesPanelTitle);
        assert_eq!(
            items[0],
            PaletteItem::new(
                PaletteTarget::Launch {
                    path: vec![
                        String::from("servers"),
                        String::from("prod"),
                        String::from("laptop"),
                    ],
                },
                group,
                "laptop",
            )
            .with_detail("servers/prod")
            .with_target(TargetKind::Ssh)
        );
        assert_eq!(
            items[1],
            PaletteItem::new(
                PaletteTarget::Launch {
                    path: vec![String::from("htop")],
                },
                group,
                "htop",
            )
            .with_target(TargetKind::Local)
        );
    }

    #[test]
    fn with_an_active_terminal_every_action_is_listed_with_its_shortcut() {
        let ctx = PaletteCtx {
            sessions: vec![session(
                1,
                "zsh",
                "fedora",
                TargetKind::Local,
                true,
            )],
            can_split_pane: true,
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        let group = i18n::t(Key::PaletteGroupActions);
        let action = |target, key, shortcut: Option<AppShortcut>| {
            let item = PaletteItem::new(target, group, i18n::t(key));
            match shortcut {
                Some(shortcut) => item.with_shortcut(shortcut.label()),
                None => item,
            }
        };
        assert_eq!(
            items[1..],
            [
                action(
                    PaletteTarget::NewTerminal,
                    Key::PaletteNewTerminal,
                    Some(AppShortcut::NewTerminal),
                ),
                action(PaletteTarget::NewLaunch, Key::PaletteNewLaunch, None),
                action(PaletteTarget::NewFolder, Key::PaletteNewFolder, None),
                action(
                    PaletteTarget::Settings,
                    Key::SettingsTabTitle,
                    Some(AppShortcut::Settings),
                ),
                action(
                    PaletteTarget::SplitPane,
                    Key::PaletteSplitPane,
                    Some(AppShortcut::SplitPane),
                ),
                action(
                    PaletteTarget::CloseSession,
                    Key::CloseSession,
                    Some(AppShortcut::CloseSession),
                ),
                action(
                    PaletteTarget::ToggleLeftPanel,
                    Key::PaletteToggleLeftPanel,
                    Some(AppShortcut::LeftPanel),
                ),
                action(
                    PaletteTarget::ToggleFilesPanel,
                    Key::PaletteToggleFilesPanel,
                    Some(AppShortcut::FilesPanel),
                ),
                action(
                    PaletteTarget::ToggleFocusMode,
                    Key::WindowFocusMode,
                    Some(AppShortcut::FocusMode),
                ),
                action(
                    PaletteTarget::SwitchMode(Mode::Terminal),
                    Key::PaletteSwitchToTerminalMode,
                    Some(AppShortcut::TerminalMode),
                ),
                action(
                    PaletteTarget::SwitchMode(Mode::Docker),
                    Key::PaletteSwitchToDockerMode,
                    Some(AppShortcut::PinnedMode1),
                ),
            ]
        );
    }

    #[test]
    fn without_tabs_split_pane_and_close_session_are_left_out() {
        let items = palette_items(&empty_ctx());

        assert_eq!(
            targets(&items),
            vec![
                PaletteTarget::NewTerminal,
                PaletteTarget::NewLaunch,
                PaletteTarget::NewFolder,
                PaletteTarget::Settings,
                PaletteTarget::ToggleLeftPanel,
                PaletteTarget::ToggleFilesPanel,
                PaletteTarget::ToggleFocusMode,
                PaletteTarget::SwitchMode(Mode::Terminal),
                PaletteTarget::SwitchMode(Mode::Docker),
            ]
        );
    }

    #[test]
    fn an_unpinned_mode_can_be_switched_to_but_has_no_shortcut() {
        let ctx = PaletteCtx {
            pinned_modes: vec![Mode::Terminal],
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        let group = i18n::t(Key::PaletteGroupActions);
        assert_eq!(
            items[items.len() - 2..],
            [
                PaletteItem::new(
                    PaletteTarget::SwitchMode(Mode::Terminal),
                    group,
                    i18n::t(Key::PaletteSwitchToTerminalMode),
                )
                .with_shortcut(AppShortcut::TerminalMode.label()),
                PaletteItem::new(
                    PaletteTarget::SwitchMode(Mode::Docker),
                    group,
                    i18n::t(Key::PaletteSwitchToDockerMode),
                ),
            ]
        );
    }

    #[test]
    fn a_non_terminal_active_tab_can_be_closed_but_not_split() {
        let ctx = PaletteCtx {
            sessions: vec![session(
                2,
                "Settings",
                "fedora",
                TargetKind::Local,
                true,
            )],
            can_split_pane: false,
            ..empty_ctx()
        };

        let targets = targets(&palette_items(&ctx));

        assert!(targets.contains(&PaletteTarget::CloseSession));
        assert!(!targets.contains(&PaletteTarget::SplitPane));
    }

    #[test]
    fn outside_terminal_mode_split_pane_close_session_and_new_folder_are_left_out()
     {
        let ctx = PaletteCtx {
            sessions: vec![session(
                1,
                "zsh",
                "fedora",
                TargetKind::Local,
                true,
            )],
            can_split_pane: true,
            active_mode: Mode::Docker,
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        assert_eq!(
            targets(&items),
            vec![
                PaletteTarget::Session { tab_id: 1 },
                PaletteTarget::NewTerminal,
                PaletteTarget::NewLaunch,
                PaletteTarget::Settings,
                PaletteTarget::ToggleLeftPanel,
                PaletteTarget::ToggleFilesPanel,
                PaletteTarget::ToggleFocusMode,
                PaletteTarget::SwitchMode(Mode::Terminal),
                PaletteTarget::SwitchMode(Mode::Docker),
            ]
        );
    }

    #[test]
    fn groups_come_in_order_sessions_launches_actions() {
        let ctx = PaletteCtx {
            sessions: vec![session(
                1,
                "zsh",
                "fedora",
                TargetKind::Local,
                false,
            )],
            launches: vec![launch(&["htop"], "htop", false)],
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        let mut groups: Vec<&str> = Vec::new();
        for item in &items {
            if groups.last() != Some(&item.group()) {
                groups.push(item.group());
            }
        }
        assert_eq!(
            groups,
            vec![
                i18n::t(Key::SessionsPanelTitle),
                i18n::t(Key::LaunchesPanelTitle),
                i18n::t(Key::PaletteGroupActions),
            ]
        );
    }
}
