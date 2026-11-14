use iced::widget::text::Wrapping;
use iced::widget::{column, container, mouse_area, row, scrollable, svg, text};
use iced::{Alignment, Element, Length, Padding};
use otty_app_i18n::{self as i18n, Key};
use otty_app_theme::icons::FILE;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::target_dot::{TargetKind, target_dot};
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};
use otty_ui_tree::{TreeRowContext, TreeView};

use super::name_field;
use crate::event::ExplorerIntent;
use crate::interaction::DropTarget;
use crate::model::SidebarTreeViewModel;
use crate::types::FileNode;

/// Props for the explorer sidebar tree view.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeProps<'a> {
    pub(crate) vm: SidebarTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the Files section: header, folder line and the tree.
pub(crate) fn view(props: SidebarTreeProps<'_>) -> Element<'_, ExplorerIntent> {
    let ui = props.theme.theme.ui();
    let folder_line = folder_line(props.vm.root_label, ui);
    let tree_list = explorer_tree(&props.vm, ui);

    let body = column![folder_line, tree_list]
        .width(Length::Fill)
        .height(Length::Fill);

    PanelSection::new(i18n::t(Key::FilesTitle), ui)
        .open(true)
        .grow(true)
        .body(body)
        .into()
}

/// Render the muted line: machine dot and folder name in mono.
fn folder_line<'a>(
    root_label: Option<&'a str>,
    palette: &UiPalette,
) -> Element<'a, ExplorerIntent> {
    let style = typography::SMALL;
    let folder = root_label.unwrap_or(i18n::t(Key::NoActiveFolder));

    // The panel follows only tabs that run on this computer (shells and
    // local launches, not SSH), so the folder is local.
    let dot = target_dot(TargetKind::Local, palette);
    let folder = text(folder)
        .size(style.size())
        .font(fonts::MONO)
        .color(palette.text_body())
        .wrapping(Wrapping::None);

    row![dot, folder]
        .spacing(spacing::SPACE_2)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_2]))
        .into()
}

/// Render the scrollable explorer tree. A press, a release and a right
/// click on a row or on the empty area under the rows go to the explorer,
/// and so does the pointer, for a drag. The name field shows in place of a
/// renamed row or under the folder of a new entry, and the folder a dragged
/// row would land in is lit.
fn explorer_tree<'a>(
    vm: &SidebarTreeViewModel<'a>,
    ui: &UiPalette,
) -> Element<'a, ExplorerIntent> {
    let row_palette = ui.clone();
    let error_color = ui.text_danger();
    let inline_edit = vm.inline_edit;
    let drop_target = vm.drop_target;

    let tree_view = TreeView::new(vm.tree, move |context| {
        render_entry(context, drop_target, &row_palette)
    })
    .selected_row(vm.selected_path)
    .hovered_row(vm.hovered_path)
    .on_press(|path| ExplorerIntent::NodePressed { path })
    .on_release(|path| ExplorerIntent::NodeReleased { path })
    .on_right_press(|path| ExplorerIntent::NodeRightClicked { path })
    .on_hover(|path| ExplorerIntent::NodeHovered { path })
    .row_visible_filter(move |context| {
        !name_field::is_renamed_row(inline_edit, context)
    })
    .after_row(move |context| {
        name_field::after_row(inline_edit, context, error_color)
    })
    .spacing(0.0);

    let content = match name_field::at_top(inline_edit, error_color) {
        Some(field) => column![field, tree_view.view()].into(),
        None => tree_view.view(),
    };

    let scrollable = scrollable::Scrollable::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(ui));

    let tree_area = mouse_area(scrollable)
        .on_move(|position| ExplorerIntent::CursorMoved { position })
        .on_press(ExplorerIntent::BackgroundPressed)
        .on_release(ExplorerIntent::BackgroundReleased)
        .on_right_press(ExplorerIntent::BackgroundRightClicked);

    // A row dragged away from the rows lands in the followed folder: the
    // whole tree is lit.
    let root_drop = matches!(drop_target, Some(DropTarget::Root))
        .then(|| ui.surface_selected());

    container(tree_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::default().bottom(spacing::SPACE_6))
        .style(move |_| container::Style {
            background: root_drop.map(Into::into),
            ..Default::default()
        })
        .into()
}

fn render_entry<'a>(
    context: &TreeRowContext<'a, FileNode>,
    drop_target: Option<&DropTarget>,
    palette: &UiPalette,
) -> Element<'a, ExplorerIntent> {
    let node = context.entry.node;

    let row = TreeRow::new(node.name(), palette)
        .depth(context.entry.depth as u16)
        .selected(context.is_selected)
        .hovered(context.is_hovered)
        .drop_target(is_in_drop_folder(drop_target, &context.entry.path));
    let row = if node.is_folder() {
        row.caret(node.is_expanded())
    } else {
        row.icon(svg::Handle::from_memory(FILE), None)
    };

    row.into()
}

/// Whether the row at `path` is the folder a dragged row would land in, or
/// inside it: they light up together.
fn is_in_drop_folder(
    drop_target: Option<&DropTarget>,
    path: &[String],
) -> bool {
    match drop_target {
        Some(DropTarget::Folder(folder)) => path.starts_with(folder),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;
    use otty_app_theme::theme::{ThemeManager, ThemeProps};

    use super::{SidebarTreeProps, view};
    use crate::event::ExplorerIntent;
    use crate::interaction::{InlineEdit, InlineEditKind};
    use crate::model::SidebarTreeViewModel;
    use crate::types::FileNode;

    fn props<'a>(
        themes: &'a ThemeManager,
        tree: &'a [FileNode],
    ) -> SidebarTreeProps<'a> {
        SidebarTreeProps {
            vm: SidebarTreeViewModel {
                root_label: Some("otty"),
                tree,
                selected_path: None,
                hovered_path: None,
                inline_edit: None,
                drop_target: None,
            },
            theme: ThemeProps::new(themes.current()),
        }
    }

    #[test]
    fn given_open_folder_when_panel_drawn_then_it_shows_title_and_folder_only()
    {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(props(&themes, &[])));

        assert!(ui.find("Files").is_ok());
        assert!(ui.find("otty").is_ok());
        assert!(ui.find("follows zsh").is_err());
    }

    #[test]
    fn given_long_folder_name_in_minimum_panel_when_drawn_then_it_stays_inside()
    {
        const WIDTH: f32 = 128.0;
        const LONG: &str = "a-very-long-project-folder-name-that-cannot-fit";
        let themes = ThemeManager::new();
        let mut long_props = props(&themes, &[]);
        long_props.vm.root_label = Some(LONG);
        let mut ui =
            simulator(iced::widget::container(view(long_props)).width(WIDTH));

        let folder = ui.find(LONG).expect("folder should show").bounds();

        assert!(
            folder.x + folder.width <= WIDTH + 0.5,
            "the folder name must stay inside the panel: {folder:?}"
        );
    }

    #[test]
    fn given_a_file_row_when_clicked_then_node_pressed_is_sent() {
        let themes = ThemeManager::new();
        let tree = [FileNode::new(
            String::from("main.rs"),
            PathBuf::from("/tmp/main.rs"),
            false,
        )];
        let mut ui = simulator(view(props(&themes, &tree)));

        ui.click("main.rs").expect("row should be visible");

        let intents: Vec<ExplorerIntent> = ui.into_messages().collect();
        assert!(intents.iter().any(|intent| matches!(
            intent,
            ExplorerIntent::NodePressed { .. }
        )));
        assert!(intents.iter().any(|intent| matches!(
            intent,
            ExplorerIntent::NodeReleased { .. }
        )));
    }

    /// A right click on a row asks for the menu of that row, not of the
    /// empty area under it.
    #[test]
    fn given_a_row_when_right_clicked_then_its_menu_is_asked_for() {
        let themes = ThemeManager::new();
        let tree = [FileNode::new(
            String::from("main.rs"),
            PathBuf::from("/tmp/main.rs"),
            false,
        )];
        let mut ui = simulator(view(props(&themes, &tree)));
        let row = ui.find("main.rs").expect("row should be visible").bounds();

        ui.point_at(row.center());
        let _ = ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        let intents: Vec<ExplorerIntent> = ui.into_messages().collect();
        assert!(intents.iter().any(|intent| matches!(
            intent,
            ExplorerIntent::NodeRightClicked { path } if path == &["main.rs"]
        )));
        assert!(!intents.iter().any(|intent| matches!(
            intent,
            ExplorerIntent::BackgroundRightClicked
        )));
    }

    /// A rename shows the name field with the old name in place of the row.
    #[test]
    fn given_a_renamed_row_when_drawn_then_the_name_field_takes_its_place() {
        let themes = ThemeManager::new();
        let tree = [FileNode::new(
            String::from("main.rs"),
            PathBuf::from("/tmp/main.rs"),
            false,
        )];
        let edit = InlineEdit::new(
            InlineEditKind::Rename {
                path: vec![String::from("main.rs")],
            },
            String::from("app.rs"),
        );
        let mut renaming = props(&themes, &tree);
        renaming.vm.inline_edit = Some(&edit);
        let mut ui = simulator(view(renaming));

        assert!(ui.find("main.rs").is_err());
        assert!(ui.find(edit.id.clone()).is_ok());
    }
}
