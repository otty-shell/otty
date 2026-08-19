use iced::widget::pane_grid::{Axis, Node, Pane, Split};

/// Compute the split ratios that spread the panes of `target`'s split
/// group evenly across the space the group occupies.
///
/// A split group is the largest run of nested splits that share one
/// axis and contain `target`. Splits outside that run, including the
/// ones on the other axis that bound it, are absent from the result,
/// so ratios adjusted by hand elsewhere in the layout stay untouched.
///
/// Returns an empty vector when `target` is the only pane, or when it
/// is not part of `layout` at all.
pub(super) fn equalized_ratios(
    layout: &Node,
    target: Pane,
) -> Vec<(Split, f32)> {
    // SHORTCUT: ratios equalize logical slots without spacing correction;
    // pass the rendered grid size and spacing if sub-pixel equality becomes
    // a product requirement for large same-axis groups.
    let mut path = Vec::new();
    if !find_path(layout, target, &mut path) {
        return Vec::new();
    }

    let Some((root, group_axis)) = group_root(&path) else {
        return Vec::new();
    };

    let mut ratios = Vec::new();
    collect_ratios(root, group_axis, &mut ratios);

    ratios
}

/// Record the chain of split nodes that leads down to `target`,
/// outermost first. Returns whether `target` was found at all.
fn find_path<'a>(
    node: &'a Node,
    target: Pane,
    path: &mut Vec<&'a Node>,
) -> bool {
    match node {
        Node::Split { a, b, .. } => {
            path.push(node);
            if find_path(a, target, path) || find_path(b, target, path) {
                return true;
            }
            path.pop();

            false
        },
        _ => matches!(node, Node::Pane(pane) if *pane == target),
    }
}

/// Find the largest same-axis split group containing the target.
fn group_root<'a>(path: &[&'a Node]) -> Option<(&'a Node, Axis)> {
    let mut root = *path.last()?;

    let Node::Split { axis, .. } = root else {
        return None;
    };

    let group_axis = *axis;
    for &node in path.iter().rev().skip(1) {
        match node {
            Node::Split { axis, .. } if *axis == group_axis => {
                root = node;
            },
            _ => break,
        }
    }

    Some((root, group_axis))
}

/// Give every split on `group_axis` a ratio proportional to how many
/// slots sit on each of its sides, so the group ends up even.
///
/// Recursion stops at the first split on the other axis: that subtree
/// belongs to a different group and keeps whatever ratio it has.
fn collect_ratios(
    node: &Node,
    group_axis: Axis,
    ratios: &mut Vec<(Split, f32)>,
) -> usize {
    let Node::Split { id, axis, a, b, .. } = node else {
        return 1;
    };
    if *axis != group_axis {
        return 1;
    }

    let a_slots = collect_ratios(a, group_axis, ratios);
    let b_slots = collect_ratios(b, group_axis, ratios);
    let total_slots = a_slots + b_slots;
    ratios.push((*id, a_slots as f32 / total_slots as f32));

    total_slots
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use iced::widget::pane_grid;

    use super::*;

    const PANE_SPACING: f32 = 1.0;

    /// Apply every computed ratio and read the resulting pane widths.
    fn equalize_and_measure(
        state: &mut pane_grid::State<u64>,
        target: Pane,
        bounds: Size,
    ) -> Vec<f32> {
        for (split, ratio) in equalized_ratios(state.layout(), target) {
            state.resize(split, ratio);
        }

        state
            .layout()
            .pane_regions(PANE_SPACING, 0.0, bounds)
            .values()
            .map(|region| region.width)
            .collect()
    }

    fn assert_width_spread(widths: &[f32], maximum: f32) {
        let minimum = widths.iter().copied().fold(f32::INFINITY, f32::min);
        let maximum_width =
            widths.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        assert!(
            maximum_width - minimum <= maximum,
            "expected a width spread no greater than {maximum}, got {widths:?}"
        );
    }

    #[test]
    fn given_two_panes_when_equalized_then_widths_match() {
        let (mut state, first) = pane_grid::State::new(1_u64);
        let (second, _) = state
            .split(Axis::Vertical, first, 2)
            .expect("first split succeeds");

        let widths =
            equalize_and_measure(&mut state, second, Size::new(900.0, 600.0));

        assert_eq!(widths.len(), 2);
        assert_width_spread(&widths, 1.0);
    }

    #[test]
    fn given_left_deep_three_pane_group_when_equalized_then_widths_match() {
        let (mut state, first) = pane_grid::State::new(1_u64);
        let _ = state
            .split(Axis::Vertical, first, 2)
            .expect("first split succeeds");
        let (third, _) = state
            .split(Axis::Vertical, first, 3)
            .expect("second split succeeds");

        let widths =
            equalize_and_measure(&mut state, third, Size::new(900.0, 600.0));

        assert_eq!(widths.len(), 3);
        assert_width_spread(&widths, 1.0);
    }

    #[test]
    fn given_right_deep_three_pane_group_when_equalized_then_widths_match() {
        let (mut state, first) = pane_grid::State::new(1_u64);
        let (second, _) = state
            .split(Axis::Vertical, first, 2)
            .expect("first split succeeds");
        let (third, _) = state
            .split(Axis::Vertical, second, 3)
            .expect("second split succeeds");

        let widths =
            equalize_and_measure(&mut state, third, Size::new(900.0, 600.0));

        assert_eq!(widths.len(), 3);
        assert_width_spread(&widths, 1.0);
    }

    #[test]
    fn given_four_panes_in_one_group_when_equalized_then_each_takes_a_quarter()
    {
        let (mut state, first) = pane_grid::State::new(1_u64);
        let (second, _) = state
            .split(Axis::Vertical, first, 2)
            .expect("first split succeeds");
        let (third, _) = state
            .split(Axis::Vertical, second, 3)
            .expect("second split succeeds");
        let (fourth, _) = state
            .split(Axis::Vertical, third, 4)
            .expect("third split succeeds");

        let widths =
            equalize_and_measure(&mut state, fourth, Size::new(1200.0, 600.0));

        assert_eq!(widths.len(), 4);
        assert_width_spread(&widths, 1.0);
    }

    #[test]
    fn given_mixed_axis_layout_when_equalized_then_other_group_is_untouched() {
        let (mut state, top) = pane_grid::State::new(1_u64);
        let (bottom, horizontal) = state
            .split(Axis::Horizontal, top, 2)
            .expect("horizontal split succeeds");
        let (right, _) = state
            .split(Axis::Vertical, bottom, 3)
            .expect("vertical split succeeds");

        let ratios = equalized_ratios(state.layout(), right);

        assert!(
            !ratios.iter().any(|(split, _)| *split == horizontal),
            "the horizontal split bounds the group and must not be resized"
        );
        assert_eq!(
            ratios.len(),
            1,
            "only the vertical split inside the group is equalized"
        );
        assert!(
            (ratios[0].1 - 0.5).abs() <= f32::EPSILON,
            "two panes in the group split it evenly, got {}",
            ratios[0].1
        );

        for (split, ratio) in ratios {
            state.resize(split, ratio);
        }

        let regions =
            state
                .layout()
                .split_regions(0.0, 0.0, Size::new(900.0, 600.0));
        let (_, _, horizontal_ratio) =
            regions.get(&horizontal).expect("horizontal split present");
        assert!(
            (horizontal_ratio - 0.5).abs() <= f32::EPSILON,
            "expected the horizontal split to stay at 0.5, got \
             {horizontal_ratio}"
        );
    }

    #[test]
    fn given_nested_same_axis_group_when_equalized_then_all_group_panes_match()
    {
        let (mut state, top) = pane_grid::State::new(1_u64);
        let (bottom, horizontal) = state
            .split(Axis::Horizontal, top, 2)
            .expect("horizontal split succeeds");
        let (right, _) = state
            .split(Axis::Vertical, bottom, 3)
            .expect("first vertical split succeeds");
        let (fourth, _) = state
            .split(Axis::Vertical, right, 4)
            .expect("second vertical split succeeds");

        let ratios = equalized_ratios(state.layout(), fourth);

        assert_eq!(ratios.len(), 2);
        assert!(!ratios.iter().any(|(split, _)| *split == horizontal));

        for (split, ratio) in ratios {
            state.resize(split, ratio);
        }

        let regions = state.layout().pane_regions(
            PANE_SPACING,
            0.0,
            Size::new(900.0, 600.0),
        );

        assert_eq!(regions.len(), 4);
        assert_width_spread(
            &[
                regions[&bottom].width,
                regions[&right].width,
                regions[&fourth].width,
            ],
            1.0,
        );
    }

    #[test]
    fn given_single_pane_when_equalized_then_no_ratios_are_produced() {
        let (state, only) = pane_grid::State::new(1_u64);

        assert!(equalized_ratios(state.layout(), only).is_empty());
    }

    #[test]
    fn given_foreign_pane_when_equalized_then_no_ratios_are_produced() {
        let (state, _) = pane_grid::State::new(1_u64);
        let (_, foreign) = pane_grid::State::new(2_u64);

        assert!(equalized_ratios(state.layout(), foreign).is_empty());
    }
}
