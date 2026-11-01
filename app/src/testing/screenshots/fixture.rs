//! Fixed data behind every screen: tabs, launches, files, widget states and
//! the panels of the screen.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use iced::Point;

use super::window::Frame;
use crate::events::palette::palette_ctx;
use crate::widgets::explorer::types::FileNode;
use crate::widgets::palette::{PaletteIntent, PaletteWidget};
use crate::widgets::quick_launch::model::QuickLaunchTreeViewModel;
use crate::widgets::quick_launch::state::{
    ContextMenuState, SecretPromptState, WizardEditorState,
};
use crate::widgets::quick_launch::types::{
    ContextMenuTarget, LaunchInfo, QuickLaunch, QuickLaunchFile, SshSecretKind,
    SshSecretRequest,
};
use crate::widgets::settings::SettingsWidget;
use crate::widgets::sidebar::SidebarWidget;
use crate::widgets::tabs::model::TabsViewModel;
use crate::widgets::tabs::types::{TabContent, TabItem, TabMachine};
use crate::widgets::terminal_workspace::model::PaneLabel;

/// Terminal tab ids in the fixture.
pub(super) const TAB_ZSH: u64 = 0;
pub(super) const TAB_EDITOR: u64 = 3;
pub(super) const TAB_SETTINGS: u64 = 4;

/// Fixed data behind every screen.
pub(super) struct Fixture {
    pub(super) frame: Frame,
    pub(super) sidebar: SidebarWidget,
    pub(super) tabs: BTreeMap<u64, TabItem>,
    pub(super) many_tabs: BTreeMap<u64, TabItem>,
    pub(super) one_tab: BTreeMap<u64, TabItem>,
    pub(super) no_tabs: BTreeMap<u64, TabItem>,
    pub(super) launches: QuickLaunchFile,
    pub(super) files: Vec<FileNode>,
    pub(super) pane: PaneLabel,
    pub(super) settings: SettingsWidget,
    pub(super) new_launch: WizardEditorState,
    pub(super) secret: SecretPromptState,
    pub(super) palette: PaletteWidget,
    pub(super) launching: HashMap<Vec<String>, LaunchInfo>,
    pub(super) launch_menu: ContextMenuState,
}

impl Fixture {
    /// Fixed data for a screen with the panels of `frame`.
    pub(super) fn new(frame: Frame) -> Self {
        let mut tabs = BTreeMap::new();
        for (id, title, content, machine) in [
            (0, "zsh", TabContent::Terminal, TabMachine::ThisComputer),
            (1, "nvim", TabContent::Terminal, TabMachine::ThisComputer),
            (
                2,
                "api-prod",
                TabContent::Terminal,
                TabMachine::Ssh(String::from("prod-api")),
            ),
            (
                TAB_EDITOR,
                "New launch",
                TabContent::QuickLaunchWizard,
                TabMachine::ThisComputer,
            ),
            (
                TAB_SETTINGS,
                "Settings",
                TabContent::Settings,
                TabMachine::ThisComputer,
            ),
        ] {
            tabs.insert(id, TabItem::new(title.to_string(), content, machine));
        }

        let launches = launch_file();
        let secret = SecretPromptState::asking(
            vec![String::from("prod-api")],
            Box::new(ssh_launch()),
            SshSecretRequest::new(
                SshSecretKind::KeyPassphrase,
                String::from("~/.ssh/id_ed25519"),
                false,
            ),
        );

        let mut palette = PaletteWidget::new();
        let fx_launching = HashMap::new();
        let ctx = palette_ctx(
            &TabsViewModel::new(&tabs, Some(TAB_ZSH), Some("fedora")),
            QuickLaunchTreeViewModel::idle(&launches, &fx_launching).launches(),
            true,
        );
        let _ = palette.reduce(PaletteIntent::Open, &ctx);
        drop(ctx);

        Self {
            frame,
            sidebar: frame.sidebar(),
            tabs,
            many_tabs: many_tabs(),
            one_tab: one_tab(),
            no_tabs: BTreeMap::new(),
            launches,
            files: files(),
            pane: PaneLabel::new(
                "zsh",
                Some(Path::new("/home/ann/otty")),
                Some(Path::new("/home/ann")),
            ),
            settings: SettingsWidget::new(),
            new_launch: WizardEditorState::new(Vec::new()),
            secret,
            palette,
            launching: HashMap::new(),
            launch_menu: ContextMenuState::at(
                ContextMenuTarget::Command(vec![String::from("htop")]),
                Point::new(140.0, 420.0),
            ),
        }
    }
}

/// Launches shown in the Launches section.
fn launch_file() -> QuickLaunchFile {
    serde_json::from_str(
        r#"{
          "version": 1,
          "root": {
            "title": "Root",
            "expanded": true,
            "children": [
              {"kind": "Folder", "title": "Work", "expanded": true, "children": [
                {"kind": "Command", "title": "prod-api",
                 "spec": {"type": "Ssh", "ssh": {"host": "prod-api", "port": 22,
                   "user": "deploy", "identity_file": null, "extra_args": []}}},
                {"kind": "Command", "title": "build",
                 "spec": {"type": "Custom", "custom": {"program": "make",
                   "args": ["-j8"], "env": [], "working_directory": null}}}
              ]},
              {"kind": "Command", "title": "htop",
               "spec": {"type": "Custom", "custom": {"program": "htop",
                 "args": [], "env": [], "working_directory": null}}}
            ]
          }
        }"#,
    )
    .expect("launch fixture should parse")
}

/// The SSH launch behind the passphrase dialog.
fn ssh_launch() -> QuickLaunch {
    serde_json::from_str(
        r#"{"title": "prod-api",
            "spec": {"type": "Ssh", "ssh": {"host": "prod-api", "port": 22,
              "user": "deploy", "identity_file": null, "extra_args": []}}}"#,
    )
    .expect("ssh launch fixture should parse")
}

/// Thirty terminal tabs on this computer and on two SSH hosts: more rows
/// than the Sessions section can show next to Launches.
fn many_tabs() -> BTreeMap<u64, TabItem> {
    (0..30)
        .map(|id| {
            let machine = match id % 3 {
                0 => TabMachine::ThisComputer,
                1 => TabMachine::Ssh(String::from("prod-api")),
                _ => TabMachine::Ssh(String::from("build-box")),
            };
            let tab = TabItem::new(
                format!("shell {id}"),
                TabContent::Terminal,
                machine,
            );

            (id, tab)
        })
        .collect()
}

/// The "zsh" terminal tab alone: the Sessions section with one row.
fn one_tab() -> BTreeMap<u64, TabItem> {
    let tab = TabItem::new(
        String::from("zsh"),
        TabContent::Terminal,
        TabMachine::ThisComputer,
    );

    BTreeMap::from([(TAB_ZSH, tab)])
}

/// Rows of the Files section.
fn files() -> Vec<FileNode> {
    [
        ("app", true),
        ("crates", true),
        ("specs", true),
        ("Cargo.toml", false),
        ("README.md", false),
    ]
    .into_iter()
    .map(|(name, is_folder)| {
        FileNode::new(
            name.to_string(),
            PathBuf::from("/home/ann/otty").join(name),
            is_folder,
        )
    })
    .collect()
}
