//! `PaletteList`: the items of the command palette, the typed query, the
//! items that match it and the lit one.
//!
//! The list knows nothing about the app. Each item carries a caller key, so
//! the app can tell which of its sessions, launches or actions was chosen.

use std::ops::Range;

use super::target_dot::TargetKind;

/// One entry of the command palette.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::palette_list::PaletteItem;
/// use otty_ui_kit::components::target_dot::TargetKind;
///
/// let item = PaletteItem::new(7_u32, "Launches", "laptop")
///     .with_detail("ubuntu@192.168.1.20")
///     .with_target(TargetKind::Ssh)
///     .with_shortcut("Alt 1");
///
/// assert_eq!(item.title(), "laptop");
/// assert_eq!(*item.key(), 7);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteItem<K> {
    key: K,
    group: String,
    title: String,
    detail: Option<String>,
    target: Option<TargetKind>,
    shortcut: Option<String>,
}

impl<K> PaletteItem<K> {
    /// Caller key that names the app item behind this entry.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Name of the group the entry is listed under.
    pub fn group(&self) -> &str {
        &self.group
    }

    /// Main text of the entry.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Muted text after the title, such as a host or a path.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Target whose dot is drawn before the title.
    pub fn target(&self) -> Option<TargetKind> {
        self.target
    }

    /// Shortcut hint shown on the right.
    pub fn shortcut(&self) -> Option<&str> {
        self.shortcut.as_deref()
    }

    /// Creates an entry with no detail, dot or shortcut.
    pub fn new(
        key: K,
        group: impl Into<String>,
        title: impl Into<String>,
    ) -> Self {
        Self {
            key,
            group: group.into(),
            title: title.into(),
            detail: None,
            target: None,
            shortcut: None,
        }
    }

    /// Sets the muted detail. The query is matched against it too.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Draws a target dot before the title.
    pub fn with_target(mut self, target: TargetKind) -> Self {
        self.target = Some(target);
        self
    }

    /// Shows a shortcut hint on the right.
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

/// An item that matches the query and the byte ranges of its title that
/// match.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PaletteMatch {
    item: usize,
    title_ranges: Vec<Range<usize>>,
}

/// Every palette entry, the typed query, the entries that match it and the
/// lit (keyboard-chosen) one.
///
/// Matching ignores case and looks for the query inside the title or the
/// detail. Matches are listed group by group, in the order each group first
/// appears in the items; groups with no match are left out. The lit entry is
/// an index into the matches. None is lit at first and after the query
/// changes; the keys of the palette, the same as those of every list of
/// suggestions of the kit, choose the next one, and [`Self::highlight`]
/// keeps it.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::palette_list::{PaletteItem, PaletteList};
///
/// let mut list = PaletteList::new(vec![
///     PaletteItem::new(1, "Sessions", "zsh"),
///     PaletteItem::new(2, "Actions", "New launch"),
/// ]);
///
/// list.set_query("la".to_owned());
/// list.highlight(Some(0));
///
/// assert_eq!(list.match_count(), 1);
/// assert_eq!(list.key_at(0), Some(&2));
/// assert_eq!(list.highlighted(), Some(0));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteList<K> {
    items: Vec<PaletteItem<K>>,
    query: String,
    matches: Vec<PaletteMatch>,
    highlighted: Option<usize>,
}

impl<K> PaletteList<K> {
    /// Text typed into the palette input.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Index of the lit entry among the matches.
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// Number of entries that match the query.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Matching entries in display order, each with the byte ranges of its
    /// title that match the query.
    pub fn matches(
        &self,
    ) -> impl Iterator<Item = (&PaletteItem<K>, &[Range<usize>])> {
        self.matches.iter().map(|found| {
            (&self.items[found.item], found.title_ranges.as_slice())
        })
    }

    /// Key of the match at `index`, as sent by a click on that entry.
    pub fn key_at(&self, index: usize) -> Option<&K> {
        let found = self.matches.get(index)?;

        Some(&self.items[found.item].key)
    }

    /// Creates a list with an empty query: every entry matches and none is
    /// lit.
    pub fn new(items: Vec<PaletteItem<K>>) -> Self {
        let mut list = Self {
            items,
            query: String::new(),
            matches: Vec::new(),
            highlighted: None,
        };

        list.match_query();

        list
    }

    /// Replaces the query and matches again; no entry is lit. A query of
    /// only spaces matches everything.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.match_query();
    }

    /// Lights the match at `row`, the row the keys chose, or none for
    /// `None`. A row past the matches lights none.
    pub fn highlight(&mut self, row: Option<usize>) {
        self.highlighted = row.filter(|row| *row < self.matches.len());
    }

    /// Rebuilds the matches for the current query, group by group, with
    /// none lit.
    fn match_query(&mut self) {
        let needle = FoldedText::new(self.query.trim()).text;

        let mut matches = Vec::new();
        for group in group_order(&self.items) {
            for (index, item) in self.items.iter().enumerate() {
                if item.group != group {
                    continue;
                }

                if let Some(title_ranges) = match_item(item, &needle) {
                    matches.push(PaletteMatch {
                        item: index,
                        title_ranges,
                    });
                }
            }
        }

        self.highlighted = None;
        self.matches = matches;
    }
}

/// Lower-case copy of a text that remembers which original char each of its
/// bytes comes from, so matches found in the copy map back to the original
/// even when lower-casing changes the byte length.
pub(super) struct FoldedText {
    text: String,
    origins: Vec<Range<usize>>,
}

impl FoldedText {
    /// The lower-case text.
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    /// Lower-cases `original` char by char.
    pub(super) fn new(original: &str) -> Self {
        let mut text = String::with_capacity(original.len());
        let mut origins = Vec::with_capacity(original.len());

        for (start, character) in original.char_indices() {
            let origin = start..start + character.len_utf8();
            for lower in character.to_lowercase() {
                text.push(lower);
                origins.extend((0..lower.len_utf8()).map(|_| origin.clone()));
            }
        }

        Self { text, origins }
    }

    /// Returns the original letters that own the given bytes of the folded
    /// text, as byte ranges of the original text; letters that are next to
    /// each other make one range.
    pub(super) fn originals_of(&self, bytes: &[usize]) -> Vec<Range<usize>> {
        let mut sorted: Vec<usize> = bytes
            .iter()
            .copied()
            .filter(|&byte| byte < self.origins.len())
            .collect();
        sorted.sort_unstable();

        let mut ranges: Vec<Range<usize>> = Vec::new();
        for byte in sorted {
            let origin = self.origins[byte].clone();

            match ranges.last_mut() {
                Some(last) if origin.start <= last.end => {
                    last.end = last.end.max(origin.end);
                },
                _ => ranges.push(origin),
            }
        }

        ranges
    }

    /// Returns the non-overlapping places of `needle`, an already folded
    /// text, as byte ranges of the original text.
    pub(super) fn find_all(&self, needle: &str) -> Vec<Range<usize>> {
        let mut ranges: Vec<Range<usize>> = Vec::new();

        for (start, found) in self.text.match_indices(needle) {
            let end = start + found.len();
            let range = self.origins[start].start..self.origins[end - 1].end;

            match ranges.last_mut() {
                Some(last) if range.start < last.end => last.end = range.end,
                _ => ranges.push(range),
            }
        }

        ranges
    }
}

/// Returns each group name once, in the order it first appears.
fn group_order<K>(items: &[PaletteItem<K>]) -> Vec<&str> {
    let mut groups: Vec<&str> = Vec::new();
    for item in items {
        if !groups.contains(&item.group.as_str()) {
            groups.push(&item.group);
        }
    }

    groups
}

/// Matches `item` against `needle`, an already folded query. Returns the
/// title ranges to highlight, empty when only the detail matches, or `None`
/// when the item does not match.
fn match_item<K>(
    item: &PaletteItem<K>,
    needle: &str,
) -> Option<Vec<Range<usize>>> {
    if needle.is_empty() {
        return Some(Vec::new());
    }

    let title_ranges = FoldedText::new(&item.title).find_all(needle);
    if !title_ranges.is_empty() {
        return Some(title_ranges);
    }

    let detail_matches = item.detail.as_deref().is_some_and(|detail| {
        !FoldedText::new(detail).find_all(needle).is_empty()
    });

    detail_matches.then(Vec::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte ranges written as `(start, end)` pairs.
    type Spans = &'static [(usize, usize)];

    fn titles(list: &PaletteList<u32>) -> Vec<&str> {
        list.matches().map(|(item, _)| item.title()).collect()
    }

    fn groups(list: &PaletteList<u32>) -> Vec<&str> {
        let mut groups: Vec<&str> = Vec::new();
        for (item, _) in list.matches() {
            if groups.last() != Some(&item.group()) {
                groups.push(item.group());
            }
        }

        groups
    }

    fn three_items() -> PaletteList<u32> {
        PaletteList::new(vec![
            PaletteItem::new(1, "Sessions", "zsh"),
            PaletteItem::new(2, "Sessions", "htop"),
            PaletteItem::new(3, "Actions", "New launch"),
        ])
    }

    #[test]
    fn given_empty_query_when_listed_then_every_item_is_shown_by_group() {
        let list = PaletteList::new(vec![
            PaletteItem::new(1, "Sessions", "zsh"),
            PaletteItem::new(2, "Launches", "laptop"),
            PaletteItem::new(3, "Sessions", "htop"),
            PaletteItem::new(4, "Actions", "Settings"),
        ]);

        assert_eq!(titles(&list), vec!["zsh", "htop", "laptop", "Settings"]);
        assert_eq!(groups(&list), vec!["Sessions", "Launches", "Actions"]);
        assert_eq!(list.highlighted(), None);
        assert!(list.matches().all(|(_, ranges)| ranges.is_empty()));
    }

    #[test]
    fn given_whitespace_query_when_listed_then_every_item_is_shown() {
        let mut list = three_items();

        list.set_query("   ".to_owned());

        assert_eq!(list.match_count(), 3);
        assert_eq!(list.query(), "   ");
    }

    #[test]
    fn given_query_when_matched_then_title_ranges_are_byte_ranges() {
        // (title, detail, query, expected title ranges; None = no match)
        // (title, detail, query, expected title ranges as (start, end)
        // bytes; None = no match)
        let cases: [(&str, Option<&str>, &str, Option<Spans>); 14] = [
            ("laptop", None, "la", Some(&[(0, 2)])),
            ("htop on laptop", None, "LA", Some(&[(8, 10)])),
            ("New launch…", None, "la", Some(&[(4, 6)])),
            ("Settings: Language", None, "la", Some(&[(10, 12)])),
            ("banana", None, "an", Some(&[(1, 3), (3, 5)])),
            ("aaaa", None, "aa", Some(&[(0, 2), (2, 4)])),
            ("laptop", None, "  la ", Some(&[(0, 2)])),
            ("laptop", Some("ubuntu@192.168.1.20"), "UBUNTU", Some(&[])),
            ("laptop", Some("ubuntu@host"), "xyz", None),
            ("laptop", None, "laptops", None),
            ("Сервер Продакшн", None, "прод", Some(&[(13, 21)])),
            ("Сервер Продакшн", Some("Москва"), "москва", Some(&[])),
            ("ターミナル設定", None, "設定", Some(&[(15, 21)])),
            ("İstanbul", None, "i", Some(&[(0, 2)])),
        ];

        for (title, detail, query, expected) in cases {
            let mut item = PaletteItem::new(1, "Group", title);
            if let Some(detail) = detail {
                item = item.with_detail(detail);
            }
            let mut list = PaletteList::new(vec![item]);

            list.set_query(query.to_owned());

            let found: Option<Vec<(usize, usize)>> =
                list.matches().next().map(|(_, ranges)| {
                    ranges
                        .iter()
                        .map(|range| (range.start, range.end))
                        .collect()
                });
            assert_eq!(
                found.as_deref(),
                expected,
                "title {title:?}, query {query:?}"
            );
            for (start, end) in found.unwrap_or_default() {
                assert!(
                    title.get(start..end).is_some(),
                    "{start}..{end} is not on a char boundary of {title:?}"
                );
            }
        }
    }

    #[test]
    fn given_filtered_items_when_listed_then_group_order_follows_all_items() {
        let mut list = PaletteList::new(vec![
            PaletteItem::new(1, "Sessions", "zsh"),
            PaletteItem::new(2, "Launches", "build"),
            PaletteItem::new(3, "Sessions", "cargo build"),
            PaletteItem::new(4, "Actions", "Settings"),
        ]);

        list.set_query("build".to_owned());

        assert_eq!(titles(&list), vec!["cargo build", "build"]);
        assert_eq!(groups(&list), vec!["Sessions", "Launches"]);
    }

    /// Issue 43: the list keeps the row the shared keys of the kit chose;
    /// it moves nothing itself.
    #[test]
    fn a_highlight_lights_that_match_only() {
        let mut list = three_items();

        list.highlight(Some(2));
        let lit = list.highlighted();
        list.highlight(None);

        assert_eq!(lit, Some(2));
        assert_eq!(list.highlighted(), None);
    }

    #[test]
    fn a_highlight_past_the_matches_lights_nothing() {
        let mut list = three_items();
        list.highlight(Some(1));

        list.highlight(Some(3));

        assert_eq!(list.highlighted(), None);
    }

    #[test]
    fn a_new_query_clears_the_highlight() {
        let mut list = three_items();
        list.highlight(Some(2));

        list.set_query("h".to_owned());

        assert_eq!(titles(&list), vec!["zsh", "htop", "New launch"]);
        assert_eq!(list.highlighted(), None);
    }

    #[test]
    fn given_no_items_when_used_then_nothing_matches_or_is_lit() {
        let mut list: PaletteList<u32> = PaletteList::new(Vec::new());

        list.highlight(Some(0));
        list.set_query("a".to_owned());

        assert_eq!(list.match_count(), 0);
        assert_eq!(list.highlighted(), None);
        assert_eq!(list.key_at(0), None);
    }

    #[test]
    fn given_matches_when_key_read_then_it_follows_display_order() {
        let list = three_items();

        assert_eq!(list.key_at(1), Some(&2));
        assert_eq!(list.key_at(2), Some(&3));
        assert_eq!(list.key_at(3), None);
    }
}
