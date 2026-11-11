//! `TagSearch`: the model of a search field that filters a table by plain
//! text and by `key:value` tags.
//!
//! The model knows nothing about what the table lists or about languages: a
//! widget supplies a [`TagSchema`] (its [`TagKey`]s in column order and one
//! [`SearchRow`] per row), and forwards each [`TagSearchAction`] to
//! [`TagSearch::perform`]. The views over this model are `TagSearchField` and
//! `TagRow`.

use std::ops::Range;

use super::palette_list::FoldedText;

/// A query of this many letters or fewer allows no typo when ranking.
const SHORT_QUERY_LETTERS: usize = 3;
/// What ends a key: a colon, or the wide colon that Chinese and Japanese input
/// often produces.
const KEY_ENDS: [char; 2] = [':', '：'];

/// One key of a table: a stable internal name, which the code uses and the
/// user never types or sees, and the name the user reads and types.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tag_search::TagKey;
///
/// let key = TagKey::new("repository", "Repository:Tag");
///
/// assert_eq!(key.name(), "repository");
/// assert_eq!(key.shown(), "Repository");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagKey {
    name: &'static str,
    shown: String,
}

impl TagKey {
    /// The internal name, which rows and tags use.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The name the user reads and types.
    pub fn shown(&self) -> &str {
        &self.shown
    }

    /// Creates a key shown as `header`, up to its first colon of either kind:
    /// a colon ends a key, so no shown name can hold one.
    pub fn new(name: &'static str, header: &str) -> Self {
        let shown = header
            .split_once(KEY_ENDS)
            .map_or(header, |(before, _)| before);

        Self {
            name,
            shown: shown.trim().to_owned(),
        }
    }
}

/// One `key:value` filter of a [`TagSearch`]. It stores the internal name of
/// its key, so it shows the current name of the key whatever the language was
/// when it was added.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tag_search::{Tag, TagKey};
///
/// let tag = Tag::new("status", "running");
///
/// assert_eq!(
///     tag.label(&[TagKey::new("status", "Status")]),
///     "Status:running"
/// );
/// assert_eq!(
///     tag.label(&[TagKey::new("status", "Статус")]),
///     "Статус:running"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    key: &'static str,
    value: String,
}

impl Tag {
    /// The internal name of the key: a [`TagKey::name`] of the schema.
    pub fn key(&self) -> &'static str {
        self.key
    }

    /// The text a cell of this key must contain.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The tag as the user reads it: the shown name of its key in `keys`,
    /// `:` and the value. A key that `keys` does not have reads its internal
    /// name.
    pub fn label(&self, keys: &[TagKey]) -> String {
        let Self { key, value } = self;
        let shown = keys
            .iter()
            .find(|one| one.name == *key)
            .map_or(*key, TagKey::shown);

        format!("{shown}:{value}")
    }

    /// Creates a tag of the key with the internal name `key`.
    pub fn new(key: &'static str, value: impl Into<String>) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }
}

/// One row of the table as the search sees it: its name and the text of its
/// cells by key. A cell with several values, such as ports, adds each one.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tag_search::SearchRow;
///
/// let row = SearchRow::new("web-1")
///     .value("status", "Running")
///     .value("ports", "8080:80/tcp")
///     .value("ports", "443/tcp");
///
/// assert_eq!(row.name(), "web-1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRow {
    name: String,
    values: Vec<(&'static str, String)>,
}

impl SearchRow {
    /// The text plain typing is matched against.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Creates a row with no values.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            values: Vec::new(),
        }
    }

    /// Adds one value of `key`. A blank cell has no value.
    pub fn value(
        mut self,
        key: &'static str,
        value: impl Into<String>,
    ) -> Self {
        let value = value.into();
        if !value.trim().is_empty() {
            self.values.push((key, value));
        }

        self
    }

    /// The values of `key`.
    pub fn values_of<'a>(
        &'a self,
        key: &'a str,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.values
            .iter()
            .filter(move |(value_key, _)| *value_key == key)
            .map(|(_, value)| value.as_str())
    }

    /// Whether a value of `key` contains `text`, ignoring case.
    fn has_value(&self, key: &str, text: &str) -> bool {
        self.values_of(key)
            .any(|value| contains_folded(value, text))
    }
}

/// What a widget tells the search about its table: the keys in column order
/// and the rows.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tag_search::{SearchRow, TagKey, TagSchema};
///
/// let schema = TagSchema::new(
///     vec![TagKey::new("name", "Name"), TagKey::new("status", "Status")],
///     vec![SearchRow::new("web-1").value("name", "web-1")],
/// );
///
/// assert_eq!(schema.keys()[1].shown(), "Status");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSchema {
    keys: Vec<TagKey>,
    rows: Vec<SearchRow>,
}

impl TagSchema {
    /// The keys in column order.
    pub fn keys(&self) -> &[TagKey] {
        &self.keys
    }

    /// The rows of the table.
    pub fn rows(&self) -> &[SearchRow] {
        &self.rows
    }

    /// Creates a schema.
    pub fn new(keys: Vec<TagKey>, rows: Vec<SearchRow>) -> Self {
        Self { keys, rows }
    }
}

/// What a [`Suggestion`] puts in the field when it is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    /// A key: the field becomes `key:` and shows its values.
    Key,
    /// A row name: the field holds the name and filters by it.
    Name,
    /// A value of the typed key: it becomes a tag and the field clears.
    Value,
    /// The text as typed, always the first entry when there is any: plain
    /// text stays as a name filter and the list closes; `key:text` becomes a
    /// tag and the field clears.
    Typed,
}

/// One entry of the suggestion list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    kind: SuggestionKind,
    text: String,
    ranges: Vec<Range<usize>>,
    is_highlighted: bool,
}

impl Suggestion {
    /// What the entry is.
    pub fn kind(&self) -> SuggestionKind {
        self.kind
    }

    /// The text of the entry.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The byte ranges of [`Self::text`] that match the typed text.
    pub fn ranges(&self) -> &[Range<usize>] {
        &self.ranges
    }

    /// Whether Enter or Tab applies this entry.
    pub fn is_highlighted(&self) -> bool {
        self.is_highlighted
    }
}

/// What the user did in the views of a [`TagSearch`]. A widget forwards it to
/// [`TagSearch::perform`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagSearchAction {
    /// The text in the field changed.
    InputChanged(String),
    /// ↑ was pressed.
    MoveUp,
    /// ↓ was pressed.
    MoveDown,
    /// Enter was pressed: applies the highlighted entry.
    Apply,
    /// Tab was pressed: completes to the first real suggestion below the
    /// typed text, and does what Enter does if there is none.
    Complete,
    /// The suggestion at this position was pressed.
    Pick(usize),
    /// Esc or a press outside closed the suggestions.
    Close,
    /// The field got the focus.
    Focused,
    /// Backspace was pressed in the empty field.
    RemoveLastTag,
    /// The cross of the tag at this position was pressed.
    RemoveTag(usize),
    /// "Clear all" was pressed.
    ClearTags,
}

/// What a [`TagSearch`] change means for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagSearchChange {
    /// Nothing changed.
    Nothing,
    /// Only the field or the suggestion list changed: the same rows pass.
    Suggestions,
    /// The tags or the name text changed: refilter the rows.
    Filter,
}

/// The state of one search: the typed text, the tags, the highlighted
/// suggestion and whether the suggestions are open.
///
/// Text before the first `:` (or wide `：`) is a key only if it is the shown
/// name of a key of the schema, so `redis:7` stays plain text and
/// `Ports:8080:80/tcp` has the value `8080:80/tcp`. A row passes when it has a
/// matching value for every key that has tags (several tags of one key match
/// if any does) and its name contains the plain text; all of it is a substring
/// match that ignores case.
///
/// # Example
///
/// ```
/// use otty_ui_kit::components::tag_search::{
///     SearchRow, TagKey, TagSchema, TagSearch, TagSearchAction,
/// };
///
/// let keys =
///     vec![TagKey::new("name", "Name"), TagKey::new("status", "Status")];
/// let web = SearchRow::new("web-1").value("status", "Running");
/// let db = SearchRow::new("db-1").value("status", "Exited");
/// let schema = TagSchema::new(keys.clone(), vec![web.clone(), db.clone()]);
/// let mut search = TagSearch::default();
///
/// for action in [
///     TagSearchAction::InputChanged("sta".to_owned()),
///     TagSearchAction::Complete,
///     TagSearchAction::InputChanged("Status:run".to_owned()),
///     TagSearchAction::Complete,
/// ] {
///     search.perform(action, &schema);
/// }
///
/// assert_eq!(search.tags()[0].key(), "status");
/// assert_eq!(search.tags()[0].label(&keys), "Status:Running");
/// assert_eq!(search.input(), "");
/// assert!(search.passes(&keys, &web));
/// assert!(!search.passes(&keys, &db));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagSearch {
    input: String,
    tags: Vec<Tag>,
    highlighted: usize,
    is_open: bool,
}

impl TagSearch {
    /// The text in the field.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The tags in the order they were added.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// Whether the suggestions are open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// The text rows are filtered by name with: the trimmed field, or nothing
    /// while the field holds `key:value`.
    pub fn name_text(&self, keys: &[TagKey]) -> &str {
        match parse(&self.input, keys) {
            ParsedInput::Plain(text) => text,
            ParsedInput::Tag { .. } => "",
        }
    }

    /// Whether any row can fail [`Self::passes`]. Without tags and name text
    /// every row passes, so the rows need not be built to ask.
    ///
    /// ```
    /// use otty_ui_kit::components::tag_search::TagSearch;
    ///
    /// assert!(!TagSearch::default().filters_rows(&[]));
    /// ```
    pub fn filters_rows(&self, keys: &[TagKey]) -> bool {
        !self.tags.is_empty() || !self.name_text(keys).is_empty()
    }

    /// Whether `row` passes the tags and the name text.
    pub fn passes(&self, keys: &[TagKey], row: &SearchRow) -> bool {
        self.passes_tags(row)
            && contains_folded(row.name(), self.name_text(keys))
    }

    /// Whether `row` passes the tags, whatever the name text is.
    pub fn passes_tags(&self, row: &SearchRow) -> bool {
        self.passes_tags_except(row, None)
    }

    /// The suggestions for the typed text; none while they are closed.
    ///
    /// The first entry is the typed text itself ([`SuggestionKind::Typed`]),
    /// except for an empty field, a bare `key:`, or text that equals another
    /// entry, ignoring case.
    ///
    /// With no key yet: the keys whose shown name matches, in column order,
    /// then the matching names of the rows the tags leave (an empty field
    /// shows only the keys). After `key:`: the values of that key, from the
    /// rows the tags of other keys leave, without the values that already are
    /// tags.
    pub fn suggestions(&self, schema: &TagSchema) -> Vec<Suggestion> {
        if !self.is_open {
            return Vec::new();
        }

        let mut found = match parse(&self.input, &schema.keys) {
            ParsedInput::Plain(text) => {
                self.key_and_name_suggestions(text, schema)
            },
            ParsedInput::Tag { key, text } => {
                self.value_suggestions(key.name(), text, schema)
            },
        };
        if let Some(typed) = self.typed_suggestion(&found, &schema.keys) {
            found.insert(0, typed);
        }

        if let Some(last) = found.len().checked_sub(1) {
            found[self.highlighted.min(last)].is_highlighted = true;
        }

        found
    }

    /// Does what the user did and says what changed.
    pub fn perform(
        &mut self,
        action: TagSearchAction,
        schema: &TagSchema,
    ) -> TagSearchChange {
        let before = self.clone();

        match action {
            TagSearchAction::InputChanged(text) => self.set_input(text),
            TagSearchAction::MoveUp => self.move_highlight(schema, Step::Up),
            TagSearchAction::MoveDown => {
                self.move_highlight(schema, Step::Down);
            },
            TagSearchAction::Apply => self.apply(schema),
            TagSearchAction::Complete => self.complete(schema),
            TagSearchAction::Pick(index) => self.pick(index, schema),
            TagSearchAction::Close => self.is_open = false,
            TagSearchAction::Focused => self.is_open = true,
            TagSearchAction::RemoveLastTag => self.remove_last_tag(),
            TagSearchAction::RemoveTag(index) => self.remove_tag(index),
            TagSearchAction::ClearTags => self.tags.clear(),
        }

        self.change_since(&before, &schema.keys)
    }

    /// Adds the tag `key:value` by the internal name of the key, replacing the
    /// other tags of `key`. Use it for a link that filters from outside the
    /// field.
    pub fn set_tag(
        &mut self,
        key: &'static str,
        value: &str,
    ) -> TagSearchChange {
        let tags_of_key = self.tags.iter().filter(|tag| tag.key == key).count();
        if tags_of_key == 1 && self.has_tag(key, value) {
            return TagSearchChange::Nothing;
        }

        self.tags.retain(|tag| tag.key != key);
        self.tags.push(Tag::new(key, value));

        TagSearchChange::Filter
    }

    /// Empties the field and closes the suggestions.
    pub fn clear_input(&mut self) {
        self.input.clear();
        self.highlighted = 0;
        self.is_open = false;
    }

    /// Closes the suggestions and keeps the text and the tags. Use it when the
    /// field goes off screen with its text kept: the list opens again from its
    /// first row once the field gets the focus.
    pub fn close_suggestions(&mut self) {
        self.highlighted = 0;
        self.is_open = false;
    }

    /// Removes every tag.
    pub fn clear_tags(&mut self) -> TagSearchChange {
        if self.tags.is_empty() {
            return TagSearchChange::Nothing;
        }

        self.tags.clear();

        TagSearchChange::Filter
    }

    /// The entry for the text as typed. There is none for an empty field or a
    /// bare `key:`, nor when `others` already has an entry with the same text
    /// (the case ignored): that entry stands for it.
    fn typed_suggestion(
        &self,
        others: &[Suggestion],
        keys: &[TagKey],
    ) -> Option<Suggestion> {
        let (shown, compared) = match parse(&self.input, keys) {
            ParsedInput::Plain(text) => (text.to_owned(), text),
            ParsedInput::Tag { key, text } => {
                let key = key.shown();

                (format!("{key}:{text}"), text)
            },
        };
        if compared.is_empty() {
            return None;
        }

        let is_listed = others
            .iter()
            .any(|other| other.text.to_lowercase() == compared.to_lowercase());
        if is_listed {
            return None;
        }

        Some(Suggestion {
            kind: SuggestionKind::Typed,
            text: shown,
            ranges: Vec::new(),
            is_highlighted: false,
        })
    }

    /// The matching keys in column order, then, once something is typed, the
    /// matching names of the rows the tags leave.
    fn key_and_name_suggestions(
        &self,
        text: &str,
        schema: &TagSchema,
    ) -> Vec<Suggestion> {
        let shown_keys: Vec<&str> =
            schema.keys.iter().map(TagKey::shown).collect();
        let mut keys = rank(text, &shown_keys);
        keys.sort_by_key(|key| key.index);

        let mut found = suggestions_of(SuggestionKind::Key, keys, &shown_keys);
        if text.is_empty() {
            return found;
        }

        let names = unique(
            schema
                .rows
                .iter()
                .filter(|row| self.passes_tags(row))
                .map(SearchRow::name),
        );
        found.extend(suggestions_of(
            SuggestionKind::Name,
            rank(text, &names),
            &names,
        ));

        found
    }

    /// The matching values of `key` from the rows the tags of other keys
    /// leave, without the values that already are tags.
    fn value_suggestions(
        &self,
        key: &'static str,
        text: &str,
        schema: &TagSchema,
    ) -> Vec<Suggestion> {
        let mut values = unique(
            schema
                .rows
                .iter()
                .filter(|row| self.passes_tags_except(row, Some(key)))
                .flat_map(|row| row.values_of(key)),
        );
        values.retain(|value| !self.has_tag(key, value));

        suggestions_of(SuggestionKind::Value, rank(text, &values), &values)
    }

    /// Whether `row` passes the tags, leaving out those of `skipped_key`.
    /// Several tags of one key match if any does.
    fn passes_tags_except(
        &self,
        row: &SearchRow,
        skipped_key: Option<&str>,
    ) -> bool {
        self.tags
            .iter()
            .filter(|tag| Some(tag.key) != skipped_key)
            .all(|tag| {
                self.tags
                    .iter()
                    .filter(|other| other.key == tag.key)
                    .any(|other| row.has_value(other.key, &other.value))
            })
    }

    /// Whether the tag `key:value` exists, ignoring the case of the value.
    fn has_tag(&self, key: &str, value: &str) -> bool {
        let value = value.to_lowercase();

        self.tags
            .iter()
            .any(|tag| tag.key == key && tag.value.to_lowercase() == value)
    }

    /// Takes the new text: the list opens with its first entry highlighted.
    fn set_input(&mut self, text: String) {
        self.input = text;
        self.highlighted = 0;
        self.is_open = true;
    }

    /// Moves the highlight one entry, wrapping at both ends. A closed list
    /// only opens.
    fn move_highlight(&mut self, schema: &TagSchema, step: Step) {
        if !self.is_open {
            self.is_open = true;
            return;
        }

        let found = self.suggestions(schema);
        let Some(current) = found.iter().position(Suggestion::is_highlighted)
        else {
            return;
        };

        self.highlighted = match step {
            Step::Up if current == 0 => found.len() - 1,
            Step::Up => current - 1,
            Step::Down => (current + 1) % found.len(),
        };
    }

    /// Enter or Tab: applies the highlighted suggestion; with none, adds the
    /// typed `key:text` as a tag; otherwise does nothing.
    fn apply(&mut self, schema: &TagSchema) {
        let found = self.suggestions(schema);

        match found.iter().find(|entry| entry.is_highlighted) {
            Some(entry) => self.apply_suggestion(entry, &schema.keys),
            None => self.add_typed_tag(&schema.keys),
        }
    }

    /// Tab: applies the highlighted suggestion if it is a real one, else the
    /// first real one below the typed text. With none, does what Enter does.
    fn complete(&mut self, schema: &TagSchema) {
        let found = self.suggestions(schema);

        let highlighted = found.iter().find(|entry| {
            entry.is_highlighted && entry.kind != SuggestionKind::Typed
        });
        let first_real = found
            .iter()
            .find(|entry| entry.kind != SuggestionKind::Typed);

        match highlighted.or(first_real) {
            Some(entry) => self.apply_suggestion(entry, &schema.keys),
            None => self.apply(schema),
        }
    }

    /// Applies the suggestion at `index`, if it is still listed.
    fn pick(&mut self, index: usize, schema: &TagSchema) {
        if let Some(entry) = self.suggestions(schema).get(index) {
            self.apply_suggestion(entry, &schema.keys);
        }
    }

    /// A key goes in the field and shows its values; a name fills the field
    /// and closes the list; a value becomes a tag and clears the field.
    fn apply_suggestion(&mut self, entry: &Suggestion, keys: &[TagKey]) {
        self.highlighted = 0;

        match entry.kind {
            SuggestionKind::Key => {
                let key = &entry.text;

                self.input = format!("{key}:");
            },
            SuggestionKind::Name => {
                self.input.clone_from(&entry.text);
                self.is_open = false;
            },
            SuggestionKind::Typed => {
                if let ParsedInput::Tag { key, text } = parse(&self.input, keys)
                {
                    let value = text.to_owned();
                    self.add_tag(key.name(), &value);
                    self.input.clear();
                } else {
                    self.is_open = false;
                }
            },
            SuggestionKind::Value => {
                if let ParsedInput::Tag { key, .. } = parse(&self.input, keys) {
                    self.add_tag(key.name(), &entry.text);
                }
                self.input.clear();
            },
        }
    }

    /// Adds the field's `key:text` as a tag and clears the field. Anything
    /// else in the field stays there.
    fn add_typed_tag(&mut self, keys: &[TagKey]) {
        let ParsedInput::Tag { key, text } = parse(&self.input, keys) else {
            return;
        };
        if text.is_empty() {
            return;
        }

        let value = text.to_owned();
        self.add_tag(key.name(), &value);

        self.input.clear();
        self.highlighted = 0;
    }

    /// Adds a tag unless it exists.
    fn add_tag(&mut self, key: &'static str, value: &str) {
        if !self.has_tag(key, value) {
            self.tags.push(Tag::new(key, value));
        }
    }

    /// Backspace: removes the last tag, only from an empty field.
    fn remove_last_tag(&mut self) {
        if self.input.is_empty() {
            self.tags.pop();
        }
    }

    /// Removes the tag at `index`, if there is one.
    fn remove_tag(&mut self, index: usize) {
        if index < self.tags.len() {
            self.tags.remove(index);
        }
    }

    /// What changed since `before`.
    fn change_since(&self, before: &Self, keys: &[TagKey]) -> TagSearchChange {
        if self.tags != before.tags
            || self.name_text(keys) != before.name_text(keys)
        {
            TagSearchChange::Filter
        } else if self != before {
            TagSearchChange::Suggestions
        } else {
            TagSearchChange::Nothing
        }
    }
}

/// What the text in the field means.
enum ParsedInput<'text, 'keys> {
    /// A filter by name.
    Plain(&'text str),
    /// A key of the schema, and the text typed after its colon.
    Tag {
        key: &'keys TagKey,
        text: &'text str,
    },
}

/// A move of the highlight.
#[derive(Clone, Copy)]
enum Step {
    Up,
    Down,
}

/// One candidate kept by [`rank`]: its position in the given list and the
/// byte ranges of its text that match.
struct Ranked {
    index: usize,
    ranges: Vec<Range<usize>>,
}

/// Reads `input`: text before the first colon of either kind is a key only if
/// it is the shown name of one of `keys`, in any case. Upper case is compared
/// because it also joins `Größe` and `GRÖSSE`, which lower case keeps apart.
fn parse<'text, 'keys>(
    input: &'text str,
    keys: &'keys [TagKey],
) -> ParsedInput<'text, 'keys> {
    if let Some((prefix, text)) = input.split_once(KEY_ENDS) {
        let prefix = prefix.trim().to_uppercase();

        if let Some(key) =
            keys.iter().find(|key| key.shown.to_uppercase() == prefix)
        {
            return ParsedInput::Tag {
                key,
                text: text.trim(),
            };
        }
    }

    ParsedInput::Plain(input.trim())
}

/// Returns the candidates that fuzzy-match `query`, best first. All of the
/// suggestion ranking lives here. The case is ignored and the letters of the
/// query may be apart in the candidate. A query of up to three letters allows
/// no typo, a longer one allows one. An empty query keeps every candidate in
/// the given order.
fn rank(query: &str, candidates: &[&str]) -> Vec<Ranked> {
    let needle = query.trim();

    if needle.is_empty() {
        return (0..candidates.len())
            .map(|index| Ranked {
                index,
                ranges: Vec::new(),
            })
            .collect();
    }

    let max_typos = if needle.chars().count() <= SHORT_QUERY_LETTERS {
        0
    } else {
        1
    };
    let config = frizbee::Config {
        max_typos: Some(max_typos),
        casing: frizbee::CaseMatching::Ignore,
        ..frizbee::Config::default()
    };
    // `frizbee` ignores case for ASCII only, so both sides are lower-cased
    // first. It reports the matched bytes of the lower-cased text, which
    // `FoldedText` maps back to the shown text.
    let needle = FoldedText::new(needle);
    let folded: Vec<FoldedText> =
        candidates.iter().map(|one| FoldedText::new(one)).collect();
    let lowered: Vec<&str> = folded.iter().map(FoldedText::text).collect();

    let mut matcher = frizbee::Matcher::new(needle.text(), &config);

    matcher
        .match_list_indices(&lowered)
        .into_iter()
        .map(|found| {
            let index = found.index as usize;
            let bytes: Vec<usize> =
                found.indices.iter().map(|&one| one as usize).collect();

            Ranked {
                index,
                ranges: folded[index].originals_of(&bytes),
            }
        })
        .collect()
}

/// Builds the suggestions of the `ranked` entries of `candidates`.
fn suggestions_of(
    kind: SuggestionKind,
    ranked: Vec<Ranked>,
    candidates: &[&str],
) -> Vec<Suggestion> {
    ranked
        .into_iter()
        .map(|one| Suggestion {
            kind,
            text: candidates[one.index].to_owned(),
            ranges: one.ranges,
            is_highlighted: false,
        })
        .collect()
}

/// Returns each text once, in the order it first appears.
fn unique<'a>(texts: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen: Vec<&str> = Vec::new();
    for text in texts {
        if !seen.contains(&text) {
            seen.push(text);
        }
    }

    seen
}

/// Whether `text` contains `needle`, ignoring case.
fn contains_folded(text: &str, needle: &str) -> bool {
    text.to_lowercase().contains(&needle.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys whose shown name is their internal name, for the rules that do
    /// not depend on the language.
    fn keys() -> Vec<TagKey> {
        ["name", "status", "image", "ports", "compose"]
            .into_iter()
            .map(|name| TagKey::new(name, name))
            .collect()
    }

    fn row(name: &str, status: &str, ports: &[&str]) -> SearchRow {
        let mut row = SearchRow::new(name)
            .value("name", name)
            .value("status", status);
        for port in ports {
            row = row.value("ports", *port);
        }

        row
    }

    /// Three rows: two running, one exited; `web-1` has two ports.
    fn schema() -> TagSchema {
        TagSchema::new(
            keys(),
            vec![
                row("web-1", "Running", &["8080:80/tcp", "8443:443/tcp"]),
                row("db-1", "Exited (3)", &["5432/tcp"]),
                row("status-page", "Running", &[]),
            ],
        )
    }

    fn typed(text: &str) -> TagSearch {
        let mut search = TagSearch::default();
        let _ = search
            .perform(TagSearchAction::InputChanged(text.to_owned()), &schema());

        search
    }

    fn texts(search: &TagSearch) -> Vec<String> {
        search
            .suggestions(&schema())
            .iter()
            .map(|suggestion| suggestion.text().to_owned())
            .collect()
    }

    fn apply(search: &mut TagSearch) -> TagSearchChange {
        search.perform(TagSearchAction::Apply, &schema())
    }

    fn labels(search: &TagSearch) -> Vec<String> {
        search.tags().iter().map(|tag| tag.label(&keys())).collect()
    }

    // Parsing.

    #[test]
    fn text_before_the_first_colon_is_a_key_only_if_the_schema_has_it() {
        // (typed text, expected name text)
        let cases = [
            ("redis", "redis"),
            ("  redis ", "redis"),
            ("redis:7", "redis:7"),
            ("unknown:value", "unknown:value"),
            (":value", ":value"),
            ("status:", ""),
            ("status:run", ""),
            ("STATUS:run", ""),
            (" status : run", ""),
            ("ports:8080:80/tcp", ""),
        ];

        for (text, expected) in cases {
            assert_eq!(typed(text).name_text(&keys()), expected, "{text:?}");
        }
    }

    // Suggestions.

    fn kinds_and_texts(search: &TagSearch) -> Vec<(SuggestionKind, String)> {
        search
            .suggestions(&schema())
            .iter()
            .map(|found| (found.kind(), found.text().to_owned()))
            .collect()
    }

    fn complete(search: &mut TagSearch) -> TagSearchChange {
        search.perform(TagSearchAction::Complete, &schema())
    }

    // The typed text.

    #[test]
    fn plain_text_is_the_first_suggestion_then_keys_then_names() {
        let found = kinds_and_texts(&typed("sta"));

        assert_eq!(
            found,
            vec![
                (SuggestionKind::Typed, "sta".to_owned()),
                (SuggestionKind::Key, "status".to_owned()),
                (SuggestionKind::Name, "status-page".to_owned()),
            ]
        );
        assert!(typed("sta").suggestions(&schema())[0].is_highlighted());
    }

    #[test]
    fn a_key_and_text_is_the_first_suggestion_then_the_values() {
        let found = kinds_and_texts(&typed("status:ru"));

        assert_eq!(
            found,
            vec![
                (SuggestionKind::Typed, "status:ru".to_owned()),
                (SuggestionKind::Value, "Running".to_owned()),
            ]
        );
    }

    #[test]
    fn the_typed_row_is_trimmed_and_takes_the_schema_spelling_of_the_key() {
        assert_eq!(texts(&typed("  zzz ")), vec!["zzz"]);
        assert_eq!(texts(&typed(" STATUS : zzz ")), vec!["status:zzz"]);
    }

    #[test]
    fn an_unknown_key_or_a_colon_in_a_name_is_a_typed_name_filter() {
        for text in ["foo:bar", "redis:7"] {
            assert_eq!(
                kinds_and_texts(&typed(text)),
                vec![(SuggestionKind::Typed, text.to_owned())],
                "{text:?}"
            );
        }
    }

    #[test]
    fn no_typed_row_for_an_empty_field_or_a_bare_key() {
        for text in ["", "   ", "status:", "STATUS: "] {
            let mut search = typed(text);
            let found = search.suggestions(&schema());

            assert!(
                found.iter().all(|one| one.kind() != SuggestionKind::Typed),
                "{text:?}"
            );
            assert!(found[0].is_highlighted(), "{text:?}");
            let _ = search.perform(TagSearchAction::Close, &schema());
        }
    }

    #[test]
    fn no_second_row_when_the_text_equals_a_suggestion_ignoring_case() {
        // (typed text, the row that stands for it)
        let cases = [
            ("STATUS", (SuggestionKind::Key, "status")),
            ("Web-1", (SuggestionKind::Name, "web-1")),
            ("status:RUNNING", (SuggestionKind::Value, "Running")),
        ];

        for (text, (kind, shown)) in cases {
            let found = kinds_and_texts(&typed(text));

            assert_eq!(found[0], (kind, shown.to_owned()), "{text:?}");
            assert!(
                found.iter().all(|(kind, _)| *kind != SuggestionKind::Typed),
                "{text:?}: {found:?}"
            );
        }
    }

    #[test]
    fn down_moves_from_the_typed_row_to_the_real_ones() {
        let mut search = typed("sta");

        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let found = search.suggestions(&schema());
        assert!(!found[0].is_highlighted());
        assert!(found[1].is_highlighted());
        assert_eq!(found[1].kind(), SuggestionKind::Key);
    }

    #[test]
    fn enter_on_plain_text_keeps_it_as_a_name_filter_and_closes_the_list() {
        for text in ["sta", "redis:7", "foo:bar"] {
            let mut search = typed(text);

            let change = apply(&mut search);

            assert_eq!(search.input(), text, "{text:?}");
            assert_eq!(search.name_text(&keys()), text, "{text:?}");
            assert!(!search.is_open(), "{text:?}");
            assert!(search.tags().is_empty(), "{text:?}");
            assert_eq!(change, TagSearchChange::Suggestions, "{text:?}");
        }
    }

    #[test]
    fn enter_on_a_key_and_text_adds_a_substring_tag_and_clears_the_field() {
        let mut search = typed("status:ru");

        let change = apply(&mut search);

        assert_eq!(labels(&search), vec!["status:ru"]);
        assert_eq!(search.input(), "");
        assert_eq!(change, TagSearchChange::Filter);
        assert!(search.passes(&keys(), &row("web-1", "Running", &[])));
        assert!(!search.passes(&keys(), &row("db-1", "Exited", &[])));
    }

    #[test]
    fn tab_completes_to_the_first_real_suggestion_below_the_typed_row() {
        let mut key = typed("sta");
        let _ = complete(&mut key);
        assert_eq!(key.input(), "status:");
        assert!(key.tags().is_empty());

        let mut value = typed("status:ru");
        let _ = complete(&mut value);
        assert_eq!(labels(&value), vec!["status:Running"]);
        assert_eq!(value.input(), "");

        let mut name = typed("web");
        let _ = complete(&mut name);
        assert_eq!(name.input(), "web-1");
        assert!(!name.is_open());
    }

    #[test]
    fn tab_applies_a_real_row_the_user_moved_to() {
        let mut search = typed("status:");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let _ = complete(&mut search);

        assert_eq!(labels(&search), vec!["status:Exited (3)"]);
    }

    #[test]
    fn tab_does_what_enter_does_when_no_real_suggestion_is_below() {
        let mut plain = typed("zzz");
        let _ = complete(&mut plain);
        assert_eq!(plain.input(), "zzz");
        assert!(!plain.is_open());

        let mut tag = typed("status:zzz");
        let _ = complete(&mut tag);
        assert_eq!(labels(&tag), vec!["status:zzz"]);
        assert_eq!(tag.input(), "");
    }

    #[test]
    fn closing_the_suggestions_keeps_the_text_and_the_tags() {
        let mut search = typed("status:running");
        let _ = apply(&mut search);
        let _ = search.perform(
            TagSearchAction::InputChanged(String::from("web")),
            &schema(),
        );
        let _ = search.perform(TagSearchAction::MoveDown, &schema());
        assert!(search.is_open());

        search.close_suggestions();

        assert!(!search.is_open());
        assert_eq!(search.input(), "web");
        assert_eq!(labels(&search), vec!["status:Running"]);

        let _ = search.perform(TagSearchAction::Focused, &schema());

        assert!(search.suggestions(&schema())[0].is_highlighted());
    }

    #[test]
    fn closed_suggestions_are_empty() {
        let search = TagSearch::default();

        assert!(!search.is_open());
        assert!(search.suggestions(&schema()).is_empty());
    }

    #[test]
    fn an_empty_focused_field_shows_only_the_keys_in_column_order() {
        let mut search = TagSearch::default();

        let change = search.perform(TagSearchAction::Focused, &schema());

        assert_eq!(change, TagSearchChange::Suggestions);
        assert!(search.is_open());
        assert_eq!(
            texts(&search),
            ["name", "status", "image", "ports", "compose"]
        );
        assert!(
            search
                .suggestions(&schema())
                .iter()
                .all(|found| found.kind() == SuggestionKind::Key)
        );
    }

    #[test]
    fn typing_suggests_matching_keys_in_column_order_then_matching_names() {
        let search = typed("s");

        let found = search.suggestions(&schema());

        let listed: Vec<(SuggestionKind, &str)> = found
            .iter()
            .map(|suggestion| (suggestion.kind(), suggestion.text()))
            .collect();
        assert_eq!(
            listed,
            vec![
                (SuggestionKind::Typed, "s"),
                (SuggestionKind::Key, "status"),
                (SuggestionKind::Key, "ports"),
                (SuggestionKind::Key, "compose"),
                (SuggestionKind::Name, "status-page"),
            ]
        );
    }

    #[test]
    fn names_starting_with_the_text_come_first_then_the_given_order() {
        let schema = TagSchema::new(
            keys(),
            vec![
                row("my-web", "Running", &[]),
                row("your-web", "Running", &[]),
                row("web-1", "Running", &[]),
            ],
        );
        let mut search = TagSearch::default();
        let _ = search
            .perform(TagSearchAction::InputChanged("WEB".to_owned()), &schema);

        let found: Vec<String> = search
            .suggestions(&schema)
            .iter()
            .map(|suggestion| suggestion.text().to_owned())
            .collect();

        assert_eq!(found, vec!["WEB", "web-1", "my-web", "your-web"]);
    }

    #[test]
    fn names_come_from_the_rows_the_tags_leave() {
        let mut search = typed("-1");
        let _ = search.set_tag("status", "exited");

        assert_eq!(texts(&search), vec!["-1", "db-1"]);
    }

    #[test]
    fn a_repeated_name_or_value_is_suggested_once() {
        assert_eq!(texts(&typed("status:")), vec!["Running", "Exited (3)"]);

        let schema = TagSchema::new(
            keys(),
            vec![row("<none>", "A", &[]), row("<none>", "B", &[])],
        );
        let mut search = TagSearch::default();
        let _ = search
            .perform(TagSearchAction::InputChanged("<n".to_owned()), &schema);

        let names = search
            .suggestions(&schema)
            .iter()
            .filter(|found| found.kind() == SuggestionKind::Name)
            .count();
        assert_eq!(names, 1);
    }

    #[test]
    fn after_a_key_the_values_of_that_key_are_suggested_and_filtered() {
        let found = typed("status:ex").suggestions(&schema());

        assert_eq!(found.len(), 2, "the typed row, then the value");
        assert_eq!(found[1].kind(), SuggestionKind::Value);
        assert_eq!(found[1].text(), "Exited (3)");
        assert_eq!(found[1].ranges().first(), Some(&(0..2)));
    }

    #[test]
    fn a_cell_with_several_values_suggests_each_one() {
        assert_eq!(
            texts(&typed("ports:")),
            vec!["8080:80/tcp", "8443:443/tcp", "5432/tcp"]
        );
    }

    #[test]
    fn values_come_from_the_rows_the_other_keys_leave() {
        let mut search = typed("ports:");
        let _ = search.set_tag("status", "exited");

        assert_eq!(texts(&search), vec!["5432/tcp"]);
    }

    #[test]
    fn tags_of_the_same_key_do_not_limit_its_values_but_are_not_repeated() {
        let mut search = typed("status:");
        let _ = search.set_tag("status", "running");

        assert_eq!(texts(&search), vec!["Exited (3)"]);
    }

    #[test]
    fn the_first_suggestion_is_highlighted() {
        let found = typed("s").suggestions(&schema());

        assert!(found[0].is_highlighted());
        assert_eq!(
            found.iter().filter(|found| found.is_highlighted()).count(),
            1
        );
    }

    #[test]
    fn the_matched_part_ignores_case_and_is_a_byte_range_of_the_shown_text() {
        let schema = TagSchema::new(keys(), vec![row("Сервер", "A", &[])]);
        let mut search = TagSearch::default();
        let _ = search
            .perform(TagSearchAction::InputChanged("вер".to_owned()), &schema);

        let found = search.suggestions(&schema);

        assert_eq!(found[1].text(), "Сервер");
        assert_eq!(found[1].ranges().first(), Some(&(6..12)));
    }

    #[test]
    fn empty_query_ranks_everything_in_the_given_order_with_no_ranges() {
        let ranked = rank("  ", &["b", "a"]);

        let indexes: Vec<usize> = ranked.iter().map(|one| one.index).collect();
        assert_eq!(indexes, vec![0, 1]);
        assert!(ranked.iter().all(|one| one.ranges.is_empty()));
    }

    fn ranked_indexes(query: &str, candidates: &[&str]) -> Vec<usize> {
        rank(query, candidates)
            .iter()
            .map(|one| one.index)
            .collect()
    }

    #[test]
    fn rank_finds_a_candidate_with_one_typo() {
        let found = ranked_indexes("contaner", &["image", "container"]);

        assert_eq!(found, vec![1]);
    }

    #[test]
    fn rank_finds_letters_that_are_not_next_to_each_other() {
        let found = ranked_indexes("cntnr", &["image", "container"]);

        assert_eq!(found, vec![1]);
    }

    #[test]
    fn rank_puts_the_better_match_first() {
        let found = ranked_indexes("run", &["pruned", "running", "other"]);

        assert_eq!(found, vec![1, 0]);
    }

    #[test]
    fn rank_ignores_case() {
        let found = ranked_indexes("RUNNING", &["Running", "exited"]);

        assert_eq!(found, vec![0]);
    }

    #[test]
    fn rank_allows_no_typo_in_a_short_query() {
        assert!(ranked_indexes("rx", &["running"]).is_empty());
        assert!(ranked_indexes("stx", &["status"]).is_empty());
    }

    #[test]
    fn rank_allows_at_most_one_typo() {
        assert!(ranked_indexes("contanxxr", &["container"]).is_empty());
    }

    #[test]
    fn rank_marks_the_matched_letters_as_byte_ranges() {
        let found = rank("run", &["a running"]);

        assert_eq!(found[0].ranges, vec![2..5]);
    }

    #[test]
    fn rank_marks_separate_letters_as_separate_ranges() {
        let found = rank("rg", &["rung"]);

        assert_eq!(found[0].ranges, vec![0..1, 3..4]);
    }

    #[test]
    fn rank_ranges_stay_byte_ranges_after_wide_letters() {
        let found = rank("вер", &["Сервер"]);

        assert_eq!(found[0].ranges, vec![6..12]);
    }

    // Moving.

    #[test]
    fn up_and_down_move_the_highlight_and_wrap_at_both_ends() {
        // (moves, expected position); `d` is down, `u` is up. "s" has five
        // suggestions, the typed text first.
        let cases = [("d", 1), ("dd", 2), ("ddddd", 0), ("u", 4), ("uud", 4)];

        for (moves, expected) in cases {
            let mut search = typed("s");
            for step in moves.chars() {
                let action = match step {
                    'd' => TagSearchAction::MoveDown,
                    _ => TagSearchAction::MoveUp,
                };
                assert_eq!(
                    search.perform(action, &schema()),
                    TagSearchChange::Suggestions
                );
            }

            let found = search.suggestions(&schema());
            let position = found.iter().position(Suggestion::is_highlighted);
            assert_eq!(position, Some(expected), "moves {moves:?}");
        }
    }

    #[test]
    fn typing_brings_the_highlight_back_to_the_first_suggestion() {
        let mut search = typed("s");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let _ = search
            .perform(TagSearchAction::InputChanged("st".to_owned()), &schema());

        assert!(search.suggestions(&schema())[0].is_highlighted());
    }

    #[test]
    fn moving_with_no_suggestions_changes_nothing() {
        let mut search = typed("zzz");

        let change = search.perform(TagSearchAction::MoveDown, &schema());

        assert_eq!(change, TagSearchChange::Nothing);
    }

    #[test]
    fn moving_in_a_closed_list_opens_it_without_moving_the_highlight() {
        let mut search = typed("s");
        let _ = search.perform(TagSearchAction::Close, &schema());

        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        assert!(search.is_open());
        assert!(search.suggestions(&schema())[0].is_highlighted());
    }

    #[test]
    fn a_list_that_shrank_keeps_a_highlighted_suggestion() {
        let mut search = typed("status:");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());
        let smaller =
            TagSchema::new(keys(), vec![row("web-1", "Running", &[])]);

        let found = search.suggestions(&smaller);

        assert_eq!(found.len(), 1);
        assert!(found[0].is_highlighted());
    }

    // Closing and focus.

    #[test]
    fn close_hides_the_suggestions_and_keeps_the_text() {
        let mut search = typed("s");

        let change = search.perform(TagSearchAction::Close, &schema());

        assert_eq!(change, TagSearchChange::Suggestions);
        assert!(!search.is_open());
        assert_eq!(search.input(), "s");
        assert_eq!(
            search.perform(TagSearchAction::Close, &schema()),
            TagSearchChange::Nothing
        );
    }

    #[test]
    fn typing_opens_closed_suggestions() {
        let mut search = typed("s");
        let _ = search.perform(TagSearchAction::Close, &schema());

        let _ = search
            .perform(TagSearchAction::InputChanged("st".to_owned()), &schema());

        assert!(search.is_open());
    }

    #[test]
    fn clearing_the_input_empties_the_field_and_closes_the_list() {
        let mut search = typed("s");

        search.clear_input();

        assert_eq!(search.input(), "");
        assert!(!search.is_open());
    }

    // Applying.

    #[test]
    fn applying_a_key_puts_it_in_the_field_and_shows_its_values() {
        let mut search = typed("sta");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let change = apply(&mut search);

        assert_eq!(search.input(), "status:");
        assert!(search.is_open());
        assert_eq!(texts(&search), vec!["Running", "Exited (3)"]);
        assert!(search.tags().is_empty());
        assert_eq!(change, TagSearchChange::Filter, "the name text is gone");
    }

    #[test]
    fn applying_a_value_adds_the_tag_and_clears_the_field() {
        let mut search = typed("status:run");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let change = apply(&mut search);

        assert_eq!(labels(&search), vec!["status:Running"]);
        assert_eq!(search.input(), "");
        assert_eq!(change, TagSearchChange::Filter);
    }

    #[test]
    fn after_a_value_the_empty_field_shows_the_keys_again() {
        let mut search = typed("status:run");

        let _ = apply(&mut search);

        assert!(search.is_open());
        assert_eq!(
            texts(&search),
            ["name", "status", "image", "ports", "compose"]
        );
    }

    #[test]
    fn applying_a_name_fills_the_field_closes_the_list_and_filters() {
        let mut search = typed("web");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let change = apply(&mut search);

        assert_eq!(search.input(), "web-1");
        assert_eq!(search.name_text(&keys()), "web-1");
        assert!(!search.is_open());
        assert!(search.tags().is_empty());
        assert_eq!(change, TagSearchChange::Filter);
    }

    #[test]
    fn applying_uses_the_highlighted_suggestion() {
        let mut search = typed("status:");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let _ = apply(&mut search);

        assert_eq!(labels(&search), vec!["status:Exited (3)"]);
    }

    #[test]
    fn a_value_with_a_colon_is_a_whole_value() {
        let mut search = typed("ports:8080:");
        let _ = search.perform(TagSearchAction::MoveDown, &schema());

        let _ = apply(&mut search);

        assert_eq!(labels(&search), vec!["ports:8080:80/tcp"]);
    }

    #[test]
    fn with_no_suggestion_enter_after_key_and_text_adds_that_tag() {
        let mut search = typed("status: zzz ");

        let change = apply(&mut search);

        assert_eq!(labels(&search), vec!["status:zzz"]);
        assert_eq!(search.input(), "");
        assert_eq!(change, TagSearchChange::Filter);
    }

    #[test]
    fn with_a_closed_list_enter_after_key_and_text_adds_the_typed_text() {
        let mut search = typed("status:run");
        let _ = search.perform(TagSearchAction::Close, &schema());

        let _ = apply(&mut search);

        assert_eq!(labels(&search), vec!["status:run"]);
    }

    #[test]
    fn a_key_typed_in_another_case_gives_the_schema_spelling() {
        let mut search = typed("STATUS:zzz");

        let _ = apply(&mut search);

        assert_eq!(search.tags()[0].key(), "status");
    }

    #[test]
    fn with_no_suggestion_enter_does_nothing_otherwise() {
        // Plain text and `key:text` always have the typed row now.
        let mut search = typed("compose:");
        let before = search.clone();

        let change = apply(&mut search);

        assert_eq!(change, TagSearchChange::Nothing);
        assert_eq!(search, before);
    }

    #[test]
    fn pressing_a_suggestion_applies_that_one() {
        let mut search = typed("status:");

        let _ = search.perform(TagSearchAction::Pick(1), &schema());

        assert_eq!(labels(&search), vec!["status:Exited (3)"]);
    }

    #[test]
    fn pressing_a_suggestion_that_is_gone_does_nothing() {
        let mut search = typed("status:");

        let change = search.perform(TagSearchAction::Pick(9), &schema());

        assert_eq!(change, TagSearchChange::Nothing);
        assert_eq!(search.input(), "status:");
    }

    // Removing.

    #[test]
    fn backspace_on_an_empty_field_removes_the_last_tag() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("status", "running");
        let _ = search.set_tag("compose", "web");

        let change = search.perform(TagSearchAction::RemoveLastTag, &schema());

        assert_eq!(labels(&search), vec!["status:running"]);
        assert_eq!(change, TagSearchChange::Filter);
    }

    #[test]
    fn backspace_with_text_or_no_tags_removes_nothing() {
        let mut with_text = typed("x");
        let _ = with_text.set_tag("status", "running");
        let mut no_tags = TagSearch::default();

        assert_eq!(
            with_text.perform(TagSearchAction::RemoveLastTag, &schema()),
            TagSearchChange::Nothing
        );
        assert_eq!(with_text.tags().len(), 1);
        assert_eq!(
            no_tags.perform(TagSearchAction::RemoveLastTag, &schema()),
            TagSearchChange::Nothing
        );
    }

    #[test]
    fn a_cross_removes_one_tag() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("status", "running");
        let _ = search.set_tag("compose", "web");

        let change = search.perform(TagSearchAction::RemoveTag(0), &schema());
        let missing = search.perform(TagSearchAction::RemoveTag(5), &schema());

        assert_eq!(labels(&search), vec!["compose:web"]);
        assert_eq!(change, TagSearchChange::Filter);
        assert_eq!(missing, TagSearchChange::Nothing);
    }

    #[test]
    fn clear_all_removes_every_tag_and_keeps_the_text() {
        let mut search = typed("x");
        let _ = search.set_tag("status", "running");

        let change = search.perform(TagSearchAction::ClearTags, &schema());
        let again = search.clear_tags();

        assert!(search.tags().is_empty());
        assert_eq!(search.input(), "x");
        assert_eq!(change, TagSearchChange::Filter);
        assert_eq!(again, TagSearchChange::Nothing);
    }

    #[test]
    fn a_blank_cell_has_no_value() {
        let row = SearchRow::new("web")
            .value("compose", "  ")
            .value("ports", "");

        assert_eq!(row.values_of("compose").count(), 0);
        assert_eq!(row.values_of("ports").count(), 0);
    }

    // Tags from outside and duplicates.

    #[test]
    fn set_tag_adds_a_tag_and_replaces_other_tags_of_the_key() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("status", "running");
        let _ = search.set_tag("compose", "web");

        let change = search.set_tag("compose", "shop");

        assert_eq!(labels(&search), vec!["status:running", "compose:shop"]);
        assert_eq!(change, TagSearchChange::Filter);
    }

    #[test]
    fn set_tag_with_the_same_tag_changes_nothing() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("compose", "web");

        assert_eq!(search.set_tag("compose", "web"), TagSearchChange::Nothing);
    }

    #[test]
    fn a_duplicate_tag_is_ignored_and_the_field_still_clears() {
        let mut search = typed("status:running");
        let _ = search.perform(TagSearchAction::Close, &schema());
        let _ = apply(&mut search);
        let _ = search.perform(
            TagSearchAction::InputChanged("status:RUNNING".to_owned()),
            &schema(),
        );
        let _ = search.perform(TagSearchAction::Close, &schema());

        let change = apply(&mut search);

        assert_eq!(labels(&search), vec!["status:running"]);
        assert_eq!(search.input(), "");
        assert_eq!(change, TagSearchChange::Suggestions);
    }

    // Matching.

    #[test]
    fn a_search_with_no_tags_and_no_text_filters_no_rows() {
        assert!(!TagSearch::default().filters_rows(&keys()));
        assert!(!typed("   ").filters_rows(&keys()));
        assert!(!typed("status:").filters_rows(&keys()));
    }

    #[test]
    fn a_search_with_text_or_a_tag_filters_rows() {
        let mut tagged = TagSearch::default();
        let _ = tagged.set_tag("compose", "web");

        assert!(typed("web").filters_rows(&keys()));
        assert!(tagged.filters_rows(&keys()));
    }

    #[test]
    fn with_no_tags_and_no_text_every_row_passes() {
        let search = TagSearch::default();

        assert!(search.passes(&keys(), &row("web-1", "Running", &[])));
    }

    #[test]
    fn plain_text_matches_part_of_the_name_ignoring_case() {
        let search = typed(" WEB ");

        assert!(search.passes(&keys(), &row("my-web-1", "Running", &[])));
        assert!(!search.passes(&keys(), &row("db-1", "Running", &[])));
    }

    #[test]
    fn a_half_typed_tag_filters_nothing() {
        let search = typed("status:zzz");

        assert!(search.passes(&keys(), &row("db-1", "Running", &[])));
    }

    #[test]
    fn a_tag_matches_part_of_a_value_of_its_key_ignoring_case() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("status", "RUN");

        assert!(search.passes(&keys(), &row("web-1", "Running", &[])));
        assert!(!search.passes(&keys(), &row("run", "Exited", &[])));
    }

    #[test]
    fn a_row_without_the_key_does_not_pass() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("compose", "web");

        assert!(!search.passes(&keys(), &row("web", "Running", &[])));
    }

    #[test]
    fn a_cell_with_several_values_matches_if_any_does() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("ports", "8443");

        assert!(search.passes(
            &keys(),
            &row("web", "Running", &["8080:80/tcp", "8443:443/tcp"])
        ));
        assert!(
            !search.passes(&keys(), &row("web", "Running", &["8080:80/tcp"]))
        );
    }

    #[test]
    fn different_keys_must_all_match() {
        let mut search = TagSearch::default();
        let _ = search.set_tag("status", "running");
        let _ = search.set_tag("ports", "8080");

        assert!(search.passes(&keys(), &row("a", "Running", &["8080:80/tcp"])));
        assert!(!search.passes(&keys(), &row("b", "Exited", &["8080:80/tcp"])));
        assert!(!search.passes(&keys(), &row("c", "Running", &["443/tcp"])));
    }

    #[test]
    fn several_tags_of_one_key_match_if_any_does() {
        let mut search = typed("status:running");
        let _ = search.perform(TagSearchAction::Close, &schema());
        let _ = apply(&mut search);
        let _ = search.perform(
            TagSearchAction::InputChanged("status:exited".to_owned()),
            &schema(),
        );
        let _ = search.perform(TagSearchAction::Close, &schema());
        let _ = apply(&mut search);

        assert_eq!(search.tags().len(), 2);
        assert!(search.passes(&keys(), &row("a", "Running", &[])));
        assert!(search.passes(&keys(), &row("b", "Exited", &[])));
        assert!(!search.passes(&keys(), &row("c", "Paused", &[])));
    }

    #[test]
    fn tags_and_name_text_must_both_match() {
        let mut search = typed("web");
        let _ = search.set_tag("status", "running");

        assert!(search.passes(&keys(), &row("web-1", "Running", &[])));
        assert!(!search.passes(&keys(), &row("db-1", "Running", &[])));
        assert!(!search.passes(&keys(), &row("web-2", "Exited", &[])));
    }

    #[test]
    fn passes_tags_ignores_the_name_text() {
        let mut search = typed("zzz");
        let _ = search.set_tag("status", "running");

        assert!(search.passes_tags(&row("web-1", "Running", &[])));
        assert!(!search.passes_tags(&row("web-1", "Exited", &[])));
    }

    // What changed.

    #[test]
    fn typing_plain_text_changes_the_filter_and_typing_a_tag_does_not() {
        let mut search = TagSearch::default();

        let plain = search
            .perform(TagSearchAction::InputChanged("w".to_owned()), &schema());
        let same_name_text = search
            .perform(TagSearchAction::InputChanged("w ".to_owned()), &schema());
        let _ = search
            .perform(TagSearchAction::InputChanged(String::new()), &schema());
        let half_tag = search.perform(
            TagSearchAction::InputChanged("status:r".to_owned()),
            &schema(),
        );

        assert_eq!(plain, TagSearchChange::Filter);
        assert_eq!(same_name_text, TagSearchChange::Suggestions);
        assert_eq!(half_tag, TagSearchChange::Suggestions);
    }

    // Shown names: a key is written as its column header.

    /// The keys of a table in Russian.
    fn russian_keys() -> Vec<TagKey> {
        vec![
            TagKey::new("name", "Имя"),
            TagKey::new("status", "Состояние"),
            TagKey::new("ports", "Порты"),
            TagKey::new("compose", "Проект"),
        ]
    }

    /// The same keys in English.
    fn english_keys() -> Vec<TagKey> {
        vec![
            TagKey::new("name", "Name"),
            TagKey::new("status", "Status"),
            TagKey::new("ports", "Ports"),
            TagKey::new("compose", "Compose"),
        ]
    }

    /// Rows as a Russian table shows them.
    fn russian_schema() -> TagSchema {
        TagSchema::new(
            russian_keys(),
            vec![
                row("web-1", "Работает", &["8080:80/tcp"]),
                row("db-1", "Остановлен (3)", &[]),
            ],
        )
    }

    /// A search over `schema` with `text` typed.
    fn typed_in(schema: &TagSchema, text: &str) -> TagSearch {
        let mut search = TagSearch::default();
        let _ = search
            .perform(TagSearchAction::InputChanged(text.to_owned()), schema);

        search
    }

    fn shown_in(
        search: &TagSearch,
        schema: &TagSchema,
    ) -> Vec<(SuggestionKind, String)> {
        search
            .suggestions(schema)
            .iter()
            .map(|found| (found.kind(), found.text().to_owned()))
            .collect()
    }

    /// Types `text`, closes the list and presses Enter, so the text itself
    /// is what becomes a tag.
    fn tag_typed_in(schema: &TagSchema, text: &str) -> TagSearch {
        let mut search = typed_in(schema, text);
        let _ = search.perform(TagSearchAction::Close, schema);
        let _ = search.perform(TagSearchAction::Apply, schema);

        search
    }

    #[test]
    fn a_shown_name_ends_at_the_first_colon_of_either_kind() {
        // (column header, shown name)
        let cases = [
            ("Status", "Status"),
            ("Repository:Tag", "Repository"),
            ("Репозиторий:тег", "Репозиторий"),
            ("仓库：标签", "仓库"),
            ("Used by", "Used by"),
            ("Genutzt von", "Genutzt von"),
            ("  Used by : x", "Used by"),
        ];

        for (header, shown) in cases {
            let key = TagKey::new("key", header);

            assert_eq!(key.shown(), shown, "{header:?}");
            assert_eq!(key.name(), "key", "{header:?}");
        }
    }

    #[test]
    fn a_shown_name_is_recognized_in_any_case_in_any_alphabet() {
        let schema = TagSchema::new(
            vec![
                TagKey::new("name", "Имя"),
                TagKey::new("status", "Состояние"),
                TagKey::new("size", "Größe"),
                TagKey::new("state", "État"),
            ],
            Vec::new(),
        );
        // (typed key, its internal name)
        let cases = [
            ("Имя", "name"),
            ("имя", "name"),
            ("ИМЯ", "name"),
            (" иМя ", "name"),
            ("СОСТОЯНИЕ", "status"),
            ("Größe", "size"),
            ("GRÖSSE", "size"),
            ("größe", "size"),
            ("état", "state"),
            ("ÉTAT", "state"),
        ];

        for (key, name) in cases {
            let text = format!("{key}:x");

            let search = tag_typed_in(&schema, &text);

            assert_eq!(search.input(), "", "{text:?}");
            assert_eq!(search.tags()[0].key(), name, "{text:?}");
            assert_eq!(search.tags()[0].value(), "x", "{text:?}");
        }
    }

    #[test]
    fn a_shown_name_with_a_space_stays_whole() {
        let schema = TagSchema::new(
            vec![TagKey::new("used-by", "Used by"), TagKey::new("age", "Age")],
            vec![SearchRow::new("data").value("used-by", "db-1")],
        );

        for text in ["Used by:db-1", "used BY : db-1"] {
            let search = tag_typed_in(&schema, text);

            assert_eq!(search.tags()[0].key(), "used-by", "{text:?}");
            assert_eq!(search.tags()[0].value(), "db-1", "{text:?}");
        }
        assert_eq!(
            typed_in(&schema, "Used:db-1").name_text(schema.keys()),
            "Used:db-1"
        );

        let mut search = typed_in(&schema, "used");
        let _ = search.perform(TagSearchAction::Complete, &schema);
        assert_eq!(search.input(), "Used by:");
    }

    #[test]
    fn the_internal_name_is_not_a_key_when_it_differs_from_the_shown_name() {
        let schema = russian_schema();

        for text in ["status:x", "STATUS:x", "name:web", "compose:x"] {
            let mut search = typed_in(&schema, text);

            assert_eq!(search.name_text(schema.keys()), text, "{text:?}");
            assert_eq!(
                shown_in(&search, &schema),
                vec![(SuggestionKind::Typed, text.to_owned())],
                "{text:?}"
            );

            let _ = search.perform(TagSearchAction::Apply, &schema);

            assert!(search.tags().is_empty(), "{text:?}");
            assert_eq!(search.input(), text, "{text:?}");
        }
    }

    #[test]
    fn the_internal_name_is_not_suggested_when_it_differs() {
        let schema = russian_schema();

        for text in ["stat", "compose", "ports"] {
            assert!(
                shown_in(&typed_in(&schema, text), &schema)
                    .iter()
                    .all(|(kind, _)| *kind != SuggestionKind::Key),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_wide_colon_counts_as_a_colon() {
        let schema = TagSchema::new(
            vec![TagKey::new("status", "状态"), TagKey::new("name", "Имя")],
            Vec::new(),
        );
        // (typed text, internal name, value)
        let cases = [
            ("状态：运行中", "status", "运行中"),
            ("状态:运行中", "status", "运行中"),
            ("Имя：web", "name", "web"),
            (" имя ： web ", "name", "web"),
        ];

        for (text, name, value) in cases {
            let search = tag_typed_in(&schema, text);

            assert_eq!(search.tags()[0].key(), name, "{text:?}");
            assert_eq!(search.tags()[0].value(), value, "{text:?}");
        }
    }

    #[test]
    fn text_before_a_colon_that_is_no_shown_name_stays_plain_text() {
        let schema = russian_schema();

        for text in ["redis:7", "redis：7", "foo:bar", ":x", "：x"] {
            let search = typed_in(&schema, text);

            assert_eq!(search.name_text(schema.keys()), text, "{text:?}");
            assert_eq!(
                shown_in(&search, &schema)[0],
                (SuggestionKind::Typed, text.to_owned()),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_value_with_colons_of_either_kind_stays_whole() {
        let schema = russian_schema();
        // (typed text, value)
        let cases = [
            ("Порты:8080:80/tcp", "8080:80/tcp"),
            ("Порты：8080:80/tcp", "8080:80/tcp"),
            ("Порты:8080：80/tcp", "8080：80/tcp"),
        ];

        for (text, value) in cases {
            let search = tag_typed_in(&schema, text);

            assert_eq!(search.tags()[0].key(), "ports", "{text:?}");
            assert_eq!(search.tags()[0].value(), value, "{text:?}");
        }
    }

    #[test]
    fn suggestions_show_the_shown_names_in_column_order() {
        let schema = russian_schema();
        let mut search = TagSearch::default();

        let _ = search.perform(TagSearchAction::Focused, &schema);

        let found: Vec<String> = shown_in(&search, &schema)
            .into_iter()
            .map(|(_, text)| text)
            .collect();
        assert_eq!(found, ["Имя", "Состояние", "Порты", "Проект"]);
    }

    #[test]
    fn typing_part_of_a_shown_name_suggests_it_in_any_case() {
        let schema = russian_schema();

        for text in ["Сост", "сост", "СОСТ"] {
            assert_eq!(
                shown_in(&typed_in(&schema, text), &schema),
                vec![
                    (SuggestionKind::Typed, text.to_owned()),
                    (SuggestionKind::Key, "Состояние".to_owned()),
                ],
                "{text:?}"
            );
        }
    }

    #[test]
    fn the_typed_row_takes_the_shown_spelling_of_the_key() {
        let schema = russian_schema();

        for text in [" СОСТОЯНИЕ : zzz ", "состояние：zzz"] {
            assert_eq!(
                shown_in(&typed_in(&schema, text), &schema),
                vec![(SuggestionKind::Typed, "Состояние:zzz".to_owned())],
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_shown_name_typed_whole_in_another_case_has_no_second_row() {
        let schema = russian_schema();

        assert_eq!(
            shown_in(&typed_in(&schema, "СОСТОЯНИЕ"), &schema),
            vec![(SuggestionKind::Key, "Состояние".to_owned())]
        );
    }

    #[test]
    fn applying_a_key_writes_its_shown_name_in_the_field() {
        let schema = russian_schema();
        let mut search = typed_in(&schema, "сост");

        let _ = search.perform(TagSearchAction::Complete, &schema);

        assert_eq!(search.input(), "Состояние:");
        assert_eq!(search.name_text(schema.keys()), "");
        assert_eq!(
            shown_in(&search, &schema),
            vec![
                (SuggestionKind::Value, "Работает".to_owned()),
                (SuggestionKind::Value, "Остановлен (3)".to_owned()),
            ]
        );
    }

    #[test]
    fn a_tag_stores_the_internal_name_and_shows_the_shown_name() {
        let schema = russian_schema();
        let mut search = typed_in(&schema, "Сост");

        let _ = search.perform(TagSearchAction::Complete, &schema);
        let _ = search.perform(
            TagSearchAction::InputChanged("Состояние:раб".to_owned()),
            &schema,
        );
        let _ = search.perform(TagSearchAction::Complete, &schema);

        let tag = &search.tags()[0];
        assert_eq!(tag.key(), "status");
        assert_eq!(tag.value(), "Работает");
        assert_eq!(tag.label(schema.keys()), "Состояние:Работает");
        assert_eq!(search.input(), "");
    }

    #[test]
    fn a_tag_key_shows_the_name_of_the_keys_it_is_given() {
        let schema = russian_schema();
        let search = tag_typed_in(&schema, "Состояние:Работает");
        let tag = &search.tags()[0];

        assert_eq!(tag.label(&russian_keys()), "Состояние:Работает");
        assert_eq!(tag.label(&english_keys()), "Status:Работает");
    }

    #[test]
    fn a_tag_of_a_key_that_is_not_given_reads_its_internal_name() {
        let tag = Tag::new("status", "Running");

        assert_eq!(tag.label(&[]), "status:Running");
    }

    #[test]
    fn tags_filter_by_the_internal_name_in_any_language() {
        let schema = russian_schema();
        let search = tag_typed_in(&schema, "состояние:работ");
        let web = &schema.rows()[0];
        let db = &schema.rows()[1];

        assert!(search.passes(&russian_keys(), web));
        assert!(!search.passes(&russian_keys(), db));
        assert!(search.passes(&english_keys(), web));
    }

    #[test]
    fn values_of_a_key_come_by_the_internal_name_of_the_typed_key() {
        let schema = russian_schema();

        assert_eq!(
            shown_in(&typed_in(&schema, "ПОРТЫ:"), &schema),
            vec![(SuggestionKind::Value, "8080:80/tcp".to_owned())]
        );
    }

    #[test]
    fn a_link_replaces_a_typed_tag_by_the_internal_name() {
        let schema = russian_schema();
        let mut search = tag_typed_in(&schema, "проект:shop");

        let change = search.set_tag("compose", "web");

        assert_eq!(change, TagSearchChange::Filter);
        assert_eq!(search.tags().len(), 1);
        assert_eq!(search.tags()[0].label(schema.keys()), "Проект:web");
    }

    #[test]
    fn text_typed_in_one_language_is_a_name_filter_in_another() {
        let search = typed_in(&russian_schema(), "Имя:web");

        assert_eq!(search.name_text(&russian_keys()), "");
        assert_eq!(search.name_text(&english_keys()), "Имя:web");
    }
}
