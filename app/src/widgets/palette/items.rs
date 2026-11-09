use otty_ui_kit::components::palette_list::PaletteItem;
use otty_ui_kit::components::target_dot::TargetKind;

use super::reducer::{PaletteCtx, PaletteLaunch, PaletteSession};
use super::types::PaletteTarget;
use crate::commands::AppCommand;
use crate::domain::mode::Mode;
use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;

/// Build every palette entry: open sessions, then launches, then the app
/// commands that can run now ([`AppCommand::is_available`]). Every mode can
/// be switched to, pinned or not.
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

/// The app commands that can run now, each with its shortcut when it has
/// one.
fn action_items(ctx: &PaletteCtx<'_>) -> Vec<PaletteItem<PaletteTarget>> {
    let mut actions = vec![
        (
            AppCommand::NewTerminal,
            Key::PaletteNewTerminal,
            Some(AppShortcut::NewTerminal),
        ),
        (AppCommand::NewLaunch, Key::PaletteNewLaunch, None),
        (AppCommand::NewFolder, Key::PaletteNewFolder, None),
        (
            AppCommand::OpenSettings,
            Key::SettingsTitle,
            Some(AppShortcut::Settings),
        ),
        (
            AppCommand::SplitPane,
            Key::PaletteSplitPane,
            Some(AppShortcut::SplitPane),
        ),
        (
            AppCommand::CloseSession,
            Key::CloseSession,
            Some(AppShortcut::CloseSession),
        ),
        (
            AppCommand::ToggleLeftPanel,
            Key::PaletteToggleLeftPanel,
            Some(AppShortcut::LeftPanel),
        ),
        (
            AppCommand::ToggleFilesPanel,
            Key::PaletteToggleFilesPanel,
            Some(AppShortcut::FilesPanel),
        ),
        (
            AppCommand::ToggleFocusMode,
            Key::WindowFocusMode,
            Some(AppShortcut::FocusMode),
        ),
    ];
    actions.extend(Mode::ALL.into_iter().map(|mode| {
        let shortcut = ctx
            .pinned_modes
            .iter()
            .position(|pinned| *pinned == mode)
            .and_then(AppShortcut::for_mode_position);

        (
            AppCommand::SwitchMode(mode),
            switch_mode_title(mode),
            shortcut,
        )
    }));

    let group = i18n::t(Key::PaletteGroupActions);
    actions
        .into_iter()
        .filter(|(command, _, _)| command.is_available(&ctx.commands))
        .map(|(command, title, shortcut)| {
            let item = PaletteItem::new(
                PaletteTarget::Command(command),
                group,
                i18n::t(title),
            );
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
        Mode::Containers => Key::PaletteSwitchToContainersMode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandCtx;

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
            pinned_modes: vec![Mode::Terminal, Mode::Containers],
            commands: CommandCtx {
                is_terminal_mode: true,
                has_active_tab: false,
                has_focused_pane: false,
            },
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
            commands: CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: true,
            },
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
                    PaletteTarget::Command(AppCommand::NewTerminal),
                    Key::PaletteNewTerminal,
                    Some(AppShortcut::NewTerminal),
                ),
                action(
                    PaletteTarget::Command(AppCommand::NewLaunch),
                    Key::PaletteNewLaunch,
                    None
                ),
                action(
                    PaletteTarget::Command(AppCommand::NewFolder),
                    Key::PaletteNewFolder,
                    None
                ),
                action(
                    PaletteTarget::Command(AppCommand::OpenSettings),
                    Key::SettingsTitle,
                    Some(AppShortcut::Settings),
                ),
                action(
                    PaletteTarget::Command(AppCommand::SplitPane),
                    Key::PaletteSplitPane,
                    Some(AppShortcut::SplitPane),
                ),
                action(
                    PaletteTarget::Command(AppCommand::CloseSession),
                    Key::CloseSession,
                    Some(AppShortcut::CloseSession),
                ),
                action(
                    PaletteTarget::Command(AppCommand::ToggleLeftPanel),
                    Key::PaletteToggleLeftPanel,
                    Some(AppShortcut::LeftPanel),
                ),
                action(
                    PaletteTarget::Command(AppCommand::ToggleFilesPanel),
                    Key::PaletteToggleFilesPanel,
                    Some(AppShortcut::FilesPanel),
                ),
                action(
                    PaletteTarget::Command(AppCommand::ToggleFocusMode),
                    Key::WindowFocusMode,
                    Some(AppShortcut::FocusMode),
                ),
                action(
                    PaletteTarget::Command(AppCommand::SwitchMode(
                        Mode::Terminal
                    )),
                    Key::PaletteSwitchToTerminalMode,
                    Some(AppShortcut::TerminalMode),
                ),
                action(
                    PaletteTarget::Command(AppCommand::SwitchMode(
                        Mode::Containers
                    )),
                    Key::PaletteSwitchToContainersMode,
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
                PaletteTarget::Command(AppCommand::NewTerminal),
                PaletteTarget::Command(AppCommand::NewLaunch),
                PaletteTarget::Command(AppCommand::NewFolder),
                PaletteTarget::Command(AppCommand::OpenSettings),
                PaletteTarget::Command(AppCommand::ToggleLeftPanel),
                PaletteTarget::Command(AppCommand::ToggleFilesPanel),
                PaletteTarget::Command(AppCommand::ToggleFocusMode),
                PaletteTarget::Command(AppCommand::SwitchMode(Mode::Terminal)),
                PaletteTarget::Command(AppCommand::SwitchMode(
                    Mode::Containers
                )),
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
                    PaletteTarget::Command(AppCommand::SwitchMode(
                        Mode::Terminal
                    )),
                    group,
                    i18n::t(Key::PaletteSwitchToTerminalMode),
                )
                .with_shortcut(AppShortcut::TerminalMode.label()),
                PaletteItem::new(
                    PaletteTarget::Command(AppCommand::SwitchMode(
                        Mode::Containers
                    )),
                    group,
                    i18n::t(Key::PaletteSwitchToContainersMode),
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
            commands: CommandCtx {
                is_terminal_mode: true,
                has_active_tab: true,
                has_focused_pane: false,
            },
            ..empty_ctx()
        };

        let targets = targets(&palette_items(&ctx));

        assert!(
            targets.contains(&PaletteTarget::Command(AppCommand::CloseSession))
        );
        assert!(
            !targets.contains(&PaletteTarget::Command(AppCommand::SplitPane))
        );
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
            commands: CommandCtx {
                is_terminal_mode: false,
                has_active_tab: true,
                has_focused_pane: true,
            },
            ..empty_ctx()
        };

        let items = palette_items(&ctx);

        assert_eq!(
            targets(&items),
            vec![
                PaletteTarget::Session { tab_id: 1 },
                PaletteTarget::Command(AppCommand::NewTerminal),
                PaletteTarget::Command(AppCommand::NewLaunch),
                PaletteTarget::Command(AppCommand::OpenSettings),
                PaletteTarget::Command(AppCommand::ToggleLeftPanel),
                PaletteTarget::Command(AppCommand::ToggleFilesPanel),
                PaletteTarget::Command(AppCommand::ToggleFocusMode),
                PaletteTarget::Command(AppCommand::SwitchMode(Mode::Terminal)),
                PaletteTarget::Command(AppCommand::SwitchMode(
                    Mode::Containers
                )),
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
