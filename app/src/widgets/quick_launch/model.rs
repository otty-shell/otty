use std::collections::HashMap;

use super::state::InlineEditState;
use super::types::{
    DropTarget, LaunchInfo, NodePath, QuickLaunchFile, QuickLaunchFolder,
    QuickLaunchNode,
};

/// Read-only view model for the quick launch tree.
#[derive(Debug, Clone)]
pub(crate) struct QuickLaunchTreeViewModel<'a> {
    pub(super) data: &'a QuickLaunchFile,
    pub(super) selected_path: Option<&'a NodePath>,
    pub(super) hovered_path: Option<&'a NodePath>,
    pub(super) inline_edit: Option<&'a InlineEditState>,
    pub(super) launching: &'a HashMap<NodePath, LaunchInfo>,
    pub(super) drop_target: Option<&'a DropTarget>,
}

impl<'a> QuickLaunchTreeViewModel<'a> {
    /// Every launch in the tree, folders left out, in tree order (a
    /// folder's launches come where the folder is).
    pub(crate) fn launches(&self) -> Vec<LaunchViewModel<'a>> {
        let mut launches = Vec::new();

        collect_launches(self.data.root(), &[], &mut launches);

        launches
    }
}

#[cfg(test)]
impl<'a> QuickLaunchTreeViewModel<'a> {
    /// Tree of `data` that no one is using: nothing selected, hovered,
    /// renamed or dragged over. `launching` holds the launches that are
    /// starting.
    pub(crate) fn idle(
        data: &'a QuickLaunchFile,
        launching: &'a HashMap<NodePath, LaunchInfo>,
    ) -> Self {
        Self {
            data,
            selected_path: None,
            hovered_path: None,
            inline_edit: None,
            launching,
            drop_target: None,
        }
    }
}

/// One launch of the tree, for lists outside the tree such as the command
/// palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchViewModel<'a> {
    /// Path in the tree: the folder titles, then the launch title.
    pub(crate) path: NodePath,
    pub(crate) title: &'a str,
    /// The launch connects over SSH.
    pub(crate) is_ssh: bool,
}

/// Add the launches of `folder` (at tree path `folder_path`) and of every
/// folder below it to `launches`, in tree order.
fn collect_launches<'a>(
    folder: &'a QuickLaunchFolder,
    folder_path: &[String],
    launches: &mut Vec<LaunchViewModel<'a>>,
) {
    for node in folder.children() {
        let mut path = folder_path.to_vec();
        path.push(node.title().to_string());

        match node {
            QuickLaunchNode::Folder(child) => {
                collect_launches(child, &path, launches);
            },
            QuickLaunchNode::Command(command) => {
                launches.push(LaunchViewModel {
                    path,
                    title: command.title(),
                    is_ssh: command.is_ssh(),
                });
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::exit_behavior::ExitBehavior;
    use crate::widgets::quick_launch::types::{
        CommandSpec, CustomCommand, QuickLaunch, SshCommand,
    };

    fn command(title: &str) -> QuickLaunchNode {
        QuickLaunchNode::Command(QuickLaunch {
            title: String::from(title),
            spec: CommandSpec::Custom {
                custom: CustomCommand {
                    program: String::from("htop"),
                    args: Vec::new(),
                    env: Vec::new(),
                    working_directory: None,
                },
            },
            exit_behavior: ExitBehavior::Close,
        })
    }

    fn ssh(title: &str) -> QuickLaunchNode {
        QuickLaunchNode::Command(QuickLaunch {
            title: String::from(title),
            spec: CommandSpec::Ssh {
                ssh: SshCommand {
                    host: String::from("example.com"),
                    port: 22,
                    user: None,
                    identity_file: None,
                    extra_args: Vec::new(),
                },
            },
            exit_behavior: ExitBehavior::Close,
        })
    }

    fn folder(title: &str, children: Vec<QuickLaunchNode>) -> QuickLaunchNode {
        QuickLaunchNode::Folder(QuickLaunchFolder {
            title: String::from(title),
            expanded: false,
            children,
        })
    }

    fn path(segments: &[&str]) -> NodePath {
        segments
            .iter()
            .map(|segment| String::from(*segment))
            .collect()
    }

    #[test]
    fn launches_walk_the_tree_in_order_and_skip_folders() {
        let mut data = QuickLaunchFile::default();
        data.root.children = vec![
            command("htop"),
            folder(
                "servers",
                vec![ssh("laptop"), folder("prod", vec![ssh("storage")])],
            ),
            folder("empty", Vec::new()),
            command("logs"),
        ];
        let launching = HashMap::new();
        let vm = QuickLaunchTreeViewModel {
            data: &data,
            selected_path: None,
            hovered_path: None,
            inline_edit: None,
            launching: &launching,
            drop_target: None,
        };

        let launches = vm.launches();

        assert_eq!(
            launches,
            vec![
                LaunchViewModel {
                    path: path(&["htop"]),
                    title: "htop",
                    is_ssh: false,
                },
                LaunchViewModel {
                    path: path(&["servers", "laptop"]),
                    title: "laptop",
                    is_ssh: true,
                },
                LaunchViewModel {
                    path: path(&["servers", "prod", "storage"]),
                    title: "storage",
                    is_ssh: true,
                },
                LaunchViewModel {
                    path: path(&["logs"]),
                    title: "logs",
                    is_ssh: false,
                },
            ]
        );
    }
}
