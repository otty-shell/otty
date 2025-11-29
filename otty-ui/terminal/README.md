# otty-ui-term

A powerful terminal emulator widget for the [Iced](https://github.com/iced-rs/iced) UI framework.

## Overview

`otty-ui-term` is a fully-featured terminal emulator widget that can be embedded into any Iced application. It provides a native Rust terminal experience with support for local shell sessions, SSH connections, customizable themes, and flexible configuration options.

## Features

- **Full Terminal Emulation**: Complete VT100/xterm-compatible terminal emulation
- **Local & Remote Sessions**: Support for both local shell and SSH connections
- **Customizable Theming**: Built-in color palettes and theme customization
- **Font Configuration**: Flexible font settings with size and scaling options
- **Keyboard Input Handling**: Full keyboard and modifier key support
- **Event-Driven Architecture**: React to terminal events like title changes and process exits

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
otty-ui-term = "0.1.0"
iced = { version = "0.13" }
```

## Quick Start

Here's a minimal example to get a terminal running in your Iced application:

```rust
use iced::advanced::graphics::core::Element;
use iced::widget::container;
use iced::{Length, Size, Subscription, Task, Theme, window};
use otty_ui_term::TerminalView;
use otty_ui_term::settings::{LocalSessionOptions, SessionKind};

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Event {
    Terminal(otty_ui_term::Event),
}

struct App {
    title: String,
    term: otty_ui_term::Terminal,
}

impl App {
    fn new() -> (Self, Task<Event>) {
        let system_shell =
            std::env::var("SHELL").expect("SHELL variable is not defined");

        let session_options =
            LocalSessionOptions::default().with_program(&system_shell);
        let session = SessionKind::from_local_options(session_options);
        let term_id = 0;
        let term_settings = otty_ui_term::settings::Settings {
            backend: otty_ui_term::settings::BackendSettings::default()
                .with_session(session),
            ..Default::default()
        };

        (
            Self {
                title: String::from("full_screen"),
                term: otty_ui_term::Terminal::new(term_id, term_settings)
                    .expect("failed to create the new terminal instance"),
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn subscription(&self) -> Subscription<Event> {
        let id = self.term.id;
        let subscription = self.term.subscription();
        Subscription::run_with_id(id, subscription).map(Event::Terminal)
    }

    fn update(&mut self, event: Event) -> Task<Event> {
        use otty_ui_term::Event::*;

        match event {
            Event::Terminal(inner) => match inner {
                Shutdown { .. } => {
                    return window::get_latest().and_then(window::close);
                },
                TitleChanged { title, .. } => {
                    self.title = title;
                },
                event => self.term.handle(event),
            },
        }

        Task::none()
    }

    fn view(&'_ self) -> Element<'_, Event, Theme, iced::Renderer> {
        container(TerminalView::show(&self.term).map(Event::Terminal))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
```

## Configuration

### Session Types

#### Local Session

Run a local shell or program:

```rust
use otty_ui_term::settings::{SessionKind, LocalSessionOptions};
use std::collections::HashMap;

let mut envs = HashMap::new();
envs.insert("TERM".to_string(), "xterm-256color".to_string());

let session = SessionKind::from_local_options(
    LocalSessionOptions::default()
        .with_program("/bin/zsh")
        .with_args(vec!["-l".to_string()])
        .with_envs(envs)
        .with_working_directory("/home/user".into())
);
```

#### SSH Session

Connect to a remote server:

```rust
use otty_ui_term::settings::{SessionKind, SSHSessionOptions};
use otty_libterm::pty::SSHAuth;

let session = SessionKind::from_ssh_options(
    SSHSessionOptions::default()
        .with_host("example.com")
        .with_user("username")
        .with_auth(SSHAuth::Password("password".to_string()))
);
```

### Font Settings

Customize the terminal font:

```rust
use otty_ui_term::settings::FontSettings;
use iced::Font;

let font_settings = FontSettings::default()
    .with_size(16.0)
    .with_scale_factor(1.5)
    .with_font_type(Font::MONOSPACE);
```

### Theme Settings

Apply custom color schemes:

```rust
use otty_ui_term::settings::ThemeSettings;
use otty_ui_term::ColorPalette;

let theme = ThemeSettings::new(Box::new(ColorPalette::default()));
```

### Complete Settings

Combine all settings:

```rust
use otty_ui_term::settings::{Settings, BackendSettings};
use otty_libterm::TerminalSize;

let settings = Settings {
    font: font_settings,
    theme: theme,
    backend: BackendSettings::default()
        .with_session(session)
        .with_size(TerminalSize::new(80, 24)),
};
```

## Handling Events

The terminal emits various events that you can handle in your application:

```rust
use otty_ui_term::Event;

match event {
    Event::TitleChanged { id, title } => {
        // Update window title
        println!("Terminal title changed to: {}", title);
    }
    Event::Shutdown { id, code } => {
        // Handle terminal process exit
        println!("Terminal exited with code: {:?}", code);
    }
    Event::SurfaceModeChanged { id, mode } => {
        // Handle alternate screen buffer changes
    }
    _ => {
        // Let the terminal handle other events
        terminal.handle(event);
    }
}
```

## Examples

Check out the [examples](./examples) directory for complete working examples:

- [**full_screen.rs**](./examples/full_screen.rs) - Full-screen terminal application
- [**split_view.rs**](./examples/split_view.rs) - Multiple terminals in a split view
- [**themes.rs**](./examples/themes.rs) - Different color schemes
- [**fonts.rs**](./examples/fonts.rs) - Font customization
- [**bindings.rs**](./examples/bindings.rs) - Custom key bindings


## API Reference

### Core Types

- **`Terminal`** - The terminal instance that manages the PTY and state
- **`TerminalView`** - The widget for rendering the terminal in the UI
- **`Event`** - Events emitted by the terminal
- **`Settings`** - Configuration for terminal behavior and appearance

### Key Methods

- `Terminal::new(id, settings)` - Create a new terminal instance
- `Terminal::subscription()` - Get the subscription for terminal events
- `Terminal::handle(event)` - Handle terminal events
- `TerminalView::show(&terminal)` - Render the terminal widget

## Requirements

- Rust 1.70 or later
- Iced 0.13

## Platform Support

- Linux
- macOS

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](../../LICENSE) for details.

## Contributing

Contributions are welcome! This project is part of the larger [OTTY](https://github.com/otty-shell/otty) workspace.

## Links

- [Homepage](https://otty.sh)
- [Repository](https://github.com/otty-shell/otty)
- [Iced Framework](https://github.com/iced-rs/iced)

## Acknowledgments

Built with the excellent [Iced](https://github.com/iced-rs/iced) UI framework and powered by `otty-libterm` for terminal emulation.
