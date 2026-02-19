use iced::widget::{Space, column, container, row, text};
use iced::{Color, Element, Length};
use otty_ui_tree::{TreeNode, TreePath, TreeRowContext, TreeView};

#[derive(Debug, Clone)]
enum Message {
    Select(TreePath),
    Hover(Option<TreePath>),
}

#[derive(Clone)]
enum Node {
    Folder {
        title: String,
        expanded: bool,
        children: Vec<Node>,
    },
    File {
        title: String,
    },
}

impl Node {
    fn title(&self) -> &str {
        match self {
            Node::Folder { title, .. } => title,
            Node::File { title } => title,
        }
    }

    fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Folder { children, .. } => Some(children),
            Node::File { .. } => None,
        }
    }

    fn toggle(&mut self) -> bool {
        match self {
            Node::Folder { expanded, .. } => {
                *expanded = !*expanded;
                true
            },
            Node::File { .. } => false,
        }
    }
}

impl TreeNode for Node {
    fn title(&self) -> &str {
        self.title()
    }

    fn children(&self) -> Option<&[Self]> {
        match self {
            Node::Folder { children, .. } => Some(children),
            Node::File { .. } => None,
        }
    }

    fn expanded(&self) -> bool {
        match self {
            Node::Folder { expanded, .. } => *expanded,
            Node::File { .. } => false,
        }
    }

    fn is_folder(&self) -> bool {
        matches!(self, Node::Folder { .. })
    }
}

struct AppState {
    tree: Vec<Node>,
    selected: Option<TreePath>,
    hovered: Option<TreePath>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tree: vec![
                Node::Folder {
                    title: String::from("General"),
                    expanded: true,
                    children: vec![
                        Node::File {
                            title: String::from("Terminal"),
                        },
                        Node::File {
                            title: String::from("Theme"),
                        },
                    ],
                },
                Node::File {
                    title: String::from("About"),
                },
            ],
            selected: None,
            hovered: None,
        }
    }
}

fn update(state: &mut AppState, message: Message) {
    match message {
        Message::Select(path) => {
            if is_folder_at_path(&state.tree, &path) {
                let _ = toggle_folder(&mut state.tree, &path);
            } else {
                state.selected = Some(path);
            }
        },
        Message::Hover(path) => {
            state.hovered = path;
        },
    }
}

fn view(state: &AppState) -> Element<'_, Message> {
    TreeView::new(&state.tree, render_row)
        .selected(state.selected.as_ref())
        .hovered(state.hovered.as_ref())
        .on_press(Message::Select)
        .on_hover(Message::Hover)
        .row_style(row_style)
        .toggle_content(toggle_icon)
        .toggle_width(16.0)
        .indent_width(14.0)
        .spacing(0.0)
        .view()
}

fn render_row<'a>(context: &TreeRowContext<'a, Node>) -> Element<'a, Message> {
    let label = if context.entry.node.is_folder() {
        format!("Folder: {}", context.entry.node.title())
    } else {
        format!("File: {}", context.entry.node.title())
    };

    let row = row![text(label)].spacing(6);
    container(column![row])
        .padding([4, 8])
        .width(Length::Fill)
        .into()
}

fn toggle_folder(nodes: &mut [Node], path: &[String]) -> bool {
    if path.is_empty() {
        return false;
    }

    for node in nodes {
        if node.title() == path[0] {
            if path.len() == 1 {
                return node.toggle();
            }
            if let Some(children) = node.children_mut() {
                return toggle_folder(children, &path[1..]);
            }
            return false;
        }
    }

    false
}

fn is_folder_at_path(nodes: &[Node], path: &[String]) -> bool {
    if path.is_empty() {
        return false;
    }

    for node in nodes {
        if node.title() == path[0] {
            if path.len() == 1 {
                return node.is_folder();
            }
            if let Some(children) = node.children() {
                return is_folder_at_path(children, &path[1..]);
            }
            return false;
        }
    }

    false
}

fn row_style(context: &TreeRowContext<'_, Node>) -> container::Style {
    let background = if context.is_selected {
        Some(Color::from_rgb(0.12, 0.26, 0.46).into())
    } else if context.is_hovered {
        Some(Color::from_rgb(0.18, 0.18, 0.18).into())
    } else {
        None
    };

    container::Style {
        background,
        text_color: Some(Color::from_rgb(0.9, 0.9, 0.9)),
        ..Default::default()
    }
}

fn toggle_icon<'a>(context: &TreeRowContext<'a, Node>) -> Element<'a, Message> {
    if context.entry.node.is_folder() {
        let label = if context.entry.node.expanded() {
            "[-]"
        } else {
            "[+]"
        };
        text(label).into()
    } else {
        Space::new().width(Length::Fixed(16.0)).into()
    }
}

fn main() -> iced::Result {
    iced::run(update, view)
}
