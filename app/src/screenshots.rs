//! Screenshots of the V4 screens in OTTY Dark and OTTY Light.
//!
//! The tests are ignored, so a normal test run skips them. Run them with
//! `cargo test -p otty -- --ignored screenshots`. PNG files go to the folder
//! in `OTTY_SCREENSHOT_DIR`, or to `otty-screens` in the system temp folder.
//!
//! Each screen is the real view functions of the app, fed with fixed data. No
//! shell is started and no settings file is read or written. The window is
//! composed here the way `view.rs` composes it, because `App::new()` loads
//! the user's settings and opens a terminal.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use iced::widget::{Space, column, container, row, text};
use iced::{Element, Length, Padding, Point, Size};
use iced_test::Simulator;
use otty_ui_kit::components::chip::Chip;
use otty_ui_kit::components::divider;
use otty_ui_kit::components::pane_header::PaneHeader;
use otty_ui_kit::components::panel_height::SectionHeight;
use otty_ui_kit::components::surface::{Surface, surface};
use otty_ui_kit::components::target_dot::TargetKind;
use otty_ui_kit::{fonts, size, typography};

use crate::app::view::stack_left_panel;
use crate::theme::{AppTheme, BuiltinTheme, ThemeProps};
use crate::widgets::chrome::model::{StatusBarInput, StatusBarViewModel};
use crate::widgets::chrome::view::{action_bar, focus_note, status_bar};
use crate::widgets::explorer::model::ExplorerTreeViewModel;
use crate::widgets::explorer::types::FileNode;
use crate::widgets::explorer::view::sidebar_tree;
use crate::widgets::palette::{
    PaletteCtx, PaletteIntent, PaletteWidget, view as palette_view,
};
use crate::widgets::quick_launch::model::QuickLaunchTreeViewModel;
use crate::widgets::quick_launch::state::{
    ContextMenuState, SecretPromptState, WizardEditorState,
};
use crate::widgets::quick_launch::types::{
    ContextMenuTarget, LaunchInfo, QuickLaunch, QuickLaunchFile, SshSecretKind,
    SshSecretRequest,
};
use crate::widgets::quick_launch::view::{
    context_menu, new_launch_menu, secret_prompt, sidebar_panel, wizard_form,
};
use crate::widgets::settings::SettingsWidget;
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar::types::HeaderMenu;
use crate::widgets::tabs::model::TabsViewModel;
use crate::widgets::tabs::types::{TabContent, TabItem, TabMachine};
use crate::widgets::tabs::view::{empty_state, new_session_menu, session_list};
use crate::widgets::terminal_workspace::model::PaneLabel;
use crate::widgets::terminal_workspace::view::pane_context_menu;

/// Window size of every screenshot, as in the design.
const WINDOW: Size = Size::new(1440.0, 900.0);
/// Width of the left panel (a fifth of the window, the default).
const LEFT_WIDTH: f32 = 288.0;
/// Top of a section body: section padding 8, header row 30, gap 1.
const SECTION_BODY_TOP: f32 = 8.0 + 30.0 + 1.0;
/// Width of the right panel when it is open.
const RIGHT_WIDTH: f32 = 280.0;

/// The message type of every screen: nothing is clicked.
type Screen<'a> = Element<'a, ()>;

/// What the panels and the status bar show around the main area.
#[derive(Clone, Copy)]
struct Frame {
    active_tab: Option<u64>,
    show_tabs: bool,
    many_sessions: bool,
    one_session: bool,
    left_open: bool,
    right_open: bool,
    focus_mode: bool,
    menu_open: Option<HeaderMenu>,
    pane_count: Option<usize>,
}

/// Fixed data behind every screen.
struct Fixture {
    tabs: BTreeMap<u64, TabItem>,
    many_tabs: BTreeMap<u64, TabItem>,
    one_tab: BTreeMap<u64, TabItem>,
    no_tabs: BTreeMap<u64, TabItem>,
    launches: QuickLaunchFile,
    files: Vec<FileNode>,
    pane: PaneLabel,
    settings: SettingsWidget,
    new_launch: WizardEditorState,
    secret: SecretPromptState,
    palette: PaletteWidget,
    launching: HashMap<Vec<String>, LaunchInfo>,
    launch_menu: ContextMenuState,
}

/// Terminal tab ids in the fixture.
const TAB_ZSH: u64 = 0;
const TAB_EDITOR: u64 = 3;
const TAB_SETTINGS: u64 = 4;

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_workspace() {
    shoot("workspace", |fx, theme| {
        window(
            fx,
            theme,
            Frame::terminal(),
            terminal_main(fx, theme),
            Vec::new(),
        )
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_many_sessions() {
    shoot("many-sessions", |fx, theme| {
        let frame = Frame {
            many_sessions: true,
            ..Frame::terminal()
        };

        window(fx, theme, frame, terminal_main(fx, theme), Vec::new())
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_one_session() {
    shoot("one-session", |fx, theme| {
        let frame = Frame {
            one_session: true,
            ..Frame::terminal()
        };

        window(fx, theme, frame, terminal_main(fx, theme), Vec::new())
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_hovered_session() {
    // Middle of the "zsh" row (selected) and the "nvim" row under it, first
    // and second under the "fedora" group label.
    let zsh_y = size::TITLEBAR
        + size::LINE
        + SECTION_BODY_TOP
        + size::GROUP
        + size::ROW / 2.0;
    let nvim_y = zsh_y + size::ROW;
    // Middle of the close cross: section padding 8, then the 24px button.
    let cross_x = LEFT_WIDTH - 8.0 - size::CONTROL_SM / 2.0;
    // Left part of the title search, away from its shortcut hint.
    let title_search = Point::new(500.0, size::TITLEBAR / 2.0);

    for (screen, cursor) in [
        ("session-hover", Point::new(80.0, nvim_y)),
        ("session-close-hover", Point::new(cross_x, nvim_y)),
        ("session-close-hover-selected", Point::new(cross_x, zsh_y)),
        ("title-search-hover", title_search),
    ] {
        shoot_at(screen, Some(cursor), |fx, theme| {
            window(
                fx,
                theme,
                Frame::terminal(),
                terminal_main(fx, theme),
                Vec::new(),
            )
        });
    }
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_focus_mode() {
    shoot("focus", |fx, theme| {
        let frame = Frame {
            left_open: false,
            right_open: false,
            focus_mode: true,
            ..Frame::terminal()
        };
        let note = focus_note::view(Some(1.0), theme).expect("focus note");

        window(fx, theme, frame, terminal_main(fx, theme), vec![note])
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_empty_state() {
    shoot("empty", |fx, theme| {
        let frame = Frame {
            active_tab: None,
            show_tabs: false,
            pane_count: None,
            ..Frame::terminal()
        };
        let main = empty_state::view(empty_state::EmptyStateProps {
            theme,
            on_new_terminal: (),
            on_new_launch: (),
        });

        window(fx, theme, frame, main, Vec::new())
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_palette() {
    shoot("palette", |fx, theme| {
        let vm = fx.palette.vm().expect("palette is open");
        let layer =
            palette_view::view(palette_view::PaletteProps { vm, theme })
                .map(|_| ());

        window(
            fx,
            theme,
            Frame::terminal(),
            terminal_main(fx, theme),
            vec![layer],
        )
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_new_session_menu() {
    shoot("new-session-menu", |fx, theme| {
        let frame = Frame {
            menu_open: Some(HeaderMenu::Sessions),
            ..Frame::terminal()
        };
        let menu =
            new_session_menu::view(new_session_menu::NewSessionMenuProps {
                theme,
                shell_name: "zsh",
                split_target: None,
                launch_titles: vec!["htop", "build"],
                on_tabs: |_| (),
                on_terminal: |_| (),
                on_launch: |_| (),
            });

        window(
            fx,
            theme,
            frame,
            terminal_main(fx, theme),
            vec![at(
                menu,
                Point::new(LEFT_WIDTH - 60.0, size::TITLEBAR + 48.0),
            )],
        )
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_launches_menu() {
    shoot("launches-menu", |fx, theme| {
        let frame = Frame {
            menu_open: Some(HeaderMenu::Launches),
            ..Frame::terminal()
        };
        let menu = new_launch_menu::view(new_launch_menu::NewLaunchMenuProps {
            theme,
            on_intent: |_| (),
        });

        window(
            fx,
            theme,
            frame,
            terminal_main(fx, theme),
            vec![at(
                menu,
                Point::new(LEFT_WIDTH - 60.0, size::TITLEBAR + 48.0 + 220.0),
            )],
        )
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_menus() {
    shoot("launch-menu", |fx, theme| {
        let layer = context_menu::view(context_menu::ContextMenuProps {
            menu: &fx.launch_menu,
            theme,
            area_size: WINDOW,
            launching: &fx.launching,
        })
        .map(|_| ());

        window(
            fx,
            theme,
            Frame::terminal(),
            terminal_main(fx, theme),
            vec![layer],
        )
    });

    shoot("pane-menu", |fx, theme| {
        let (_grid, pane) = iced::widget::pane_grid::State::new(());
        let layer =
            pane_context_menu::view(pane_context_menu::PaneContextMenuProps {
                tab_id: TAB_ZSH,
                pane,
                cursor: Point::new(700.0, 300.0),
                grid_size: Size::new(WINDOW.width, WINDOW.height),
                terminal_id: 1,
                focus_target: iced::widget::Id::unique(),
                has_block_selection: true,
                theme,
            })
            .map(|_| ());

        window(
            fx,
            theme,
            Frame::terminal(),
            terminal_main(fx, theme),
            vec![layer],
        )
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_passphrase_dialog() {
    shoot("passphrase", |fx, theme| {
        let layer = secret_prompt::view(secret_prompt::SecretPromptProps {
            prompt: &fx.secret,
            theme,
        })
        .map(|_| ());

        window(
            fx,
            theme,
            Frame::terminal(),
            terminal_main(fx, theme),
            vec![layer],
        )
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_launch_editor() {
    shoot("launch-editor", |fx, theme| {
        let frame = Frame {
            active_tab: Some(TAB_EDITOR),
            pane_count: None,
            ..Frame::terminal()
        };
        let main = wizard_form::view(wizard_form::WizardFormProps {
            tab_id: TAB_EDITOR,
            editor: &fx.new_launch,
            theme,
        })
        .map(|_| ());

        window(fx, theme, frame, main, Vec::new())
    });
}

#[test]
#[ignore = "writes PNG files; run with --ignored screenshots"]
fn screenshots_settings() {
    shoot("settings", |fx, theme| {
        let frame = Frame {
            active_tab: Some(TAB_SETTINGS),
            right_open: false,
            pane_count: None,
            ..Frame::terminal()
        };
        let main = settings_form::view(settings_form::SettingsFormProps {
            vm: fx.settings.vm(),
            theme,
        })
        .map(|_| ());

        window(fx, theme, frame, main, Vec::new())
    });
}

impl Frame {
    /// The default workspace: a terminal tab, both panels, two panes.
    fn terminal() -> Self {
        Self {
            active_tab: Some(TAB_ZSH),
            show_tabs: true,
            many_sessions: false,
            one_session: false,
            left_open: true,
            right_open: true,
            focus_mode: false,
            menu_open: None,
            pane_count: Some(2),
        }
    }
}

impl Fixture {
    fn new() -> Self {
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
            tabs.insert(
                id,
                TabItem::new(id, title.to_string(), content, machine),
            );
        }

        let launches = launch_file();
        let secret = SecretPromptState::new(
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
        let ctx = PaletteCtx {
            sessions: TabsViewModel::new(&tabs, Some(TAB_ZSH), Some("fedora")),
            launches: QuickLaunchTreeViewModel {
                data: &launches,
                selected_path: None,
                hovered_path: None,
                inline_edit: None,
                launching: &fx_launching,
                drop_target: None,
            }
            .launches(),
            can_split_pane: true,
        };
        let _ = palette.reduce(PaletteIntent::Open, &ctx);
        drop(ctx);

        Self {
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
            launch_menu: ContextMenuState::default()
                .with_target(ContextMenuTarget::Command(vec![String::from(
                    "htop",
                )]))
                .with_cursor(Point::new(140.0, 420.0)),
        }
    }
}

/// Render `build` in both themes and write the PNG files.
fn shoot(
    screen: &str,
    build: impl for<'a> Fn(&'a Fixture, ThemeProps<'a>) -> Screen<'a>,
) {
    shoot_at(screen, None, build);
}

/// Render `build` in both themes with the pointer at `cursor`, if any, and
/// write the PNG files.
fn shoot_at(
    screen: &str,
    cursor: Option<Point>,
    build: impl for<'a> Fn(&'a Fixture, ThemeProps<'a>) -> Screen<'a>,
) {
    let fixture = Fixture::new();

    for (suffix, builtin) in [
        ("dark", BuiltinTheme::OttyDark),
        ("light", BuiltinTheme::OttyLight),
    ] {
        let app_theme = AppTheme::new(builtin);
        let iced_theme = iced::Theme::from(&app_theme);
        let element = build(&fixture, ThemeProps::new(&app_theme));

        let settings = iced::Settings {
            fonts: fonts::all_bytes()
                .into_iter()
                .map(std::borrow::Cow::Borrowed)
                .collect(),
            default_font: fonts::UI,
            ..iced::Settings::default()
        };
        let mut ui = Simulator::with_size(settings, WINDOW, element);
        if let Some(cursor) = cursor {
            ui.point_at(cursor);
            let _ = ui.simulate([iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { position: cursor },
            )]);
        }
        let snapshot = ui.snapshot(&iced_theme).expect("snapshot should draw");

        save(&snapshot, &format!("{screen}-{suffix}"));
    }
}

/// Write `snapshot` as `<name>.png` in the screenshot folder.
fn save(snapshot: &iced_test::simulator::Snapshot, name: &str) {
    let folder = std::env::var_os("OTTY_SCREENSHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("otty-screens"));
    let stage = folder.join(format!(".stage-{name}"));
    let _ = fs::remove_dir_all(&stage);
    fs::create_dir_all(&stage).expect("screenshot folder should be writable");

    // `matches_image` names the file after the renderer, so it goes to a
    // stage folder first and is renamed from there.
    snapshot
        .matches_image(stage.join(name))
        .expect("PNG should be written");
    let written = fs::read_dir(&stage)
        .expect("stage folder should be readable")
        .next()
        .expect("one PNG should be written")
        .expect("entry should be readable")
        .path();
    fs::rename(written, folder.join(format!("{name}.png")))
        .expect("PNG should be moved");
    let _ = fs::remove_dir_all(&stage);
}

/// The window: title bar, panels around `main`, status bar and `overlays`
/// on top of the panels and the main area.
fn window<'a>(
    fx: &'a Fixture,
    theme: ThemeProps<'a>,
    frame: Frame,
    main: Screen<'a>,
    overlays: Vec<Screen<'a>>,
) -> Screen<'a> {
    let palette = theme.theme.ui();
    let tabs = if !frame.show_tabs {
        &fx.no_tabs
    } else if frame.many_sessions {
        &fx.many_tabs
    } else if frame.one_session {
        &fx.one_tab
    } else {
        &fx.tabs
    };

    let title = tabs
        .get(&frame.active_tab.unwrap_or(u64::MAX))
        .map(TabItem::title);
    let header = column![
        action_bar::view(action_bar::ActionBarProps {
            title,
            theme,
            left_open: frame.left_open,
            right_open: frame.right_open,
            focus_mode: frame.focus_mode,
        })
        .map(|_| ()),
        divider::horizontal(palette),
    ];

    let mut body = row![].height(Length::Fill);
    if frame.left_open {
        body = body
            .push(
                container(left_panel(fx, tabs, theme, frame)).width(LEFT_WIDTH),
            )
            .push(divider::vertical(palette));
    }
    body = body.push(container(main).width(Length::Fill));
    if frame.right_open {
        body = body.push(divider::vertical(palette)).push(
            container(surface(
                sidebar_tree::view(sidebar_tree::SidebarTreeProps {
                    vm: ExplorerTreeViewModel {
                        root_label: Some("otty"),
                        tree: &fx.files,
                        selected_path: None,
                        hovered_path: None,
                    },
                    theme,
                })
                .map(|_| ()),
                Surface::Panel,
                palette,
            ))
            .width(RIGHT_WIDTH),
        );
    }

    let mut layers = vec![body.into()];
    layers.extend(overlays);
    let content = iced::widget::Stack::with_children(layers)
        .width(Length::Fill)
        .height(Length::Fill);

    let session_count = tabs.len();
    let focused = (frame.pane_count.is_some()).then_some(&fx.pane);
    let status = StatusBarViewModel::new(StatusBarInput {
        machine_name: "fedora",
        machine_target: TargetKind::Local,
        focused_pane: focused,
        pane_count: frame.pane_count,
        session_count,
    });

    column![header, content, status_bar::view(&status, "0.1.0", theme)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Sessions over Launches on the panel surface, sharing the panel height
/// the way `view.rs` does.
fn left_panel<'a>(
    fx: &'a Fixture,
    tabs: &'a BTreeMap<u64, TabItem>,
    theme: ThemeProps<'a>,
    frame: Frame,
) -> Screen<'a> {
    let palette = theme.theme.ui();
    let launches_vm = || QuickLaunchTreeViewModel {
        data: &fx.launches,
        selected_path: None,
        hovered_path: None,
        inline_edit: None,
        launching: &fx.launching,
        drop_target: None,
    };
    let sessions_vm =
        move || TabsViewModel::new(tabs, frame.active_tab, Some("fedora"));

    let sessions_height = SectionHeight::panel_section(
        session_list::content_height(&sessions_vm()),
        true,
    );
    let launches_height = SectionHeight::panel_section(
        sidebar_panel::content_height(&launches_vm()),
        true,
    );

    let sections = stack_left_panel(
        sessions_height,
        launches_height,
        palette,
        move || {
            let sessions = session_list::view(session_list::SessionListProps {
                vm: sessions_vm(),
                theme,
                is_open: true,
                new_session_title: "zsh",
                is_menu_open: frame.menu_open == Some(HeaderMenu::Sessions),
                on_toggle: (),
                on_menu: (),
                on_intent: |_| (),
            });
            let launches =
                sidebar_panel::view(sidebar_panel::SidebarPanelProps {
                    vm: launches_vm(),
                    theme,
                    is_open: true,
                    is_menu_open: frame.menu_open == Some(HeaderMenu::Launches),
                    on_toggle: (),
                    on_menu: (),
                    on_intent: |_| (),
                });

            (sessions, launches)
        },
    );

    surface(sections, Surface::Panel, palette).into()
}

/// Two panes with headers and a few lines of terminal text.
fn terminal_main<'a>(fx: &'a Fixture, theme: ThemeProps<'a>) -> Screen<'a> {
    let palette = theme.theme.ui();

    let pane = |program: &str, folder: &str, is_focused: bool| {
        let chip = Chip::new("fedora", palette).target(TargetKind::Local);
        let header = PaneHeader::new(program, palette)
            .folder(folder)
            .focused(is_focused)
            .trailing(chip);

        let canvas = palette.surface_canvas();
        let strong = palette.text_strong();
        let muted = palette.text_muted();
        let lines = column![
            terminal_line("~/otty  main", "cargo test -p otty", strong, muted),
            terminal_line("", "test result: ok. 512 passed", muted, muted),
            terminal_line("~/otty  main", "", strong, muted),
        ]
        .spacing(4);

        column![
            Element::from(header),
            container(lines)
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(canvas.into()),
                    ..container::Style::default()
                }),
        ]
        .width(Length::Fill)
    };

    let panes = row![
        pane(
            fx.pane.program(),
            fx.pane.folder_name().unwrap_or("~"),
            true
        ),
        divider::vertical(palette),
        pane("cargo", "app", false),
    ];

    panes.width(Length::Fill).height(Length::Fill).into()
}

/// One line of terminal text: a prompt and a command.
fn terminal_line<'a>(
    prompt: &'a str,
    command: &'a str,
    command_color: iced::Color,
    prompt_color: iced::Color,
) -> Element<'a, ()> {
    let style = typography::BODY;

    row![
        text(prompt)
            .font(fonts::MONO)
            .size(style.size())
            .color(prompt_color),
        text(command)
            .font(fonts::MONO)
            .size(style.size())
            .color(command_color),
    ]
    .spacing(8)
    .into()
}

/// Put `element` with its top left corner at `point` of a full-size layer.
fn at<'a>(element: Screen<'a>, point: Point) -> Screen<'a> {
    container(row![Space::new().width(0), element])
        .padding(Padding {
            top: point.y,
            left: point.x,
            ..Padding::ZERO
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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
                id,
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
        TAB_ZSH,
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
