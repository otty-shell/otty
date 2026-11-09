//! The edits that wire a new widget into `widgets/mod.rs`,
//! `dispatch/mod.rs`, `effects/mod.rs` and `app.rs`.

use super::error::NewWidgetError;
use super::name::WidgetName;

/// The files this module edits, relative to `app/src`.
pub(super) const WIDGETS_MOD: &str = "widgets/mod.rs";
pub(super) const DISPATCH_MOD: &str = "dispatch/mod.rs";
pub(super) const EFFECTS_MOD: &str = "effects/mod.rs";
pub(super) const APP: &str = "app.rs";

/// Module declarations; rustfmt keeps them in alphabetical order.
const MODULE: &str = "pub(crate) mod ";

/// Imports from the widgets; rustfmt keeps them in alphabetical order.
const WIDGET_IMPORT: &str = "use crate::widgets::";

/// The blocks that get lines for each widget.
const WIDGETS_STRUCT: &str = "pub(crate) struct Widgets {";
const APP_EVENT_ENUM: &str = "pub(crate) enum AppEvent {";
const DISPATCH_FN: &str = "pub(crate) fn dispatch(";
const WIDGETS_LITERAL: &str = "let widgets = Widgets {";

/// How errors name the lines that the variant and the arms go after.
const WIDGET_VARIANT: &str = "a widget variant such as `Tabs(TabsEvent),` \
                              in `pub(crate) enum AppEvent {`";
const WIDGET_ARM: &str = "a widget arm such as \
                          `AppEvent::Tabs(TabsEvent::Effect(effect)) =>` \
                          in `pub(crate) fn dispatch(`";

/// `widgets/mod.rs` with the widget's module and its `Widgets` field.
pub(super) fn widgets_mod(
    text: &str,
    name: &WidgetName,
) -> Result<String, NewWidgetError> {
    let snake = name.snake();
    let camel = name.camel();
    let mut lines = lines_of(text);

    let module = format!("{MODULE}{snake};");
    insert_sorted(&mut lines, WIDGETS_MOD, MODULE, module)?;
    let field = format!("pub(crate) {snake}: {snake}::{camel}Widget,");
    insert_last_field(&mut lines, WIDGETS_MOD, WIDGETS_STRUCT, &field)?;

    Ok(joined(&lines))
}

/// `dispatch/mod.rs` with the event import, the delivery module, the
/// `AppEvent` variant and its two arms in `dispatch`: the intents go to the
/// delivery, the effects to the effects file.
pub(super) fn dispatch_mod(
    text: &str,
    name: &WidgetName,
) -> Result<String, NewWidgetError> {
    let snake = name.snake();
    let camel = name.camel();
    let mut lines = lines_of(text);

    let (start, end) = block(&lines, DISPATCH_MOD, APP_EVENT_ENUM)?;
    let variants = &lines[start + 1..end];
    if variants.iter().any(|line| names_variant(line, camel)) {
        let taken = format!("{DISPATCH_MOD}: `AppEvent::{camel}`");
        return Err(NewWidgetError::NameTaken(taken));
    }

    let import = format!("{WIDGET_IMPORT}{snake}::event::{camel}Event;");
    insert_sorted(&mut lines, DISPATCH_MOD, WIDGET_IMPORT, import)?;
    let module = format!("{MODULE}{snake};");
    insert_sorted(&mut lines, DISPATCH_MOD, MODULE, module)?;
    let variant = format!("{camel}({camel}Event),");
    insert_after_last_widget_item(
        &mut lines,
        APP_EVENT_ENUM,
        is_widget_variant,
        &[variant],
        WIDGET_VARIANT,
    )?;
    let arms = [
        format!(
            "AppEvent::{camel}({camel}Event::Intent(intent)) => \
             {snake}::deliver(app, intent),"
        ),
        format!(
            "AppEvent::{camel}({camel}Event::Effect(effect)) => \
             effects::{snake}::run(effect),"
        ),
    ];
    insert_after_last_widget_item(
        &mut lines,
        DISPATCH_FN,
        is_widget_arm,
        &arms,
        WIDGET_ARM,
    )?;

    Ok(joined(&lines))
}

/// `effects/mod.rs` with the module of the widget's effects file.
pub(super) fn effects_mod(
    text: &str,
    name: &WidgetName,
) -> Result<String, NewWidgetError> {
    let snake = name.snake();
    let mut lines = lines_of(text);

    let module = format!("{MODULE}{snake};");
    insert_sorted(&mut lines, EFFECTS_MOD, MODULE, module)?;

    Ok(joined(&lines))
}

/// `app.rs` with the widget import and the widget built in `App::new`.
pub(super) fn app(
    text: &str,
    name: &WidgetName,
) -> Result<String, NewWidgetError> {
    let snake = name.snake();
    let camel = name.camel();
    let mut lines = lines_of(text);

    let import = format!("{WIDGET_IMPORT}{snake}::{camel}Widget;");
    insert_sorted(&mut lines, APP, WIDGET_IMPORT, import)?;
    let field = format!("{snake}: {camel}Widget::new(),");
    insert_last_field(&mut lines, APP, WIDGETS_LITERAL, &field)?;

    Ok(joined(&lines))
}

fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

fn joined(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');

    text
}

/// Insert `line` among the statements that start a line with `prefix`,
/// before the first one that sorts after it, or after the last one.
fn insert_sorted(
    lines: &mut Vec<String>,
    file: &'static str,
    prefix: &str,
    line: String,
) -> Result<(), NewWidgetError> {
    let starts: Vec<usize> = (0..lines.len())
        .filter(|index| lines[*index].starts_with(prefix))
        .collect();
    let Some(&last) = starts.last() else {
        let anchor = format!("a `{}` line", prefix.trim_end());
        return Err(anchor_not_found(file, anchor));
    };

    let at = match starts.iter().find(|index| lines[**index] > line) {
        Some(&index) => index,
        None => statement_end(lines, last) + 1,
    };
    lines.insert(at, line);

    Ok(())
}

/// The line where the statement that starts at `start` ends with `;`.
fn statement_end(lines: &[String], start: usize) -> usize {
    (start..lines.len())
        .find(|index| lines[*index].trim_end().ends_with(';'))
        .unwrap_or(start)
}

/// Add `field` as the last line of the block `opening`, one indent deeper
/// than the opening line.
fn insert_last_field(
    lines: &mut Vec<String>,
    file: &'static str,
    opening: &str,
    field: &str,
) -> Result<(), NewWidgetError> {
    let (start, end) = block(lines, file, opening)?;

    let indent = indent_of(&lines[start]);
    let line = format!("{indent}    {field}");
    lines.insert(end, line);

    Ok(())
}

/// Add `code`, one line each, after the last item of the block `opening` in
/// `dispatch/mod.rs` whose first line `is_widget_line` accepts, with the
/// same indent. An item that opens a block with `{` ends where it closes.
fn insert_after_last_widget_item(
    lines: &mut Vec<String>,
    opening: &str,
    is_widget_line: fn(&str) -> bool,
    code: &[String],
    missing: &str,
) -> Result<(), NewWidgetError> {
    let (start, end) = block(lines, DISPATCH_MOD, opening)?;
    let Some(last) =
        (start + 1..end).rfind(|index| is_widget_line(&lines[*index]))
    else {
        return Err(anchor_not_found(DISPATCH_MOD, missing.to_string()));
    };

    let item_end = if lines[last].trim_end().ends_with('{') {
        let item = format!("the end of the arm `{}`", lines[last].trim());
        closing_line(lines, last)
            .filter(|close| *close < end)
            .ok_or_else(|| anchor_not_found(DISPATCH_MOD, item))?
    } else {
        last
    };

    let indent = indent_of(&lines[last]).to_string();
    for (offset, code_line) in code.iter().enumerate() {
        let line = format!("{indent}{code_line}");
        lines.insert(item_end + 1 + offset, line);
    }

    Ok(())
}

/// The line that opens the block starting with `opening` and the line that
/// closes it at the same indent.
fn block(
    lines: &[String],
    file: &'static str,
    opening: &str,
) -> Result<(usize, usize), NewWidgetError> {
    let start = lines
        .iter()
        .position(|line| line.trim_start().starts_with(opening))
        .ok_or_else(|| anchor_not_found(file, format!("`{opening}`")))?;

    let end = closing_line(lines, start).ok_or_else(|| {
        anchor_not_found(file, format!("the end of `{opening}`"))
    })?;

    Ok((start, end))
}

/// The first line after `start`, at the same indent, that closes a block:
/// `}`, `};` or `},`.
fn closing_line(lines: &[String], start: usize) -> Option<usize> {
    let indent = indent_of(&lines[start]);

    (start + 1..lines.len()).find(|index| {
        lines[*index]
            .strip_prefix(indent)
            .is_some_and(|rest| rest == "}" || rest == "};" || rest == "},")
    })
}

fn indent_of(line: &str) -> &str {
    let code = line.trim_start().len();

    &line[..line.len() - code]
}

/// Whether `line` declares the `AppEvent` variant `camel`.
fn names_variant(line: &str, camel: &str) -> bool {
    line.trim().strip_prefix(camel).is_some_and(|rest| {
        rest.is_empty() || rest.starts_with(['(', ',', ' ', '{'])
    })
}

/// Whether `line` is a widget variant such as `Tabs(TabsEvent),`.
fn is_widget_variant(line: &str) -> bool {
    let Some(inside) = line.trim().strip_suffix("Event),") else {
        return false;
    };

    inside
        .split_once('(')
        .is_some_and(|(variant, event)| !variant.is_empty() && variant == event)
}

/// Whether `line` starts a widget arm such as
/// `AppEvent::Tabs(TabsEvent::Intent(intent)) =>` or
/// `AppEvent::Tabs(TabsEvent::Effect(effect)) =>`.
fn is_widget_arm(line: &str) -> bool {
    let Some(arm) = line.trim().strip_prefix("AppEvent::") else {
        return false;
    };
    let Some((variant, rest)) = arm.split_once('(') else {
        return false;
    };

    let event = format!("{variant}Event::");
    rest.strip_prefix(event.as_str()).is_some_and(|rest| {
        rest.starts_with("Intent(intent)) =>")
            || rest.starts_with("Effect(effect)) =>")
    })
}

fn anchor_not_found(file: &'static str, anchor: String) -> NewWidgetError {
    NewWidgetError::AnchorNotFound { file, anchor }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDGETS_MOD_TEXT: &str = "\
pub(crate) mod chrome;
pub(crate) mod tabs;

pub(crate) struct Widgets {
    pub(crate) tabs: tabs::TabsWidget,
    pub(crate) chrome: chrome::ChromeWidget,
}
";

    const DISPATCH_MOD_TEXT: &str = "\
use iced::Task;

use crate::app::App;
use crate::effects;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::tabs::event::TabsEvent;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};

pub(crate) mod chrome;
pub(crate) mod command;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;

#[derive(Clone)]
pub(crate) enum AppEvent {
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    TerminalWorkspace(TerminalWorkspaceEvent),
    /// Run an app command.
    Command(AppCommand),
    IcedReady,
    SyncTerminalGridSizes,
    Window(iced::window::Event),
}

/// Hand an app event to the one that handles it.
pub(crate) fn dispatch(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => effects::startup::run(app),
        AppEvent::Chrome(ChromeEvent::Intent(intent)) => {
            chrome::deliver(app, intent)
        },
        AppEvent::Chrome(ChromeEvent::Effect(effect)) => {
            effects::chrome::run(effect)
        },
        AppEvent::Tabs(TabsEvent::Intent(intent)) => tabs::deliver(app, intent),
        AppEvent::Tabs(TabsEvent::Effect(effect)) => {
            effects::tabs::run(app, effect)
        },
        AppEvent::Command(command) => command::deliver(app, command),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        AppEvent::Window(_) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::sidebar::event::SidebarIntent;
}
";

    const EFFECTS_MOD_TEXT: &str = "\
//! The Effects layer.

pub(crate) mod chrome;
pub(crate) mod startup;
pub(crate) mod tabs;
";

    const APP_TEXT: &str = "\
use crate::widgets::Widgets;
use crate::widgets::chrome::ChromeWidget;
use crate::widgets::tabs::TabsWidget;
use crate::widgets::tabs::services::{
    close_tab, open_tab,
};

impl App {
    pub(crate) fn new() -> Self {
        let widgets = Widgets {
            chrome: ChromeWidget::new(),
            tabs: TabsWidget::load(),
        };

        Self { widgets }
    }
}
";

    fn name(name: &str) -> WidgetName {
        WidgetName::parse(name).expect("a valid name")
    }

    #[test]
    fn the_widget_module_keeps_the_order_and_the_field_comes_last() {
        let changed = widgets_mod(WIDGETS_MOD_TEXT, &name("notes"))
            .expect("widgets/mod.rs has its anchors");

        assert_eq!(
            changed,
            "\
pub(crate) mod chrome;
pub(crate) mod notes;
pub(crate) mod tabs;

pub(crate) struct Widgets {
    pub(crate) tabs: tabs::TabsWidget,
    pub(crate) chrome: chrome::ChromeWidget,
    pub(crate) notes: notes::NotesWidget,
}
"
        );
    }

    #[test]
    fn the_dispatch_gets_its_import_module_variant_and_arms() {
        let changed = dispatch_mod(DISPATCH_MOD_TEXT, &name("zoom"))
            .expect("dispatch/mod.rs has its anchors");

        assert_eq!(
            changed,
            "\
use iced::Task;

use crate::app::App;
use crate::effects;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::tabs::event::TabsEvent;
use crate::widgets::terminal_workspace::event::{
    TerminalWorkspaceEvent, TerminalWorkspaceIntent,
};
use crate::widgets::zoom::event::ZoomEvent;

pub(crate) mod chrome;
pub(crate) mod command;
pub(crate) mod tabs;
pub(crate) mod terminal_workspace;
pub(crate) mod zoom;

#[derive(Clone)]
pub(crate) enum AppEvent {
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    TerminalWorkspace(TerminalWorkspaceEvent),
    Zoom(ZoomEvent),
    /// Run an app command.
    Command(AppCommand),
    IcedReady,
    SyncTerminalGridSizes,
    Window(iced::window::Event),
}

/// Hand an app event to the one that handles it.
pub(crate) fn dispatch(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::IcedReady => effects::startup::run(app),
        AppEvent::Chrome(ChromeEvent::Intent(intent)) => {
            chrome::deliver(app, intent)
        },
        AppEvent::Chrome(ChromeEvent::Effect(effect)) => {
            effects::chrome::run(effect)
        },
        AppEvent::Tabs(TabsEvent::Intent(intent)) => tabs::deliver(app, intent),
        AppEvent::Tabs(TabsEvent::Effect(effect)) => {
            effects::tabs::run(app, effect)
        },
        AppEvent::Zoom(ZoomEvent::Intent(intent)) => zoom::deliver(app, intent),
        AppEvent::Zoom(ZoomEvent::Effect(effect)) => effects::zoom::run(effect),
        AppEvent::Command(command) => command::deliver(app, command),
        AppEvent::SyncTerminalGridSizes => Task::done(
            AppEvent::TerminalWorkspace(TerminalWorkspaceEvent::Intent(
                TerminalWorkspaceIntent::SyncPaneGridSize,
            )),
        ),
        AppEvent::Window(_) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::sidebar::event::SidebarIntent;
}
"
        );
    }

    #[test]
    fn the_new_arms_go_after_a_widget_arm_on_one_line() {
        let one_line = DISPATCH_MOD_TEXT.replace(
            "        AppEvent::Tabs(TabsEvent::Effect(effect)) => {
            effects::tabs::run(app, effect)
        },
",
            "        AppEvent::Tabs(TabsEvent::Effect(effect)) => effects::tabs::run(app, effect),
",
        );

        let changed = dispatch_mod(&one_line, &name("zoom"))
            .expect("dispatch/mod.rs has its anchors");

        let arms = "        \
AppEvent::Tabs(TabsEvent::Effect(effect)) => effects::tabs::run(app, effect),
        AppEvent::Zoom(ZoomEvent::Intent(intent)) => zoom::deliver(app, intent),
        AppEvent::Zoom(ZoomEvent::Effect(effect)) => effects::zoom::run(effect),
        AppEvent::Command(command) => command::deliver(app, command),
";
        assert!(changed.contains(arms), "{changed}");
    }

    #[test]
    fn a_name_that_app_event_has_already_is_refused() {
        let error = dispatch_mod(DISPATCH_MOD_TEXT, &name("window"))
            .expect_err("AppEvent::Window exists");

        assert_eq!(
            error.to_string(),
            "dispatch/mod.rs: `AppEvent::Window` already exists"
        );
    }

    #[test]
    fn the_effects_module_keeps_the_order() {
        let changed = effects_mod(EFFECTS_MOD_TEXT, &name("notes"))
            .expect("effects/mod.rs has its anchors");

        assert_eq!(
            changed,
            "\
//! The Effects layer.

pub(crate) mod chrome;
pub(crate) mod notes;
pub(crate) mod startup;
pub(crate) mod tabs;
"
        );
    }

    #[test]
    fn the_app_imports_the_widget_in_order_and_builds_it_last() {
        let changed =
            app(APP_TEXT, &name("notes")).expect("app.rs has its anchors");

        assert_eq!(
            changed,
            "\
use crate::widgets::Widgets;
use crate::widgets::chrome::ChromeWidget;
use crate::widgets::notes::NotesWidget;
use crate::widgets::tabs::TabsWidget;
use crate::widgets::tabs::services::{
    close_tab, open_tab,
};

impl App {
    pub(crate) fn new() -> Self {
        let widgets = Widgets {
            chrome: ChromeWidget::new(),
            tabs: TabsWidget::load(),
            notes: NotesWidget::new(),
        };

        Self { widgets }
    }
}
"
        );
    }

    #[test]
    fn a_missing_anchor_names_the_file_and_what_is_missing() {
        let no_variants = DISPATCH_MOD_TEXT
            .replace("    Chrome(ChromeEvent),\n", "")
            .replace("    Tabs(TabsEvent),\n", "")
            .replace("    TerminalWorkspace(TerminalWorkspaceEvent),\n", "");
        let no_arms = DISPATCH_MOD_TEXT
            .replace("AppEvent::Chrome(ChromeEvent::", "AppEvent::Chrome(")
            .replace("AppEvent::Tabs(TabsEvent::", "AppEvent::Tabs(");
        let unclosed_arm = DISPATCH_MOD_TEXT.replace(
            "            effects::tabs::run(app, effect)\n        },\n",
            "            effects::tabs::run(app, effect)\n",
        );
        let unclosed_widgets = APP_TEXT.replace("        };\n", "");

        let cases = [
            (
                widgets_mod("pub(crate) mod tabs;\n", &name("notes")),
                "widgets/mod.rs: cannot find `pub(crate) struct Widgets {`",
            ),
            (
                widgets_mod("pub(crate) struct Widgets {\n}\n", &name("notes")),
                "widgets/mod.rs: cannot find a `pub(crate) mod` line",
            ),
            (
                dispatch_mod(&no_variants, &name("notes")),
                "dispatch/mod.rs: cannot find a widget variant such as \
                 `Tabs(TabsEvent),` in `pub(crate) enum AppEvent {`",
            ),
            (
                dispatch_mod(&no_arms, &name("notes")),
                "dispatch/mod.rs: cannot find a widget arm such as \
                 `AppEvent::Tabs(TabsEvent::Effect(effect)) =>` in \
                 `pub(crate) fn dispatch(`",
            ),
            (
                dispatch_mod(&unclosed_arm, &name("notes")),
                "dispatch/mod.rs: cannot find the end of the arm \
                 `AppEvent::Tabs(TabsEvent::Effect(effect)) => {`",
            ),
            (
                effects_mod("//! The Effects layer.\n", &name("notes")),
                "effects/mod.rs: cannot find a `pub(crate) mod` line",
            ),
            (
                app(&unclosed_widgets, &name("notes")),
                "app.rs: cannot find the end of \
                 `let widgets = Widgets {`",
            ),
        ];

        for (result, missing) in cases {
            let error = result.expect_err(missing);

            assert_eq!(
                error.to_string(),
                format!("{missing}; nothing was changed")
            );
        }
    }
}
