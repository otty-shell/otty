use iced::widget::text::Wrapping;
use iced::widget::{column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Length, Padding};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::scrollbar;
use otty_ui_kit::components::target_dot::{TargetKind, target_dot};
use otty_ui_kit::components::tree_row::TreeRow;
use otty_ui_kit::palette::UiPalette;
use otty_ui_kit::{fonts, spacing, typography};
use otty_ui_tree::{TreeRowContext, TreeView};

use super::super::event::ExplorerIntent;
use super::super::model::ExplorerTreeViewModel;
use super::super::types::FileNode;
use crate::i18n::{self, Key};
use crate::icons::FILE;
use crate::theme::ThemeProps;

/// Props for the explorer sidebar tree view.
#[derive(Debug, Clone)]
pub(crate) struct SidebarTreeProps<'a> {
    pub(crate) vm: ExplorerTreeViewModel<'a>,
    /// Name of the shell the active tab runs; shown as "follows <shell>".
    pub(crate) shell_name: &'a str,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the Files section: header, folder line and the tree.
pub(crate) fn view(
    props: SidebarTreeProps<'_>,
) -> Element<'_, ExplorerIntent, iced::Theme, iced::Renderer> {
    let ui = props.theme.theme.ui();
    let folder_line = folder_line(props.vm.root_label, props.shell_name, ui);
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

/// Render the muted line: machine dot, folder in mono, followed shell.
fn folder_line<'a>(
    root_label: Option<&'a str>,
    shell_name: &str,
    palette: &UiPalette,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let style = typography::SMALL;
    let folder = root_label.unwrap_or(i18n::t(Key::NoActiveFolder));

    // The panel follows shell tabs only, and shell tabs always run on this
    // computer (SSH launches open command tabs), so the folder is local.
    let dot = target_dot(TargetKind::Local, palette);
    let folder = text(folder)
        .size(style.size())
        .font(fonts::MONO)
        .color(palette.text_body())
        .wrapping(Wrapping::None);
    let follows = text(i18n::files_follows(shell_name))
        .size(style.size())
        .font(fonts::UI)
        .color(palette.text_muted())
        .wrapping(Wrapping::None);

    row![dot, folder, follows]
        .spacing(spacing::SPACE_2)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .padding(Padding::from([spacing::SPACE_1, spacing::SPACE_2]))
        .into()
}

/// Render the scrollable explorer tree.
fn explorer_tree<'a>(
    vm: &ExplorerTreeViewModel<'a>,
    ui: &UiPalette,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let row_palette = ui.clone();
    let tree_view = TreeView::new(vm.tree, move |context| {
        render_entry(context, &row_palette)
    })
    .selected_row(vm.selected_path)
    .hovered_row(vm.hovered_path)
    .on_press(|path| ExplorerIntent::NodePressed { path })
    .on_hover(|path| ExplorerIntent::NodeHovered { path })
    .spacing(0.0);

    let scrollable = scrollable::Scrollable::new(tree_view.view())
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(scrollbar::vertical())
        .style(scrollbar::style(ui));

    container(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::default().bottom(spacing::SPACE_6))
        .into()
}

fn render_entry<'a>(
    context: &TreeRowContext<'a, FileNode>,
    palette: &UiPalette,
) -> Element<'a, ExplorerIntent, iced::Theme, iced::Renderer> {
    let node = context.entry.node;

    let row = TreeRow::new(node.name(), palette)
        .depth(context.entry.depth as u16)
        .selected(context.is_selected)
        .hovered(context.is_hovered);
    let row = if node.is_folder() {
        row.caret(node.is_expanded())
    } else {
        row.icon(svg::Handle::from_memory(FILE), None)
    };

    row.into()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;

    use super::{SidebarTreeProps, view};
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::explorer::event::ExplorerIntent;
    use crate::widgets::explorer::model::ExplorerTreeViewModel;
    use crate::widgets::explorer::types::FileNode;

    fn props<'a>(
        themes: &'a ThemeManager,
        tree: &'a [FileNode],
    ) -> SidebarTreeProps<'a> {
        SidebarTreeProps {
            vm: ExplorerTreeViewModel {
                root_label: Some("otty"),
                tree,
                selected_path: None,
                hovered_path: None,
            },
            shell_name: "zsh",
            theme: ThemeProps::new(themes.current()),
        }
    }

    #[test]
    fn given_open_folder_when_panel_drawn_then_it_shows_title_folder_and_shell()
    {
        let themes = ThemeManager::new();
        let mut ui = simulator(view(props(&themes, &[])));

        assert!(ui.find("Files").is_ok());
        assert!(ui.find("otty").is_ok());
        assert!(ui.find("follows zsh").is_ok());
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
    }
}
