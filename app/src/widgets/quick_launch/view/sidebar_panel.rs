use iced::Element;
use iced::widget::svg;
use otty_ui_kit::components::icon_button::{IconButton, IconButtonSize};
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::size;
use otty_ui_tree::flatten_tree;

use super::super::event::QuickLaunchIntent;
use super::super::model::QuickLaunchTreeViewModel;
use super::super::types::InlineEditKind;
use super::sidebar_tree;
use crate::i18n::{self, Key};
use crate::icons;
use crate::theme::ThemeProps;

/// Props for the Launches section of the left panel.
///
/// The section is generic over the message type because its header toggle
/// belongs to the sidebar, while the tree and the header buttons send
/// quick launch intents.
pub(crate) struct SidebarPanelProps<'a, Message> {
    pub(crate) vm: QuickLaunchTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
    /// Whether the section body is shown.
    pub(crate) is_open: bool,
    /// Message sent when the section title is pressed.
    pub(crate) on_toggle: Message,
    /// Turns a quick launch intent into the caller's message.
    pub(crate) on_intent: fn(QuickLaunchIntent) -> Message,
}

/// Render the Launches section: header with New folder and New launch
/// buttons, and the launch tree as its body.
pub(crate) fn view<'a, Message: Clone + 'a>(
    props: SidebarPanelProps<'a, Message>,
) -> Element<'a, Message> {
    let palette = props.theme.theme.ui();
    let on_intent = props.on_intent;

    let new_folder = IconButton::new(
        svg::Handle::from_memory(icons::FOLDER_PLUS),
        i18n::t(Key::CreateFolder),
        palette,
    )
    .size(IconButtonSize::Small)
    .on_press(on_intent(QuickLaunchIntent::HeaderCreateFolder));

    let new_launch = IconButton::new(
        svg::Handle::from_memory(icons::FILE_PLUS),
        i18n::t(Key::CreateQuickLaunch),
        palette,
    )
    .size(IconButtonSize::Small)
    .on_press(on_intent(QuickLaunchIntent::HeaderCreateCommand));

    let launch_count = props.vm.data.launch_count();

    let tree = sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        data: props.vm.data,
        selected_path: props.vm.selected_path,
        hovered_path: props.vm.hovered_path,
        inline_edit: props.vm.inline_edit,
        launching: props.vm.launching,
        drop_target: props.vm.drop_target,
        theme: props.theme,
    })
    .map(on_intent);

    let mut section =
        PanelSection::new(i18n::t(Key::LaunchesPanelTitle), palette)
            .open(props.is_open)
            .grow(true)
            .action(new_folder)
            .action(new_launch)
            .body(tree)
            .on_toggle(props.on_toggle);
    if !props.is_open {
        section = section.count(launch_count);
    }

    section.into()
}

/// Height of the launch tree: one row per visible launch or folder, and one
/// more while a new folder name is typed.
///
/// Counted from the rows because iced cannot measure a widget before the
/// left panel shares its height between the sections. An error line under a name input is not counted: Launches is the last
/// section and takes the height left under the others anyway.
pub(crate) fn content_height(vm: &QuickLaunchTreeViewModel<'_>) -> f32 {
    let visible_rows = flatten_tree(vm.data.root().children()).len();
    let is_creating_folder = matches!(
        vm.inline_edit.map(|edit| &edit.kind),
        Some(InlineEditKind::CreateFolder { .. })
    );

    let rows = if is_creating_folder {
        visible_rows + 1
    } else {
        visible_rows
    };
    rows as f32 * size::ROW
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use iced::Point;
    use iced_test::simulator;
    use otty_ui_kit::size;

    use super::{SidebarPanelProps, content_height, view};
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::theme::{ThemeManager, ThemeProps};
    use crate::widgets::quick_launch::event::QuickLaunchIntent;
    use crate::widgets::quick_launch::model::QuickLaunchTreeViewModel;
    use crate::widgets::quick_launch::state::InlineEditState;
    use crate::widgets::quick_launch::types::{
        CommandSpec, CustomCommand, InlineEditKind, LaunchInfo, QuickLaunch,
        QuickLaunchFile, QuickLaunchNode,
    };

    /// Width of the panel in the tests.
    const PANEL_WIDTH: f32 = 260.0;
    /// Section padding 8, header row 30, gap 1: the top of the rows.
    const BODY_TOP: f32 = 8.0 + 30.0 + 1.0;
    /// Section padding 8, header row 30 high: the middle of the header.
    const HEADER_Y: f32 = 8.0 + 15.0;

    #[derive(Debug, Clone)]
    enum Message {
        Toggle,
        Intent(QuickLaunchIntent),
    }

    fn file_with_one_launch() -> QuickLaunchFile {
        let mut file = QuickLaunchFile::default();
        file.root.children = vec![QuickLaunchNode::Command(QuickLaunch {
            title: String::from("build"),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("make"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
            exit_behavior: ExitBehavior::Close,
        })];
        file
    }

    /// A "Work" folder with two launches, then "htop" at the root.
    fn file_with_folder(is_expanded: bool) -> QuickLaunchFile {
        let launch = |title: &str| {
            format!(
                r#"{{"kind": "Command", "title": "{title}",
                    "spec": {{"type": "Custom", "custom": {{"program": "{title}",
                      "args": [], "env": [], "working_directory": null}}}}}}"#
            )
        };
        let json = format!(
            r#"{{"version": 1, "root": {{"title": "Root", "expanded": true,
                "children": [
                  {{"kind": "Folder", "title": "Work", "expanded": {is_expanded},
                    "children": [{}, {}]}},
                  {}
                ]}}}}"#,
            launch("build"),
            launch("deploy"),
            launch("htop"),
        );

        serde_json::from_str(&json).expect("launch fixture should parse")
    }

    fn tree_vm<'a>(
        file: &'a QuickLaunchFile,
        launching: &'a HashMap<Vec<String>, LaunchInfo>,
        inline_edit: Option<&'a InlineEditState>,
    ) -> QuickLaunchTreeViewModel<'a> {
        QuickLaunchTreeViewModel {
            data: file,
            selected_path: None,
            hovered_path: None,
            inline_edit,
            launching,
            drop_target: None,
        }
    }

    fn panel<'a>(
        file: &'a QuickLaunchFile,
        launching: &'a HashMap<Vec<String>, LaunchInfo>,
        themes: &'a ThemeManager,
        is_open: bool,
    ) -> iced::Element<'a, Message> {
        let vm = QuickLaunchTreeViewModel {
            data: file,
            selected_path: None,
            hovered_path: None,
            inline_edit: None,
            launching,
            drop_target: None,
        };

        iced::widget::container(view(SidebarPanelProps {
            vm,
            theme: ThemeProps::new(themes.current()),
            is_open,
            on_toggle: Message::Toggle,
            on_intent: Message::Intent,
        }))
        .width(PANEL_WIDTH)
        .into()
    }

    fn click_header_at(x: f32, is_open: bool) -> Vec<Message> {
        let file = file_with_one_launch();
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut ui = simulator(panel(&file, &launching, &themes, is_open));

        ui.point_at(Point::new(x, HEADER_Y));
        let _ = ui.simulate(iced_test::simulator::click());

        ui.into_messages().collect()
    }

    #[test]
    fn given_header_when_title_pressed_then_section_toggles() {
        let file = file_with_one_launch();
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut ui = simulator(panel(&file, &launching, &themes, true));

        ui.click("Launches").expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(messages.as_slice(), [Message::Toggle]));
    }

    #[test]
    fn given_header_when_new_launch_pressed_then_header_create_command_is_sent()
    {
        let messages = click_header_at(PANEL_WIDTH - 24.0, true);

        assert!(matches!(
            messages.as_slice(),
            [Message::Intent(QuickLaunchIntent::HeaderCreateCommand)]
        ));
    }

    #[test]
    fn given_header_when_new_folder_pressed_then_header_create_folder_is_sent()
    {
        let messages = click_header_at(PANEL_WIDTH - 54.0, true);

        assert!(matches!(
            messages.as_slice(),
            [Message::Intent(QuickLaunchIntent::HeaderCreateFolder)]
        ));
    }

    #[test]
    fn given_open_state_when_viewed_then_launches_show_only_while_open() {
        let file = file_with_one_launch();
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut open = simulator(panel(&file, &launching, &themes, true));
        let mut closed = simulator(panel(&file, &launching, &themes, false));

        assert!(open.find("build").is_ok());
        assert!(closed.find("build").is_err());
    }

    #[test]
    fn given_closed_section_when_viewed_then_header_shows_launch_count() {
        let file = file_with_one_launch();
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut open = simulator(panel(&file, &launching, &themes, true));
        let mut closed = simulator(panel(&file, &launching, &themes, false));

        assert!(open.find("1").is_err());
        assert!(closed.find("1").is_ok());
    }

    #[test]
    fn given_active_launch_when_viewed_then_row_shows_running_word() {
        let file = file_with_one_launch();
        let themes = ThemeManager::new();
        let idle = HashMap::new();
        let mut active = HashMap::new();
        active.insert(
            vec![String::from("build")],
            LaunchInfo {
                id: 1,
                launch_ticks: 0,
                is_indicator_highlighted: false,
                cancel: Default::default(),
            },
        );
        let mut idle_ui = simulator(panel(&file, &idle, &themes, true));
        let mut active_ui = simulator(panel(&file, &active, &themes, true));

        assert!(idle_ui.find("running").is_err());
        assert!(active_ui.find("running").is_ok());
    }

    #[test]
    fn given_tree_when_content_height_read_then_it_counts_visible_rows() {
        let expanded = file_with_folder(true);
        let collapsed = file_with_folder(false);
        let launching = HashMap::new();

        let expanded_height =
            content_height(&tree_vm(&expanded, &launching, None));
        let collapsed_height =
            content_height(&tree_vm(&collapsed, &launching, None));

        assert_eq!(expanded_height, 4.0 * size::ROW);
        assert_eq!(collapsed_height, 2.0 * size::ROW);
    }

    #[test]
    fn given_tree_when_content_height_read_then_it_ends_at_the_last_row() {
        let file = file_with_folder(true);
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut ui = simulator(panel(&file, &launching, &themes, true));

        let content = content_height(&tree_vm(&file, &launching, None));
        let last = ui.find("htop").expect("last row should be visible");

        let row_middle = BODY_TOP + content - size::ROW / 2.0;
        assert!(
            (last.bounds().center_y() - row_middle).abs() < 1.0,
            "last row middle {} should be at {row_middle}",
            last.bounds().center_y()
        );
    }

    #[test]
    fn given_new_folder_input_when_content_height_read_then_it_adds_a_row() {
        let file = file_with_folder(false);
        let launching = HashMap::new();
        let create = InlineEditState {
            kind: InlineEditKind::CreateFolder {
                parent_path: Vec::new(),
            },
            value: String::new(),
            error: None,
            id: iced::widget::Id::unique(),
        };
        let rename = InlineEditState {
            kind: InlineEditKind::Rename {
                path: vec![String::from("htop")],
            },
            value: String::new(),
            error: None,
            id: iced::widget::Id::unique(),
        };

        let creating =
            content_height(&tree_vm(&file, &launching, Some(&create)));
        let renaming =
            content_height(&tree_vm(&file, &launching, Some(&rename)));

        assert_eq!(creating, 3.0 * size::ROW);
        assert_eq!(renaming, 2.0 * size::ROW);
    }
}
