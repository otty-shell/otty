//! The widget generator behind `cargo xtask new-widget <name>`.
//!
//! It creates the files every widget has and the widget's route, as in
//! «Пример виджета» of `docs/internal/architecture.md`, and wires the widget
//! into `widgets/mod.rs`, `router/mod.rs` and `app.rs`. Every file is
//! prepared in memory first, so a bad name or a missing anchor stops it
//! before anything is written. Showing the widget is left to the author.

mod error;
mod name;
mod templates;
mod wiring;

use std::fs;
use std::path::Path;

pub use error::NewWidgetError;
use name::WidgetName;

/// A file of `app/src` and the contents the generator writes to it.
#[derive(Debug)]
struct FileWrite {
    path: String,
    contents: String,
}

/// A widget the generator created, with the files it wrote.
#[derive(Debug)]
pub struct NewWidget {
    name: WidgetName,
    created: Vec<FileWrite>,
    changed: Vec<FileWrite>,
}

impl NewWidget {
    /// The new files, relative to `app/src`.
    pub fn created(&self) -> impl Iterator<Item = &str> {
        self.created.iter().map(|file| file.path.as_str())
    }

    /// The existing files the widget was wired into, relative to `app/src`.
    pub fn changed(&self) -> impl Iterator<Item = &str> {
        self.changed.iter().map(|file| file.path.as_str())
    }

    /// Prepare every file of the widget `name` without writing any.
    fn plan(app_src: &Path, name: &str) -> Result<Self, NewWidgetError> {
        let name = WidgetName::parse(name)?;
        let snake = name.snake();

        let taken = [
            format!("widgets/{snake}/"),
            format!("widgets/{snake}.rs"),
            format!("router/{snake}.rs"),
        ];
        if let Some(path) =
            taken.iter().find(|path| app_src.join(path).exists())
        {
            return Err(NewWidgetError::NameTaken(format!("`{path}`")));
        }

        let widgets_mod = read(app_src, wiring::WIDGETS_MOD)?;
        let router_mod = read(app_src, wiring::ROUTER_MOD)?;
        let app = read(app_src, wiring::APP)?;

        let changed = vec![
            FileWrite {
                path: wiring::WIDGETS_MOD.to_string(),
                contents: wiring::widgets_mod(&widgets_mod, &name)?,
            },
            FileWrite {
                path: wiring::ROUTER_MOD.to_string(),
                contents: wiring::router_mod(&router_mod, &name)?,
            },
            FileWrite {
                path: wiring::APP.to_string(),
                contents: wiring::app(&app, &name)?,
            },
        ];

        Ok(Self {
            created: templates::new_files(&name),
            changed,
            name,
        })
    }

    /// Write the prepared files into `app_src`: the new ones, then the
    /// changed ones.
    fn write(&self, app_src: &Path) -> Result<(), NewWidgetError> {
        for file in self.created.iter().chain(&self.changed) {
            let path = app_src.join(&file.path);

            let dir = path.parent().unwrap_or(app_src);
            let written = fs::create_dir_all(dir)
                .and_then(|()| fs::write(&path, &file.contents));
            if let Err(source) = written {
                return Err(NewWidgetError::Write { path, source });
            }
        }

        Ok(())
    }

    /// What the generator did and what the author does next, for the
    /// terminal.
    pub fn summary(&self) -> String {
        let snake = self.name.snake();
        let camel = self.name.camel();
        let mut text = format!("Created the widget `{snake}` in app/src:\n");

        for path in self.created() {
            text.push_str(&format!("  {path}\n"));
        }
        text.push_str("Wired it into:\n");
        for path in self.changed() {
            text.push_str(&format!("  {path}\n"));
        }

        text.push_str(&format!(
            "
Next steps:
  1. Run `cargo +nightly fmt`: the generator does not wrap long lines.
  2. Show the widget in view.rs if it is visible in every mode, or in the
     file of its mode in view/:

         use crate::widgets::{snake}::event::{camel}Event;
         use crate::widgets::{snake}::view::{snake}_panel;

         {snake}_panel::view({snake}_panel::{camel}PanelProps {{
             vm: app.widgets.{snake}.vm(),
             theme: theme_props,
         }})
         .map(|intent| AppEvent::{camel}({camel}Event::Intent(intent)))

  3. If the widget needs a subscription, add it to subscription.rs.

Until the widget is shown, the compiler warns about dead code in it: the
view, `vm()`, the view model and the `Intent` variant. Two warnings stay
after that: the fields of the panel props, until the view reads them, and
the `Effect` variant, until the reducer returns an effect.
"
        ));

        text
    }
}

/// Create the widget `name` in `app_src`, the `app/src` folder, and wire it
/// in. Nothing is written when the name or an existing file stops it.
///
/// ```no_run
/// use std::path::Path;
///
/// use otty_xtask::new_widget::{self, NewWidgetError};
///
/// let widget = new_widget::create(Path::new("app/src"), "notes")?;
/// print!("{}", widget.summary());
/// # Ok::<(), NewWidgetError>(())
/// ```
pub fn create(app_src: &Path, name: &str) -> Result<NewWidget, NewWidgetError> {
    let widget = NewWidget::plan(app_src, name)?;
    widget.write(app_src)?;

    Ok(widget)
}

fn read(app_src: &Path, file: &str) -> Result<String, NewWidgetError> {
    let path = app_src.join(file);

    fs::read_to_string(&path)
        .map_err(|source| NewWidgetError::Read { path, source })
}
