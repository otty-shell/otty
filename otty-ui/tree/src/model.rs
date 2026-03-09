/// Path of node titles from root to the target row.
///
/// The path is built by concatenating [`TreeNode::title`] values while walking
/// from the root to the current node.
///
/// Note: if siblings can have the same title, title-based paths become
/// ambiguous. In that case you should ensure unique titles per sibling set.
pub type TreePath = Vec<String>;

/// Trait implemented by tree node types consumable by this crate.
pub trait TreeNode {
    /// Title used to identify the node within its parent.
    fn title(&self) -> &str;
    /// Children for the node (folders only).
    fn children(&self) -> Option<&[Self]>
    where
        Self: Sized;
    /// Whether the node is expanded (folders only).
    fn expanded(&self) -> bool;
    /// Whether this node is a folder.
    fn is_folder(&self) -> bool;
}

/// Flattened representation of a visible tree node.
pub struct FlattenedNode<'a, T: TreeNode> {
    /// Zero-based tree depth (`0` for root-level rows).
    pub depth: usize,
    /// Borrowed source node.
    pub node: &'a T,
    /// Title-based path from the root to this row.
    pub path: TreePath,
}

/// Flatten a tree into a depth-first list of visible rows.
///
/// The output is stable and sorted per level using these rules:
/// - folders are placed before files;
/// - nodes of the same kind use natural title ordering (case-insensitive first,
///   then case-sensitive tiebreak).
///
/// Children are included only when `node.is_folder() && node.expanded()`.
pub fn flatten_tree<'a, T: TreeNode>(
    nodes: &'a [T],
) -> Vec<FlattenedNode<'a, T>> {
    let mut entries = Vec::new();
    let mut path = Vec::new();
    for index in sorted_indices(nodes) {
        push_node(&nodes[index], 0, &mut path, &mut entries);
    }
    entries
}

fn push_node<'a, T: TreeNode>(
    node: &'a T,
    depth: usize,
    path: &mut Vec<String>,
    entries: &mut Vec<FlattenedNode<'a, T>>,
) {
    path.push(node.title().to_string());
    entries.push(FlattenedNode {
        depth,
        node,
        path: path.clone(),
    });

    if node.is_folder() && node.expanded() {
        if let Some(children) = node.children() {
            for index in sorted_indices(children) {
                push_node(&children[index], depth + 1, path, entries);
            }
        }
    }

    path.pop();
}

fn sorted_indices<T: TreeNode>(nodes: &[T]) -> Vec<usize> {
    let mut ordered: Vec<usize> = (0..nodes.len()).collect();
    ordered.sort_by(|a, b| {
        let left = &nodes[*a];
        let right = &nodes[*b];
        match (!left.is_folder()).cmp(&(!right.is_folder())) {
            std::cmp::Ordering::Equal => {
                compare_titles(left.title(), right.title())
            },
            other => other,
        }
    });
    ordered
}

fn compare_titles(left: &str, right: &str) -> std::cmp::Ordering {
    let left_segments = split_segments(left);
    let right_segments = split_segments(right);
    let mut left_iter = left_segments.iter();
    let mut right_iter = right_segments.iter();

    loop {
        match (left_iter.next(), right_iter.next()) {
            (Some(Segment::Digits(l)), Some(Segment::Digits(r))) => {
                let ord = compare_digit_segments(l, r);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            },
            (Some(Segment::Text(l)), Some(Segment::Text(r))) => {
                let ord = compare_text_segments(l, r);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            },
            (Some(Segment::Digits(_)), Some(Segment::Text(_))) => {
                return std::cmp::Ordering::Less;
            },
            (Some(Segment::Text(_)), Some(Segment::Digits(_))) => {
                return std::cmp::Ordering::Greater;
            },
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => break,
        }
    }

    left.cmp(right)
}

#[derive(Debug)]
enum Segment {
    Text(String),
    Digits(String),
}

fn split_segments(input: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_is_digit: Option<bool> = None;

    for ch in input.chars() {
        let is_digit = ch.is_ascii_digit();
        match current_is_digit {
            Some(kind) if kind == is_digit => current.push(ch),
            Some(kind) => {
                segments.push(if kind {
                    Segment::Digits(current)
                } else {
                    Segment::Text(current)
                });
                current = String::new();
                current.push(ch);
                current_is_digit = Some(is_digit);
            },
            None => {
                current.push(ch);
                current_is_digit = Some(is_digit);
            },
        }
    }

    if let Some(kind) = current_is_digit {
        segments.push(if kind {
            Segment::Digits(current)
        } else {
            Segment::Text(current)
        });
    }

    segments
}

fn compare_text_segments(left: &str, right: &str) -> std::cmp::Ordering {
    let left_lower = left.to_lowercase();
    let right_lower = right.to_lowercase();
    match left_lower.cmp(&right_lower) {
        std::cmp::Ordering::Equal => left.cmp(right),
        other => other,
    }
}

fn compare_digit_segments(left: &str, right: &str) -> std::cmp::Ordering {
    let left_trim = left.trim_start_matches('0');
    let right_trim = right.trim_start_matches('0');

    let left_trim = if left_trim.is_empty() { "0" } else { left_trim };
    let right_trim = if right_trim.is_empty() {
        "0"
    } else {
        right_trim
    };

    match left_trim.len().cmp(&right_trim.len()) {
        std::cmp::Ordering::Equal => match left_trim.cmp(right_trim) {
            std::cmp::Ordering::Equal => left.len().cmp(&right.len()),
            other => other,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    enum TestNode {
        Folder {
            title: String,
            expanded: bool,
            children: Vec<TestNode>,
        },
        VirtualFolder {
            title: String,
            expanded: bool,
        },
        File {
            title: String,
        },
    }

    impl TestNode {
        fn folder(title: &str, expanded: bool, children: Vec<Self>) -> Self {
            Self::Folder {
                title: title.to_owned(),
                expanded,
                children,
            }
        }

        fn virtual_folder(title: &str, expanded: bool) -> Self {
            Self::VirtualFolder {
                title: title.to_owned(),
                expanded,
            }
        }

        fn file(title: &str) -> Self {
            Self::File {
                title: title.to_owned(),
            }
        }
    }

    impl TreeNode for TestNode {
        fn title(&self) -> &str {
            match self {
                TestNode::Folder { title, .. } => title,
                TestNode::VirtualFolder { title, .. } => title,
                TestNode::File { title } => title,
            }
        }

        fn children(&self) -> Option<&[Self]> {
            match self {
                TestNode::Folder { children, .. } => Some(children),
                TestNode::VirtualFolder { .. } => None,
                TestNode::File { .. } => None,
            }
        }

        fn expanded(&self) -> bool {
            match self {
                TestNode::Folder { expanded, .. } => *expanded,
                TestNode::VirtualFolder { expanded, .. } => *expanded,
                TestNode::File { .. } => false,
            }
        }

        fn is_folder(&self) -> bool {
            matches!(
                self,
                TestNode::Folder { .. } | TestNode::VirtualFolder { .. }
            )
        }
    }

    fn path(parts: &[&str]) -> TreePath {
        parts.iter().map(|part| (*part).to_owned()).collect()
    }

    fn flat_titles(entries: &[FlattenedNode<'_, TestNode>]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| entry.node.title().to_owned())
            .collect()
    }

    #[test]
    fn flatten_tree_handles_empty_input() {
        let nodes: Vec<TestNode> = Vec::new();
        let entries = flatten_tree(&nodes);
        assert!(entries.is_empty());
    }

    #[test]
    fn flatten_tree_respects_expansion_and_depth() {
        let nodes = vec![
            TestNode::folder(
                "root",
                true,
                vec![
                    TestNode::file("child2"),
                    TestNode::folder(
                        "child-folder",
                        false,
                        vec![TestNode::file("hidden")],
                    ),
                    TestNode::file("child1"),
                ],
            ),
            TestNode::file("top"),
        ];

        let entries = flatten_tree(&nodes);
        assert_eq!(
            flat_titles(&entries),
            vec!["root", "child-folder", "child1", "child2", "top"]
        );

        assert_eq!(entries[0].depth, 0);
        assert_eq!(entries[1].depth, 1);
        assert_eq!(entries[2].depth, 1);
        assert_eq!(entries[3].depth, 1);
        assert_eq!(entries[4].depth, 0);
        assert_eq!(entries[0].path, path(&["root"]));
        assert_eq!(entries[2].path, path(&["root", "child1"]));
        assert_eq!(entries[4].path, path(&["top"]));
    }

    #[test]
    fn flatten_tree_hides_children_of_collapsed_nodes() {
        let nodes = vec![TestNode::folder(
            "root",
            false,
            vec![TestNode::file("hidden")],
        )];
        let entries = flatten_tree(&nodes);
        assert_eq!(flat_titles(&entries), vec!["root"]);
    }

    #[test]
    fn flatten_tree_handles_folder_without_children_slice() {
        let nodes = vec![TestNode::virtual_folder("virtual", true)];
        let entries = flatten_tree(&nodes);
        assert_eq!(flat_titles(&entries), vec!["virtual"]);
        assert_eq!(entries[0].path, path(&["virtual"]));
    }

    #[test]
    fn flatten_tree_path_invariant_holds() {
        let nodes = vec![
            TestNode::folder(
                "a",
                true,
                vec![TestNode::folder("b", true, vec![TestNode::file("c")])],
            ),
            TestNode::file("d"),
        ];

        let entries = flatten_tree(&nodes);
        for entry in entries {
            assert_eq!(entry.path.len(), entry.depth + 1);
        }
    }

    #[test]
    fn sorted_indices_put_folders_before_files_and_apply_natural_sort() {
        let nodes = vec![
            TestNode::file("file10"),
            TestNode::folder("zeta", false, vec![]),
            TestNode::folder("A2", false, vec![]),
            TestNode::file("file2"),
            TestNode::folder("a10", false, vec![]),
        ];

        let order = sorted_indices(&nodes);
        let titles: Vec<String> = order
            .iter()
            .map(|index| nodes[*index].title().to_owned())
            .collect();
        assert_eq!(titles, vec!["A2", "a10", "zeta", "file2", "file10"]);
    }

    #[test]
    fn compare_titles_covers_text_digit_and_prefix_branches() {
        assert_eq!(compare_titles("file2", "file10"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_titles("file02", "file2"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(compare_titles("1a", "a1"), std::cmp::Ordering::Less);
        assert_eq!(compare_titles("a1", "1a"), std::cmp::Ordering::Greater);
        assert_eq!(compare_titles("abc", "abc1"), std::cmp::Ordering::Less);
        assert_eq!(compare_titles("abc1", "abc"), std::cmp::Ordering::Greater);
        assert_eq!(compare_titles("same", "same"), std::cmp::Ordering::Equal);
        assert_eq!(
            compare_titles("a9b10", "a9b2"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn compare_titles_uses_case_insensitive_with_case_tiebreak() {
        assert_eq!(compare_titles("alpha", "Bravo"), std::cmp::Ordering::Less);
        assert_eq!(compare_titles("a", "A"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn split_segments_handles_empty_text_digits_and_mixed_inputs() {
        let empty = split_segments("");
        assert!(empty.is_empty());

        let text = split_segments("abc");
        assert_eq!(text.len(), 1);
        assert!(matches!(text[0], Segment::Text(ref value) if value == "abc"));

        let digits = split_segments("123");
        assert_eq!(digits.len(), 1);
        assert!(
            matches!(digits[0], Segment::Digits(ref value) if value == "123")
        );

        let mixed = split_segments("ab12cd34");
        assert_eq!(mixed.len(), 4);
        assert!(matches!(mixed[0], Segment::Text(ref value) if value == "ab"));
        assert!(
            matches!(mixed[1], Segment::Digits(ref value) if value == "12")
        );
        assert!(matches!(mixed[2], Segment::Text(ref value) if value == "cd"));
        assert!(
            matches!(mixed[3], Segment::Digits(ref value) if value == "34")
        );
    }

    #[test]
    fn split_segments_treats_non_ascii_digits_as_text() {
        let segments = split_segments("١٢");
        assert_eq!(segments.len(), 1);
        assert!(
            matches!(segments[0], Segment::Text(ref value) if value == "١٢")
        );
    }

    #[test]
    fn compare_digit_segments_handles_leading_zeros_and_zero_only_segments() {
        assert_eq!(compare_digit_segments("2", "02"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_digit_segments("000", "0"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_digit_segments("010", "10"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn compare_titles_is_antisymmetric_for_representative_inputs() {
        let titles =
            ["a1", "a01", "A1", "a2", "file10", "file2", "abc", "abc1"];

        for left in titles {
            for right in titles {
                let forward = compare_titles(left, right);
                let reverse = compare_titles(right, left);
                assert_eq!(forward, reverse.reverse());
            }
        }
    }

    #[test]
    fn compare_titles_is_transitive_for_sorted_sequence() {
        let mut values = [
            String::from("a2"),
            String::from("a10"),
            String::from("a02"),
            String::from("A1"),
            String::from("1a"),
            String::from("z"),
        ];
        values.sort_by(|left, right| compare_titles(left, right));

        for first in 0..values.len() {
            for second in first..values.len() {
                for third in second..values.len() {
                    let a = &values[first];
                    let b = &values[second];
                    let c = &values[third];
                    assert!(
                        compare_titles(a, b) != std::cmp::Ordering::Greater
                    );
                    assert!(
                        compare_titles(b, c) != std::cmp::Ordering::Greater
                    );
                    assert!(
                        compare_titles(a, c) != std::cmp::Ordering::Greater
                    );
                }
            }
        }
    }
}
