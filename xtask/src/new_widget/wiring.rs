//! The edits that wire a new widget into `widgets/mod.rs`, `router/mod.rs`
//! and `app.rs`.

use super::error::NewWidgetError;
use super::name::WidgetName;

/// The files this module edits, relative to `app/src`.
pub(super) const WIDGETS_MOD: &str = "widgets/mod.rs";
pub(super) const ROUTER_MOD: &str = "router/mod.rs";
pub(super) const APP: &str = "app.rs";

/// Module declarations; rustfmt keeps them in alphabetical order.
const MODULE: &str = "pub(crate) mod ";

/// Imports from the widgets; rustfmt keeps them in alphabetical order.
const WIDGET_IMPORT: &str = "use crate::widgets::";

/// The blocks that get one line per widget.
const WIDGETS_STRUCT: &str = "pub(crate) struct Widgets {";
const APP_EVENT_ENUM: &str = "pub(crate) enum AppEvent {";
const ROUTE_FN: &str = "pub(crate) fn route(";
const WIDGETS_LITERAL: &str = "let widgets = Widgets {";

/// How errors name the lines that the variant and the arm go after.
const WIDGET_VARIANT: &str = "a widget variant such as `Tabs(TabsEvent),` \
                              in `pub(crate) enum AppEvent {`";
const WIDGET_ARM: &str = "a widget arm such as \
                          `AppEvent::Tabs(event) => tabs::route(app, event),` \
                          in `pub(crate) fn route(`";

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

/// `router/mod.rs` with the event import, the route module, the
/// `AppEvent` variant and its arm in `route`.
pub(super) fn router_mod(
    text: &str,
    name: &WidgetName,
) -> Result<String, NewWidgetError> {
    let snake = name.snake();
    let camel = name.camel();
    let mut lines = lines_of(text);

    let (start, end) = block(&lines, ROUTER_MOD, APP_EVENT_ENUM)?;
    let variants = &lines[start + 1..end];
    if variants.iter().any(|line| names_variant(line, camel)) {
        let taken = format!("{ROUTER_MOD}: `AppEvent::{camel}`");
        return Err(NewWidgetError::NameTaken(taken));
    }

    let import = format!("{WIDGET_IMPORT}{snake}::event::{camel}Event;");
    insert_sorted(&mut lines, ROUTER_MOD, WIDGET_IMPORT, import)?;
    let module = format!("{MODULE}{snake};");
    insert_sorted(&mut lines, ROUTER_MOD, MODULE, module)?;
    let variant = format!("{camel}({camel}Event),");
    insert_after_last_widget_line(
        &mut lines,
        APP_EVENT_ENUM,
        is_widget_variant,
        &variant,
        WIDGET_VARIANT,
    )?;
    let arm =
        format!("AppEvent::{camel}(event) => {snake}::route(app, event),");
    insert_after_last_widget_line(
        &mut lines,
        ROUTE_FN,
        is_widget_arm,
        &arm,
        WIDGET_ARM,
    )?;

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

/// Add `code` after the last line of the block `opening` in `router/mod.rs`
/// that `is_widget_line` accepts, with the same indent.
fn insert_after_last_widget_line(
    lines: &mut Vec<String>,
    opening: &str,
    is_widget_line: fn(&str) -> bool,
    code: &str,
    missing: &str,
) -> Result<(), NewWidgetError> {
    let (start, end) = block(lines, ROUTER_MOD, opening)?;
    let Some(last) =
        (start + 1..end).rfind(|index| is_widget_line(&lines[*index]))
    else {
        return Err(anchor_not_found(ROUTER_MOD, missing.to_string()));
    };

    let indent = indent_of(&lines[last]);
    let line = format!("{indent}{code}");
    lines.insert(last + 1, line);

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

    let indent = indent_of(&lines[start]);
    let end = (start + 1..lines.len())
        .find(|index| {
            lines[*index]
                .strip_prefix(indent)
                .is_some_and(|rest| rest == "}" || rest == "};")
        })
        .ok_or_else(|| {
            anchor_not_found(file, format!("the end of `{opening}`"))
        })?;

    Ok((start, end))
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

/// Whether `line` is a widget arm such as
/// `AppEvent::Tabs(event) => tabs::route(app, event),`.
fn is_widget_arm(line: &str) -> bool {
    let text = line.trim();

    text.starts_with("AppEvent::") && text.ends_with("::route(app, event),")
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

    const ROUTER_MOD_TEXT: &str = "\
use iced::Task;

use crate::app::App;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::tabs::event::{
    TabsEvent, TabsIntent,
};

pub(crate) mod chrome;
pub(crate) mod command;
pub(crate) mod tabs;

#[derive(Clone)]
pub(crate) enum AppEvent {
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    /// Run an app command.
    Command(AppCommand),
    Window(iced::window::Event),
}

/// Route an app event to the handler of the widget it belongs to.
pub(crate) fn route(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::Chrome(event) => chrome::route(app, event),
        AppEvent::Tabs(event) => tabs::route(app, event),
        AppEvent::Command(command) => command::route(app, command),
        AppEvent::Window(_) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::tabs::event::TabsIntent;
}
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
    fn the_route_gets_its_import_module_variant_and_arm() {
        let changed = router_mod(ROUTER_MOD_TEXT, &name("zoom"))
            .expect("router/mod.rs has its anchors");

        assert_eq!(
            changed,
            "\
use iced::Task;

use crate::app::App;
use crate::widgets::chrome::event::ChromeEvent;
use crate::widgets::tabs::event::{
    TabsEvent, TabsIntent,
};
use crate::widgets::zoom::event::ZoomEvent;

pub(crate) mod chrome;
pub(crate) mod command;
pub(crate) mod tabs;
pub(crate) mod zoom;

#[derive(Clone)]
pub(crate) enum AppEvent {
    Chrome(ChromeEvent),
    Tabs(TabsEvent),
    Zoom(ZoomEvent),
    /// Run an app command.
    Command(AppCommand),
    Window(iced::window::Event),
}

/// Route an app event to the handler of the widget it belongs to.
pub(crate) fn route(app: &mut App, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::Chrome(event) => chrome::route(app, event),
        AppEvent::Tabs(event) => tabs::route(app, event),
        AppEvent::Zoom(event) => zoom::route(app, event),
        AppEvent::Command(command) => command::route(app, command),
        AppEvent::Window(_) => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::tabs::event::TabsIntent;
}
"
        );
    }

    #[test]
    fn a_name_that_app_event_has_already_is_refused() {
        let error = router_mod(ROUTER_MOD_TEXT, &name("window"))
            .expect_err("AppEvent::Window exists");

        assert_eq!(
            error.to_string(),
            "router/mod.rs: `AppEvent::Window` already exists"
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
        let no_variants = ROUTER_MOD_TEXT
            .replace("    Chrome(ChromeEvent),\n", "")
            .replace("    Tabs(TabsEvent),\n", "");
        let no_arms = ROUTER_MOD_TEXT
            .replace(
                "AppEvent::Chrome(event) => chrome::route(app, event),",
                "",
            )
            .replace("AppEvent::Tabs(event) => tabs::route(app, event),", "");
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
                router_mod(&no_variants, &name("notes")),
                "router/mod.rs: cannot find a widget variant such as \
                 `Tabs(TabsEvent),` in `pub(crate) enum AppEvent {`",
            ),
            (
                router_mod(&no_arms, &name("notes")),
                "router/mod.rs: cannot find a widget arm such as \
                 `AppEvent::Tabs(event) => tabs::route(app, event),` in \
                 `pub(crate) fn route(`",
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
