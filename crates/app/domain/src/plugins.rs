//! What the Plugins page and Agents mode tell each other about the
//! installed plugins (spec 2026-11-19-agent-plugins, "What the UI shows"):
//! where the page opens, the parts that are off and the updates that wait
//! for the user's OK.

/// Where the Plugins page opens.
///
/// # Examples
///
/// ```
/// use otty_app_domain::plugins::PluginsPlace;
///
/// let read_by = PluginsPlace::Plugin(String::from("example"));
///
/// assert_ne!(read_by, PluginsPlace::List);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginsPlace {
    /// The list of the installed plugins.
    List,
    /// The URL field of Add plugin…, empty.
    AddPlugin,
    /// The row of the plugin with this id, as "Read by" links to it.
    Plugin(String),
    /// The update dialog of the plugin with this id, whose update waits
    /// for the user's OK.
    Review(String),
}

/// Whether a part serves an agent or a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSide {
    /// An agent part, such as Coder.
    Agent,
    /// A provider part, such as Example Cloud.
    Provider,
}

/// A part of an installed plugin that is off: the account need "<Part> of
/// the <Plugin> plugin is off [Turn on]" and "Read by: <Plugin> <version>
/// (off)" name it. A plain bag of names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffPart {
    /// The plugin id, such as `example`.
    pub plugin: String,
    /// The plugin's name, such as "Example Cloud".
    pub plugin_name: String,
    /// The plugin's installed version, such as `0.2.0`.
    pub version: String,
    /// Whether the part serves an agent or a provider.
    pub side: PartSide,
    /// The part id: the agent id or the provider id it serves.
    pub part: String,
    /// The part's name, such as "Example Cloud".
    pub part_name: String,
    /// The hosts a provider part serves.
    pub hosts: Vec<String>,
}

/// An update of a plugin that asks for new permissions or parts and waits
/// for the user's OK: "Plugin update needs your OK: Example 0.4.0 → 0.5.0".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingUpdate {
    /// The plugin id.
    pub plugin: String,
    /// The plugin's name.
    pub name: String,
    /// The installed version.
    pub from: String,
    /// The waiting version.
    pub to: String,
}

/// What Agents mode knows of the installed plugins besides the registry,
/// which holds only the parts that are on.
///
/// # Examples
///
/// ```
/// use otty_app_domain::plugins::PluginNotes;
///
/// let none = PluginNotes::default();
///
/// assert!(none.off_parts.is_empty());
/// assert!(none.waiting_updates.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginNotes {
    /// The parts of installed plugins that are off.
    pub off_parts: Vec<OffPart>,
    /// The updates that wait for the user's OK.
    pub waiting_updates: Vec<WaitingUpdate>,
}
