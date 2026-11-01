//! Sharing the height of a side panel between its sections.
//!
//! A side panel stacks sections such as Sessions and Launches. A closed
//! section takes only its header. Open sections take the height of their
//! content while everything fits. When it does not fit, the open sections
//! share the free height: each gets an equal share, but a section whose
//! content is smaller than its share keeps only its content height and
//! leaves the rest to the others. Each section scrolls inside its share.

use super::panel_section;

/// Header and content height of one section, as input to
/// [`share_panel_height`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionHeight {
    header: f32,
    content: f32,
    is_open: bool,
}

impl SectionHeight {
    /// Creates the size of a section: the `header` height that always
    /// shows, the `content` height shown only while the section is open,
    /// and whether it is open. Negative heights count as zero.
    pub fn new(header: f32, content: f32, is_open: bool) -> Self {
        Self {
            header: header.max(0.0),
            content: content.max(0.0),
            is_open,
        }
    }

    /// Creates the size of a [`PanelSection`] whose body is `content` high.
    ///
    /// [`PanelSection`]: panel_section::PanelSection
    pub fn panel_section(content: f32, is_open: bool) -> Self {
        let header = if is_open {
            panel_section::HEADER_HEIGHT + panel_section::BODY_GAP
        } else {
            panel_section::HEADER_HEIGHT
        };

        Self::new(header, content, is_open)
    }
}

/// Returns the height of each section in `sections`, top to bottom, when
/// they share `available` height.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::panel_height::{
///     SectionHeight, share_panel_height,
/// };
///
/// let sessions = SectionHeight::new(40.0, 600.0, true);
/// let launches = SectionHeight::new(40.0, 100.0, true);
///
/// // 320 free pixels: Launches keeps its 100, Sessions gets the other 220.
/// let heights = share_panel_height(400.0, &[sessions, launches]);
/// assert_eq!(heights, vec![260.0, 140.0]);
/// ```
pub fn share_panel_height(
    available: f32,
    sections: &[SectionHeight],
) -> Vec<f32> {
    let headers: f32 = sections.iter().map(|section| section.header).sum();
    let mut free = (available - headers).max(0.0);
    let mut bodies: Vec<Option<f32>> = sections
        .iter()
        .map(|section| if section.is_open { None } else { Some(0.0) })
        .collect();

    loop {
        let waiting: Vec<usize> = (0..sections.len())
            .filter(|&index| bodies[index].is_none())
            .collect();
        if waiting.is_empty() {
            break;
        }

        let share = free / waiting.len() as f32;
        let small: Vec<usize> = waiting
            .iter()
            .copied()
            .filter(|&index| sections[index].content <= share)
            .collect();

        if small.is_empty() {
            for index in waiting {
                bodies[index] = Some(share);
            }
            break;
        }
        for index in small {
            let content = sections[index].content;
            bodies[index] = Some(content);
            free -= content;
        }
    }

    sections
        .iter()
        .zip(bodies)
        .map(|(section, body)| section.header + body.unwrap_or(0.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Header height used by the table rows.
    const H: f32 = 40.0;

    /// One table row: name, available height, `(content, is_open)` of each
    /// section, expected heights.
    type Case = (&'static str, f32, Vec<(f32, bool)>, Vec<f32>);

    #[test]
    fn given_sections_when_height_shared_then_each_gets_its_expected_height() {
        let cases: Vec<Case> = vec![
            (
                "everything fits: open sections take their content",
                1000.0,
                vec![(100.0, true), (200.0, true)],
                vec![140.0, 240.0],
            ),
            (
                "closed sections take only their header",
                1000.0,
                vec![(100.0, false), (200.0, true)],
                vec![40.0, 240.0],
            ),
            (
                "all closed: headers only",
                1000.0,
                vec![(100.0, false), (200.0, false)],
                vec![40.0, 40.0],
            ),
            (
                "both too big: equal shares",
                480.0,
                vec![(900.0, true), (900.0, true)],
                vec![240.0, 240.0],
            ),
            (
                "small section keeps its content, the big one gets the rest",
                480.0,
                vec![(100.0, true), (900.0, true)],
                vec![140.0, 340.0],
            ),
            (
                "big section first, small one second",
                480.0,
                vec![(900.0, true), (100.0, true)],
                vec![340.0, 140.0],
            ),
            (
                "content exactly the share: keeps it",
                480.0,
                vec![(200.0, true), (900.0, true)],
                vec![240.0, 240.0],
            ),
            (
                "too big next to a closed section: takes all free height",
                480.0,
                vec![(900.0, true), (100.0, false)],
                vec![440.0, 40.0],
            ),
            (
                "three sections: small ones fill first, rest to the big one",
                420.0,
                vec![(50.0, true), (100.0, true), (1000.0, true)],
                vec![90.0, 140.0, 190.0],
            ),
            (
                "three sections: two big ones split what the small one left",
                420.0,
                vec![(50.0, true), (1000.0, true), (1000.0, true)],
                vec![90.0, 165.0, 165.0],
            ),
            (
                "less height than the headers: headers only",
                50.0,
                vec![(100.0, true), (100.0, true)],
                vec![40.0, 40.0],
            ),
            (
                "open section with no content: header only",
                480.0,
                vec![(0.0, true), (900.0, true)],
                vec![40.0, 440.0],
            ),
            ("no sections", 480.0, vec![], vec![]),
        ];

        for (case, available, sections, expected) in cases {
            let sections: Vec<SectionHeight> = sections
                .into_iter()
                .map(|(content, is_open)| {
                    SectionHeight::new(H, content, is_open)
                })
                .collect();

            let heights = share_panel_height(available, &sections);

            assert_eq!(heights, expected, "{case}");
        }
    }

    #[test]
    fn given_negative_heights_when_created_then_they_count_as_zero() {
        let section = SectionHeight::new(-5.0, -10.0, true);

        assert_eq!(section, SectionHeight::new(0.0, 0.0, true));
    }

    #[test]
    fn given_panel_section_when_sized_then_open_header_includes_body_gap() {
        let open = SectionHeight::panel_section(100.0, true);
        let closed = SectionHeight::panel_section(100.0, false);

        assert_eq!(
            open,
            SectionHeight::new(
                panel_section::HEADER_HEIGHT + panel_section::BODY_GAP,
                100.0,
                true
            )
        );
        assert_eq!(
            closed,
            SectionHeight::new(panel_section::HEADER_HEIGHT, 100.0, false)
        );
    }
}
