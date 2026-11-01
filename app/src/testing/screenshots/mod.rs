//! Screenshots of the V4 screens in OTTY Dark and OTTY Light.
//!
//! The tests are ignored, so a normal test run skips them. Run them with
//! `cargo test -p otty -- --ignored screenshots`. PNG files go to the folder
//! in `OTTY_SCREENSHOT_DIR`, or to `otty-screens` in the system temp folder.
//!
//! Each screen is the real view functions of the app, fed with fixed data. No
//! shell is started and no settings file is read or written.

mod capture;
mod fixture;
mod window;

use iced::{Point, Size};
use otty_ui_kit::size;

use self::capture::{shoot, shoot_at};
use self::fixture::{TAB_EDITOR, TAB_SETTINGS, TAB_ZSH};
use self::window::{Frame, LEFT_WIDTH, WINDOW, at, terminal_main, window};
use crate::widgets::chrome::view::focus_note;
use crate::widgets::palette::view as palette_view;
use crate::widgets::quick_launch::view::{
    context_menu, new_launch_menu, secret_prompt, wizard_form,
};
use crate::widgets::settings::view::settings_form;
use crate::widgets::sidebar::types::HeaderMenu;
use crate::widgets::tabs::view::{empty_state, new_session_menu};
use crate::widgets::terminal_workspace::view::pane_context_menu;

/// Top of a section body: section padding 8, header row 30, gap 1.
const SECTION_BODY_TOP: f32 = 8.0 + 30.0 + 1.0;

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
