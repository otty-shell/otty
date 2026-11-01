use otty_ui_kit::components::palette_list::PaletteItem;
use otty_ui_kit::components::target_dot::TargetKind;

use super::reducer::PaletteCtx;
use super::types::PaletteTarget;
use crate::i18n::{self, Key};
use crate::shortcuts::AppShortcut;
use crate::widgets::quick_launch::model::LaunchViewModel;
use crate::widgets::tabs::model::TabsViewModel;

/// Build every palette entry from the other widgets' view models: open
/// sessions, then launches, then the app actions. Split pane is listed only
/// when there is a focused pane to split, Close session only when a tab is
/// active.
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
    sessions: &TabsViewModel<'_>,
) -> Vec<PaletteItem<PaletteTarget>> {
    let group = i18n::t(Key::SessionsPanelTitle);
    let mut items = Vec::new();

    for machine in &sessions.groups {
        for session in &machine.sessions {
            items.push(
                PaletteItem::new(
                    PaletteTarget::Session {
                        tab_id: session.tab_id,
                    },
                    group,
                    session.title,
                )
                .with_detail(machine.name)
                .with_target(machine.target),
            );
        }
    }

    items
}

/// One entry per launch, its folder path as detail (none at the top of
/// the tree).
fn launch_items(
    launches: &[LaunchViewModel<'_>],
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
    let has_active_session = ctx
        .sessions
        .groups
        .iter()
        .flat_map(|machine| &machine.sessions)
        .any(|session| session.is_active);

    let mut actions = vec![
        (
            PaletteTarget::NewTerminal,
            Key::PaletteNewTerminal,
            Some(AppShortcut::NewTerminal),
        ),
        (PaletteTarget::NewLaunch, Key::PaletteNewLaunch, None),
        (PaletteTarget::NewFolder, Key::PaletteNewFolder, None),
        (
            PaletteTarget::Settings,
            Key::SettingsTabTitle,
            Some(AppShortcut::Settings),
        ),
    ];
    if ctx.can_split_pane {
        actions.push((
            PaletteTarget::SplitPane,
            Key::PaletteSplitPane,
            Some(AppShortcut::SplitPane),
        ));
    }
    if has_active_session {
        actions.push((
            PaletteTarget::CloseSession,
            Key::PaletteCloseSession,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::tabs::model::{
        SessionGroupViewModel, SessionViewModel,
    };

    fn session(
        tab_id: u64,
        title: &'static str,
        is_active: bool,
    ) -> SessionViewModel<'static> {
        SessionViewModel {
            tab_id,
            title,
            state: None,
            is_active,
        }
    }

    fn sessions(
        groups: Vec<SessionGroupViewModel<'static>>,
    ) -> TabsViewModel<'static> {
        let session_count = groups.iter().map(|g| g.sessions.len()).sum();

        TabsViewModel {
            groups,
            session_count,
        }
    }

    fn launch(path: &[&str], is_ssh: bool) -> LaunchViewModel<'static> {
        LaunchViewModel {
            path: path.iter().map(|segment| String::from(*segment)).collect(),
            title: "",
            is_ssh,
        }
    }

    fn empty_ctx() -> PaletteCtx<'static> {
        PaletteCtx {
            sessions: sessions(Vec::new()),
            launches: Vec::new(),
            can_split_pane: false,
        }
    }

    fn targets(items: &[PaletteItem<PaletteTarget>]) -> Vec<PaletteTarget> {
        items.iter().map(|item| item.key().clone()).collect()
    }

    #[test]
    fn sessions_name_their_machine_and_switch_to_their_tab() {
        let ctx = PaletteCtx {
            sessions: sessions(vec![
                SessionGroupViewModel {
                    name: "fedora",
                    target: TargetKind::Local,
                    sessions: vec![session(1, "zsh  ~/otty", true)],
                },
                SessionGroupViewModel {
                    name: "laptop",
                    target: TargetKind::Ssh,
                    sessions: vec![session(4, "htop", false)],
                },
            ]),
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
        let mut laptop = launch(&["servers", "prod", "laptop"], true);
        laptop.title = "laptop";
        let mut htop = launch(&["htop"], false);
        htop.title = "htop";
        let ctx = PaletteCtx {
            launches: vec![laptop, htop],
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
            sessions: sessions(vec![SessionGroupViewModel {
                name: "fedora",
                target: TargetKind::Local,
                sessions: vec![session(1, "zsh", true)],
            }]),
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
                    Key::PaletteCloseSession,
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
            ]
        );
    }

    #[test]
    fn a_non_terminal_active_tab_can_be_closed_but_not_split() {
        let ctx = PaletteCtx {
            sessions: sessions(vec![SessionGroupViewModel {
                name: "fedora",
                target: TargetKind::Local,
                sessions: vec![session(2, "Settings", true)],
            }]),
            can_split_pane: false,
            ..empty_ctx()
        };

        let targets = targets(&palette_items(&ctx));

        assert!(targets.contains(&PaletteTarget::CloseSession));
        assert!(!targets.contains(&PaletteTarget::SplitPane));
    }

    #[test]
    fn groups_come_in_order_sessions_launches_actions() {
        let mut htop = launch(&["htop"], false);
        htop.title = "htop";
        let ctx = PaletteCtx {
            sessions: sessions(vec![SessionGroupViewModel {
                name: "fedora",
                target: TargetKind::Local,
                sessions: vec![session(1, "zsh", false)],
            }]),
            launches: vec![htop],
            can_split_pane: false,
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
