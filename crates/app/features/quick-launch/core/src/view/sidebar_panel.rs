use iced::Element;
use iced::widget::svg;
use otty_app_i18n::{self as i18n, Key};
use otty_app_slots::{SectionMessage, SectionState};
use otty_app_theme::icons;
use otty_app_theme::theme::ThemeProps;
use otty_ui_kit::components::panel_section::PanelSection;
use otty_ui_kit::components::split_button::SplitButton;
use otty_ui_kit::size;
use otty_ui_tree::visible_row_count;

use super::sidebar_tree;
use crate::event::QuickLaunchIntent;
use crate::model::SidebarTreeViewModel;
use crate::types::InlineEditKind;

/// What a press in the Launches section sends: its title and chevron belong
/// to the frame, its tree and its plus to quick launch.
type Message = SectionMessage<QuickLaunchIntent>;

/// Props for the Launches section of the left panel.
///
/// The frame keeps whether the section is open and whether its Launches
/// menu is open, so the title and the chevron send
/// [`SectionMessage::Toggle`] and [`SectionMessage::OpenMenu`]; the tree and
/// the New launch button send quick launch intents.
pub(crate) struct SidebarPanelProps<'a> {
    pub(crate) vm: SidebarTreeViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
    /// Whether the section body is shown and its Launches menu is open,
    /// which frames the chevron.
    pub(crate) section: SectionState,
}

/// Render the Launches section: header with the New launch split button, and
/// the launch tree as its body.
pub(crate) fn view(props: SidebarPanelProps<'_>) -> Element<'_, Message> {
    let palette = props.theme.theme.ui();
    let is_open = props.section.is_open();

    let new_launch = SplitButton::new(
        svg::Handle::from_memory(icons::ADD_TAB_HEADER),
        i18n::t(Key::CreateQuickLaunch),
        i18n::t(Key::LaunchesMenu),
        palette,
    )
    .menu_open(props.section.is_menu_open())
    .on_press(SectionMessage::Feature(
        QuickLaunchIntent::HeaderCreateCommand,
    ))
    .on_menu(SectionMessage::OpenMenu);

    let launch_count = props.vm.data.launch_count();

    let tree = sidebar_tree::view(sidebar_tree::SidebarTreeProps {
        vm: props.vm,
        theme: props.theme,
    })
    .map(SectionMessage::Feature);

    let mut section =
        PanelSection::new(i18n::t(Key::LaunchesPanelTitle), palette)
            .open(is_open)
            .grow(true)
            .action(new_launch)
            .body(tree)
            .on_toggle(SectionMessage::Toggle);
    if !is_open {
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
pub(crate) fn content_height(vm: &SidebarTreeViewModel<'_>) -> f32 {
    let visible_rows = visible_row_count(vm.data.root().children());
    let is_creating_folder = matches!(
        vm.inline_edit.map(|edit| edit.kind),
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
    use otty_app_domain::exit_behavior::ExitBehavior;
    use otty_app_domain::quick_launch::{
        CommandSpec, CustomCommand, QuickLaunch,
    };
    use otty_app_slots::{SectionMessage, SectionState};
    use otty_app_theme::theme::{ThemeManager, ThemeProps};
    use otty_ui_kit::size;

    use super::{Message, SidebarPanelProps, content_height, view};
    use crate::event::QuickLaunchIntent;
    use crate::model::{InlineEditViewModel, SidebarTreeViewModel};
    use crate::state::InlineEditState;
    use crate::types::{
        InlineEditKind, LaunchInfo, QuickLaunchFile, QuickLaunchNode,
    };

    /// Width of the panel in the tests.
    const PANEL_WIDTH: f32 = 260.0;
    /// Section padding 8, header row 30, gap 1: the top of the rows.
    const BODY_TOP: f32 = 8.0 + 30.0 + 1.0;
    /// The plus of the split button, from the right edge of the panel.
    const PLUS_X: f32 =
        PANEL_WIDTH - 8.0 - size::SPLIT_CHEVRON - size::CONTROL_SM / 2.0;
    /// The chevron of the split button, from the right edge of the panel.
    const CHEVRON_X: f32 = PANEL_WIDTH - 8.0 - size::SPLIT_CHEVRON / 2.0;
    /// Section padding 8, header row 30 high: the middle of the header.
    const HEADER_Y: f32 = 8.0 + 15.0;

    fn file_with_one_launch() -> QuickLaunchFile {
        let mut file = QuickLaunchFile::default();
        file.root.children = vec![QuickLaunchNode::Command(QuickLaunch::new(
            String::from("build"),
            CommandSpec::Custom {
                custom: CustomCommand::new(
                    String::from("make"),
                    Vec::new(),
                    Vec::new(),
                    None,
                ),
            },
            ExitBehavior::Close,
        ))];
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
    ) -> SidebarTreeViewModel<'a> {
        SidebarTreeViewModel {
            data: file,
            selected_path: None,
            hovered_path: None,
            inline_edit: inline_edit.map(InlineEditViewModel::from_state),
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
        let vm = SidebarTreeViewModel {
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
            section: SectionState::new(is_open, false),
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
    fn given_folder_and_launch_of_one_level_when_drawn_then_their_names_start_together()
     {
        let file = file_with_folder(true);
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut ui = simulator(panel(&file, &launching, &themes, true));

        let folder = ui.find("Work").expect("folder").bounds().x;
        let launch = ui.find("htop").expect("root launch").bounds().x;
        let nested = ui.find("build").expect("nested launch").bounds().x;

        assert_eq!(launch, folder, "a launch should start where a folder does");
        assert_eq!(nested, folder + size::TREE_INDENT);
    }

    #[test]
    fn given_header_when_title_pressed_then_section_toggles() {
        let file = file_with_one_launch();
        let launching = HashMap::new();
        let themes = ThemeManager::new();
        let mut ui = simulator(panel(&file, &launching, &themes, true));

        ui.click("Launches").expect("title should be visible");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(matches!(messages.as_slice(), [SectionMessage::Toggle]));
    }

    #[test]
    fn given_header_when_plus_pressed_then_header_create_command_is_sent() {
        let messages = click_header_at(PLUS_X, true);

        assert!(matches!(
            messages.as_slice(),
            [SectionMessage::Feature(
                QuickLaunchIntent::HeaderCreateCommand
            )]
        ));
    }

    #[test]
    fn given_header_when_chevron_pressed_then_the_menu_opens() {
        let messages = click_header_at(CHEVRON_X, true);

        assert!(matches!(messages.as_slice(), [SectionMessage::OpenMenu]));
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
