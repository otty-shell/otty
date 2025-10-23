<div align="center">

<img src="docs/iced_logo.svg" width="140px" />
<img src="docs/iced_term_logo.png" width="140px" />

# iced_term

![GitHub License](https://img.shields.io/github/license/Harzu/iced_term)
![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/iced_term)

Terminal emulator widget powered by ICED framework and alacritty terminal backend.

<a href="./examples/full_screen">
  <img src="examples/full_screen/assets/screenshot.png" width="275px">
</a>
<a href="./examples/split_view/assets/screenshot.png">
  <img src="examples/split_view/assets/screenshot.png" width="273px">
</a>

</div>

## Unstable widget API

The ICED framework does not have the stable API and this widget is also under development, so I can not promise the stable API and to document it at least while the ICED won't release the 1.0.0 version.

## Features

The widget is currently under development and does not provide full terminal features make sure that widget is covered everything you want.

- PTY content rendering ([msgcat --color=test](./docs/colortest))
- Multiple instance support
- Basic keyboard input
- Mouse interaction in different modes
- Adding custom keyboard or mouse bindings
- Resizing
- Scrolling
- Focusing
- Selecting
- Changing Font/Color scheme
- Hyperlinks processing (hover/open)

This widget was tested on MacOS, Linux and Windows (but only under [WSL2](https://learn.microsoft.com/en-us/windows/wsl/about)).

## Installation

From crates.io

```toml
iced_term = "0.6.0"
```

From git

```toml
iced_term = { git = "https://github.com/Harzu/iced_term", branch = "master" }
```
## Overview

Interacting with the widget is happened via:

**Commands** - you can send commands to widget for changing the widget state.

```rust
#[derive(Debug, Clone)]
pub enum Command {
    ChangeTheme(Box<ColorPalette>),
    ChangeFont(FontSettings),
    AddBindings(Vec<(Binding<InputKind>, BindingAction)>),
    ProxyToBackend(backend::Command),
}
```

**Events** - widget is produced some events that can be handled in application. Every event has the first `u64` argument that is **terminal instance id**.

```rust
#[derive(Debug, Clone)]
pub enum Event {
    BackendCall(u64, backend::Command),
}
```

Right now there is the only internal **CommandReceived** event that is needed for backend <-> view communication. You can also handle this event unwrap the command and process command additionally if you want.

**Actions** - widget's method `update(&mut self, cmd: Command)` returns **Action** that you can handle after widget updated.

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Action {
    Shutdown,
    ChangeTitle,
    #[default]
    Ignore,
}
```

For creating workable application example with this widget you need to do a several things

**Step 1.** Add widget to your `App` struct

```rust
struct App {
    title: String,
    term: iced_term::Terminal,
}
```

**Step 2.** Create instance in `App` constructor

```rust
impl App {
    fn new() -> (Self, Task<Event>) {
        let system_shell = std::env::var("SHELL")
            .expect("SHELL variable is not defined")
            .to_string();
        let term_id = 0;
        let term_settings = iced_term::settings::Settings {
            backend: iced_term::settings::BackendSettings {
                shell: system_shell.to_string(),
            },
            ..Default::default()
        };

        (
            Self {
                title: String::from("Terminal app"),
                term: iced_term::Terminal::new(term_id, term_settings)
                    .expect("failed to create the new terminal instance"),
            },
            Task::none(),
        )
    }
}
```

**Step 3.** Add event kind to **Events/Messages** enum that will be container of internal widget events for application's **Events/Messages**. You will have to wrap inner widget events via `.map(Event::Terminal)` where it's necessary. 

```rust
#[derive(Debug, Clone)]
pub enum Event {
    // ... other events
    Terminal(iced_term::Event),
}
```

**Step 4.** Add **Terminal** event kind processing to application `update` method.

```rust
impl App {
    // ... other methods
    fn update(&mut self, event: Event) -> Task<Event> {
        match event {
            Event::Terminal(iced_term::Event::BackendCall(_, cmd)) => {
                match self.term.handle(iced_term::Command::ProxyToBackend(cmd))
                {
                    iced_term::actions::Action::Shutdown => {
                        return window::get_latest().and_then(window::close)
                    },
                    _ => {},
                }
            },
        }

        Task::none()
    }
}
```

**Step 5.** Add view to your application

```rust
impl App {
    // ... other methods
    fn view(&self) -> Element<Event, Theme, iced::Renderer> {
        container(iced_term::TerminalView::show(&self.term).map(Event::Terminal))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
```

**Step 6.** Add event subscription for getting internal events from backend (pty).

```rust
impl App {
    // ... other methods
    fn subscription(&self) -> Subscription<Event> {
        Subscription::run_with_id(self.term.id, self.term.subscription())
            .map(Event::Terminal)
    }
}
```

**Step 7.** Add main function

```rust
fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .antialiasing(false)
        .window_size(Size {
            width: 1280.0,
            height: 720.0,
        })
        .subscription(App::subscription)
        .run_with(App::new)
}
```

**Step 8.** Run your application

```shell
cargo run --release
```

**Step 9.** To be happy!

## Examples

You can also look at [examples](./examples) directory for more information about widget using.

- [full_screen](./examples/full_screen/) - The basic example of terminal emulator.
- [split_view](./examples/split_view/) - The example based on split_view iced widget that show how multiple instance feature work.
- [custom_bindings](./examples/custom_bindings/) - The example that show how you can add custom keyboard or mouse bindings to your terminal emulator app.
- [themes](./examples/themes/) - The example that show how you can change terminal color scheme.
- [fonts](./examples/fonts/) - The examples that show how you can change font type or font size in your terminal emulator app.

You can run any example via

```shell
cargo run --package <example name>
```

## Dependencies

 - [iced (0.13.1)](https://github.com/iced-rs/iced/tree/master)
 - [alacritty_terminal (0.25.0)](https://github.com/alacritty/alacritty/tree/master/alacritty_terminal)
 - [tokio (1.47.1)](https://github.com/tokio-rs/tokio)

## Contributing / Feedback

All feedbacks, issues and pull requests are welcomed! Guidelines is coming soon =)
