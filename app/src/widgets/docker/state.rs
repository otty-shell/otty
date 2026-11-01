use otty_docker::{
    Container, Diagnosis, Discovery, EngineAddress, EngineInfo, Image, Network,
    Volume,
};

use super::types::{
    LoadedLists, Place, ResourceKind, container_name, filtered, image_name,
    network_name, volume_name,
};

/// Runtime state of Docker mode: the engines and their lists, the open
/// place, the selected row of each list and the name filter.
#[derive(Debug, Default)]
pub(crate) struct DockerState {
    is_active: bool,
    search: Search,
    engines: Vec<EngineState>,
    place: Option<Place>,
    filter: String,
}

impl DockerState {
    /// Return whether Docker mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.is_active
    }

    /// Return how far the engine search is.
    pub(crate) fn search(&self) -> &Search {
        &self.search
    }

    /// Return the reached engines, in search order.
    pub(crate) fn engines(&self) -> &[EngineState] {
        &self.engines
    }

    /// Return the list the main area shows; `None` without engines.
    pub(crate) fn place(&self) -> Option<&Place> {
        self.place.as_ref()
    }

    /// Return the name filter above the table.
    pub(crate) fn filter(&self) -> &str {
        &self.filter
    }

    /// Return the engine of the open place.
    pub(crate) fn open_engine(&self) -> Option<&EngineState> {
        let place = self.place.as_ref()?;

        self.engine(place.engine())
    }

    /// Return the engine listening at `address`.
    pub(crate) fn engine(
        &self,
        address: &EngineAddress,
    ) -> Option<&EngineState> {
        self.engines
            .iter()
            .find(|engine| engine.info().address() == address)
    }

    /// Mark Docker mode as active or not.
    pub(super) fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

    /// Start a new engine search. Engines found before stay shown until it
    /// finishes.
    pub(super) fn start_search(&mut self) {
        self.search = Search::Searching;
    }

    /// Take the search result: keep the lists of engines found again, add
    /// new engines, drop engines that are gone. The open place stays when
    /// its engine is still there, else it moves to the first engine's
    /// containers.
    pub(super) fn finish_search(&mut self, discovery: Discovery) {
        let reached = match discovery.diagnosis() {
            Diagnosis::Engines(infos) => infos.clone(),
            _ => Vec::new(),
        };

        let mut previous = std::mem::take(&mut self.engines);
        self.engines = reached
            .into_iter()
            .map(|info| {
                match previous
                    .iter()
                    .position(|engine| engine.info.address() == info.address())
                {
                    Some(index) => {
                        let mut engine = previous.swap_remove(index);
                        engine.info = info;
                        engine
                    },
                    None => EngineState::new(info),
                }
            })
            .collect();

        let place_is_kept = self
            .place
            .as_ref()
            .is_some_and(|place| self.engine(place.engine()).is_some());
        if !place_is_kept {
            self.place = self.engines.first().map(|engine| {
                Place::new(
                    engine.info.address().clone(),
                    ResourceKind::Containers,
                )
            });
        }

        self.search = Search::Done(discovery);
    }

    /// Take the lists of one load. Returns `false` when the engine is no
    /// longer known.
    pub(super) fn apply_lists(
        &mut self,
        address: &EngineAddress,
        lists: LoadedLists,
    ) -> bool {
        let Some(engine) = self
            .engines
            .iter_mut()
            .find(|engine| engine.info.address() == address)
        else {
            return false;
        };

        engine.apply_lists(lists);
        true
    }

    /// Show `place`. Opening another place clears the filter.
    pub(super) fn open(&mut self, place: Place) {
        if self.place.as_ref() == Some(&place) {
            return;
        }

        self.place = Some(place);
        self.filter.clear();
    }

    /// Replace the name filter.
    pub(super) fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }

    /// Select the row at `index` of the shown, filtered table. Returns
    /// `false` when there is no such row.
    pub(super) fn select_shown_row(&mut self, index: usize) -> bool {
        let Some(place) = self.place.clone() else {
            return false;
        };
        let Some(engine) = self
            .engines
            .iter_mut()
            .find(|engine| engine.info.address() == place.engine())
        else {
            return false;
        };

        let filter = self.filter.as_str();
        let id = match place.resource() {
            ResourceKind::Containers => {
                filtered(engine.containers.items(), filter, container_name)
                    .get(index)
                    .map(|container| container.id().to_string())
            },
            ResourceKind::Images => {
                filtered(engine.images.items(), filter, image_name)
                    .get(index)
                    .map(|image| image.id().to_string())
            },
            ResourceKind::Volumes => {
                filtered(engine.volumes.items(), filter, volume_name)
                    .get(index)
                    .map(|volume| volume.name().to_string())
            },
            ResourceKind::Networks => {
                filtered(engine.networks.items(), filter, network_name)
                    .get(index)
                    .map(|network| network.id().to_string())
            },
        };
        let Some(id) = id else {
            return false;
        };

        match place.resource() {
            ResourceKind::Containers => engine.containers.select(id),
            ResourceKind::Images => engine.images.select(id),
            ResourceKind::Volumes => engine.volumes.select(id),
            ResourceKind::Networks => engine.networks.select(id),
        }
        true
    }
}

/// How far the engine search is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) enum Search {
    /// A search runs (or none ran yet).
    #[default]
    Searching,
    /// The last search finished; its result says which engines answered or
    /// why none did (ticket 10 shows the reason).
    Done(Discovery),
}

/// One reached engine and its four lists.
#[derive(Debug)]
pub(crate) struct EngineState {
    info: EngineInfo,
    containers: ResourceList<Container>,
    images: ResourceList<Image>,
    volumes: ResourceList<Volume>,
    networks: ResourceList<Network>,
}

impl EngineState {
    /// Return what the engine said about itself.
    pub(crate) fn info(&self) -> &EngineInfo {
        &self.info
    }

    /// Return the containers list.
    pub(crate) fn containers(&self) -> &ResourceList<Container> {
        &self.containers
    }

    /// Return the images list.
    pub(crate) fn images(&self) -> &ResourceList<Image> {
        &self.images
    }

    /// Return the volumes list.
    pub(crate) fn volumes(&self) -> &ResourceList<Volume> {
        &self.volumes
    }

    /// Return the networks list.
    pub(crate) fn networks(&self) -> &ResourceList<Network> {
        &self.networks
    }

    /// A reached engine whose lists are not loaded yet.
    fn new(info: EngineInfo) -> Self {
        Self {
            info,
            containers: ResourceList::new(),
            images: ResourceList::new(),
            volumes: ResourceList::new(),
            networks: ResourceList::new(),
        }
    }

    /// Take each list of one load on its own.
    fn apply_lists(&mut self, lists: LoadedLists) {
        self.containers.apply(lists.containers, Container::id);
        self.images.apply(lists.images, Image::id);
        self.volumes.apply(lists.volumes, Volume::name);
        self.networks.apply(lists.networks, Network::id);
    }
}

/// One list of an engine and the id of its selected row.
#[derive(Debug)]
pub(crate) struct ResourceList<T> {
    load: ListLoad<T>,
    selected: Option<String>,
}

impl<T> ResourceList<T> {
    /// Return whether the list is loading, loaded or failed.
    pub(crate) fn load(&self) -> &ListLoad<T> {
        &self.load
    }

    /// Return the loaded items; empty while loading or after a failure.
    pub(crate) fn items(&self) -> &[T] {
        match &self.load {
            ListLoad::Loaded(items) => items,
            _ => &[],
        }
    }

    /// Return the id of the selected row.
    pub(crate) fn selected_id(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// Return the selected item, if it is in the loaded list.
    pub(crate) fn selected(&self, id_of: fn(&T) -> &str) -> Option<&T> {
        let selected = self.selected.as_deref()?;

        self.items().iter().find(|item| id_of(item) == selected)
    }

    /// A list that is not loaded yet.
    fn new() -> Self {
        Self {
            load: ListLoad::Loading,
            selected: None,
        }
    }

    /// Take a load result. A loaded list keeps the selected id when it is
    /// still there and selects the first row otherwise; a failed list keeps
    /// the id, so the row is selected again once it loads.
    fn apply(&mut self, result: Result<Vec<T>, String>, id_of: fn(&T) -> &str) {
        let items = match result {
            Ok(items) => items,
            Err(message) => {
                self.load = ListLoad::Failed(message);
                return;
            },
        };

        let is_still_there = self
            .selected
            .as_deref()
            .is_some_and(|id| items.iter().any(|item| id_of(item) == id));
        if !is_still_there {
            self.selected = items.first().map(|item| id_of(item).to_string());
        }

        self.load = ListLoad::Loaded(items);
    }

    /// Select the row with `id`.
    fn select(&mut self, id: String) {
        self.selected = Some(id);
    }
}

/// Load state of one list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ListLoad<T> {
    /// The first load has not finished.
    Loading,
    /// The engine sent the list. A reload keeps it until the new one comes.
    Loaded(Vec<T>),
    /// The engine could not send the list; the engine's message.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use otty_docker::{EngineKind, fixtures};

    use super::*;

    #[test]
    fn a_loaded_list_selects_its_first_row() {
        let mut list = ResourceList::new();

        list.apply(Ok(vec!["a", "b"]), |item| item);

        assert_eq!(list.items(), &["a", "b"]);
        assert_eq!(list.selected_id(), Some("a"));
    }

    #[test]
    fn a_reload_keeps_the_selection_while_the_row_is_there() {
        let mut list = ResourceList::new();
        list.apply(Ok(vec!["a", "b"]), |item| item);
        list.select(String::from("b"));

        list.apply(Ok(vec!["c", "b"]), |item| item);

        assert_eq!(list.selected_id(), Some("b"));
        assert_eq!(list.selected(|item| item), Some(&"b"));
    }

    #[test]
    fn a_reload_without_the_row_selects_the_first_one() {
        let mut list = ResourceList::new();
        list.apply(Ok(vec!["a", "b"]), |item| item);
        list.select(String::from("b"));

        list.apply(Ok(vec!["c", "d"]), |item| item);

        assert_eq!(list.selected_id(), Some("c"));
    }

    #[test]
    fn an_empty_reload_selects_nothing() {
        let mut list = ResourceList::new();
        list.apply(Ok(vec!["a"]), |item| item);

        list.apply(Ok(Vec::new()), |item| item);

        assert_eq!(list.selected_id(), None);
        assert!(list.items().is_empty());
    }

    #[test]
    fn a_failed_load_shows_the_error_and_no_rows() {
        let mut list = ResourceList::new();
        list.apply(Ok(vec!["a"]), |item| item);

        list.apply(Err(String::from("engine gone")), |item| item);

        assert_eq!(list.load(), &ListLoad::Failed(String::from("engine gone")));
        assert!(list.items().is_empty());
        assert_eq!(list.selected(|item| item), None);
    }

    #[test]
    fn a_new_state_searches_and_shows_nothing() {
        let state = DockerState::default();

        assert_eq!(state.search(), &Search::Searching);
        assert!(state.engines().is_empty());
        assert!(state.place().is_none());
        assert!(!state.is_active());
    }

    #[test]
    fn a_new_engine_starts_with_every_list_loading() {
        let engine = EngineState::new(fixtures::engine(EngineKind::Docker));

        assert_eq!(engine.containers().load(), &ListLoad::Loading);
        assert_eq!(engine.images().load(), &ListLoad::Loading);
        assert_eq!(engine.volumes().load(), &ListLoad::Loading);
        assert_eq!(engine.networks().load(), &ListLoad::Loading);
    }
}
