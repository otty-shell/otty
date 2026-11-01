use std::path::Path;

use iced::Point;
use otty_docker::{
    Container, Diagnosis, Discovery, EngineAddress, EngineInfo, EngineKind,
    Image, Network, Volume,
};

use super::actions::{ActionRequest, Confirm};
use super::forms::Form;
use super::inspector::{
    InspectTarget, InspectorState, InspectorTab, LogTarget,
};
use super::run_form::RunForm;
use super::sort::{TableSort, TableSorts};
use super::types::{
    LoadedList, LoadedLists, Place, ResourceKind, shown_containers,
    shown_images, shown_networks, shown_volumes,
};

/// Runtime state of Docker mode: the engines and their lists, the open
/// place, the selected row of each list, the name filter and the actions:
/// the ones that run, the open confirmation, the row menu and the notice of
/// the last failure.
#[derive(Debug, Default)]
pub(crate) struct DockerState {
    is_active: bool,
    search: Search,
    engines: Vec<EngineState>,
    place: Option<Place>,
    filter: String,
    /// The kept sort of each table; not saved to disk.
    sorts: TableSorts,
    inspector: InspectorState,
    running: Vec<ActionRequest>,
    confirm: Option<Confirm>,
    /// The open row menu.
    row_menu: Option<RowMenu>,
    notice: Option<String>,
    /// The open create dialog.
    form: Option<Form>,
    /// The open Run container form; it replaces the table.
    run_form: Option<RunForm>,
}

impl DockerState {
    /// Return the open Run container form.
    pub(crate) fn run_form(&self) -> Option<&RunForm> {
        self.run_form.as_ref()
    }

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

    /// Return the full path of the command line tool of `kind`, when the
    /// last search found it (a `podman-docker` wrapper counts as Podman).
    pub(crate) fn cli_path(&self, kind: EngineKind) -> Option<&Path> {
        let Search::Done(discovery) = &self.search else {
            return None;
        };

        match kind {
            EngineKind::Docker => discovery.cli().docker_path(),
            EngineKind::Podman => discovery.cli().podman_path(),
        }
    }

    /// Return the name filter above the table.
    pub(crate) fn filter(&self) -> &str {
        &self.filter
    }

    /// Return how the `resource` table is sorted.
    pub(crate) fn sort(&self, resource: ResourceKind) -> TableSort {
        self.sorts.get(resource)
    }

    /// Return the state of the inspector tabs.
    pub(crate) fn inspector(&self) -> &InspectorState {
        &self.inspector
    }

    /// Return the container whose logs should be followed now: Docker mode
    /// is active, the Logs tab is open and a container is selected.
    pub(crate) fn wanted_log_target(&self) -> Option<LogTarget> {
        if !self.is_active || self.inspector.tab() != InspectorTab::Logs {
            return None;
        }

        let (engine, row) = self.selected_row()?;
        let SelectedRow::Container(container) = row else {
            return None;
        };

        Some(LogTarget::new(engine.info.clone(), container.id()))
    }

    /// Return the row whose inspect JSON should be shown now, with its
    /// engine: Docker mode is active, the Inspect tab is open and a row is
    /// selected.
    pub(crate) fn wanted_inspect_target(
        &self,
    ) -> Option<(EngineInfo, InspectTarget)> {
        if !self.is_active || self.inspector.tab() != InspectorTab::Inspect {
            return None;
        }

        let (engine, row) = self.selected_row()?;
        let target = InspectTarget::new(
            engine.info.address().clone(),
            row.resource(),
            row.id(),
        );

        Some((engine.info.clone(), target))
    }

    /// Return the selected row of the open list and its engine; `None` when
    /// nothing is selected or the row is not in the loaded list.
    pub(crate) fn selected_row(
        &self,
    ) -> Option<(&EngineState, SelectedRow<'_>)> {
        let engine = self.open_engine()?;
        let place = self.place.as_ref()?;

        let row = match place.resource() {
            ResourceKind::Containers => SelectedRow::Container(
                engine.containers.selected(Container::id)?,
            ),
            ResourceKind::Images => {
                SelectedRow::Image(engine.images.selected(Image::id)?)
            },
            ResourceKind::Volumes => {
                SelectedRow::Volume(engine.volumes.selected(Volume::name)?)
            },
            ResourceKind::Networks => {
                SelectedRow::Network(engine.networks.selected(Network::id)?)
            },
        };

        Some((engine, row))
    }

    /// Return the action that waits for the user to confirm it.
    pub(crate) fn confirm(&self) -> Option<&Confirm> {
        self.confirm.as_ref()
    }

    /// Return where the row menu is open, in window coordinates.
    pub(crate) fn row_menu(&self) -> Option<Point> {
        self.row_menu.as_ref().map(|menu| menu.position)
    }

    /// Return the id of the row the open row menu is for.
    pub(crate) fn row_menu_target(&self) -> Option<&str> {
        self.row_menu.as_ref().map(|menu| menu.target.as_str())
    }

    /// Return the id of the selected row of the open list.
    pub(crate) fn selected_id(&self) -> Option<&str> {
        let place = self.place.as_ref()?;
        let engine = self.open_engine()?;

        match place.resource() {
            ResourceKind::Containers => engine.containers.selected_id(),
            ResourceKind::Images => engine.images.selected_id(),
            ResourceKind::Volumes => engine.volumes.selected_id(),
            ResourceKind::Networks => engine.networks.selected_id(),
        }
    }

    /// Return the notice of the last failed action.
    pub(crate) fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Return the open create dialog.
    pub(crate) fn form(&self) -> Option<&Form> {
        self.form.as_ref()
    }

    /// Return whether an action runs on the row `target` (or, with `None`,
    /// a prune on the whole list) of the `resource` list of the engine at
    /// `address`.
    pub(crate) fn is_busy(
        &self,
        address: &EngineAddress,
        resource: ResourceKind,
        target: Option<&str>,
    ) -> bool {
        self.running
            .iter()
            .any(|request| request.acts_on(address, resource, target))
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

    /// Return the inspector tabs for a change.
    pub(super) fn inspector_mut(&mut self) -> &mut InspectorState {
        &mut self.inspector
    }

    /// Open the inspector `tab` when the open list has it (only containers
    /// have Logs).
    pub(super) fn select_inspector_tab(&mut self, tab: InspectorTab) {
        let Some(place) = self.place.as_ref() else {
            return;
        };
        if !InspectorTab::of(place.resource()).contains(&tab) {
            return;
        }

        self.inspector.select_tab(tab);
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

    /// Take one reloaded list. Returns `false` when the engine is no longer
    /// known.
    pub(super) fn apply_list(
        &mut self,
        address: &EngineAddress,
        list: LoadedList,
    ) -> bool {
        let Some(engine) = self
            .engines
            .iter_mut()
            .find(|engine| engine.info.address() == address)
        else {
            return false;
        };

        engine.apply_list(list);
        true
    }

    /// Show `place`. Opening another place clears the filter and goes
    /// back to the Overview tab.
    pub(super) fn open(&mut self, place: Place) {
        if self.place.as_ref() == Some(&place) {
            return;
        }

        self.place = Some(place);
        self.filter.clear();
        self.inspector.select_tab(InspectorTab::Overview);
        self.notice = None;
    }

    /// Remember that `request` runs, so its buttons are disabled.
    pub(super) fn start_action(&mut self, request: ActionRequest) {
        self.running.push(request);
    }

    /// Forget a finished `request`.
    pub(super) fn finish_action(&mut self, request: &ActionRequest) {
        if let Some(index) =
            self.running.iter().position(|running| running == request)
        {
            self.running.remove(index);
        }
    }

    /// Ask the user to confirm an action.
    pub(super) fn open_confirm(&mut self, confirm: Confirm) {
        self.confirm = Some(confirm);
    }

    /// Close the confirmation and return it.
    pub(super) fn take_confirm(&mut self) -> Option<Confirm> {
        self.confirm.take()
    }

    /// Turn the switch of the open confirmation on or off.
    pub(super) fn set_confirm_switch(&mut self, is_on: bool) {
        if let Some(confirm) = self.confirm.as_mut() {
            confirm.set_switch(is_on);
        }
    }

    /// Open the menu of the row with the id `target` at `position`.
    pub(super) fn open_row_menu(&mut self, position: Point, target: String) {
        self.row_menu = Some(RowMenu { position, target });
    }

    /// Close the row menu.
    pub(super) fn close_row_menu(&mut self) {
        self.row_menu = None;
    }

    /// Show `notice` above the table until it is dismissed.
    pub(super) fn set_notice(&mut self, notice: String) {
        self.notice = Some(notice);
    }

    /// Hide the notice.
    pub(super) fn dismiss_notice(&mut self) {
        self.notice = None;
    }

    /// Open a create dialog.
    pub(super) fn open_form(&mut self, form: Form) {
        self.form = Some(form);
    }

    /// Return the open create dialog to change it.
    pub(super) fn form_mut(&mut self) -> Option<&mut Form> {
        self.form.as_mut()
    }

    /// Close the create dialog and return it.
    pub(super) fn take_form(&mut self) -> Option<Form> {
        self.form.take()
    }

    /// Show the Run container form in place of the table.
    pub(super) fn open_run_form(&mut self, form: RunForm) {
        self.run_form = Some(form);
    }

    /// Return the open Run container form to change it.
    pub(super) fn run_form_mut(&mut self) -> Option<&mut RunForm> {
        self.run_form.as_mut()
    }

    /// Close the Run container form and return it.
    pub(super) fn take_run_form(&mut self) -> Option<RunForm> {
        self.run_form.take()
    }

    /// Show the containers of `engine` without a filter and select the
    /// container with `id`, which a reload brings into the list.
    pub(super) fn show_new_container(
        &mut self,
        engine: &EngineAddress,
        id: String,
    ) {
        self.open(Place::new(engine.clone(), ResourceKind::Containers));
        self.filter.clear();

        if let Some(engine) = self
            .engines
            .iter_mut()
            .find(|state| state.info.address() == engine)
        {
            engine.containers.select(id);
        }
    }

    /// Show all containers of the open engine again, leaving a Compose
    /// project.
    pub(super) fn clear_compose_project(&mut self) {
        let Some(place) = self.place.as_ref() else {
            return;
        };
        if place.compose_project().is_none() {
            return;
        }

        let all = Place::new(place.engine().clone(), ResourceKind::Containers);
        self.open(all);
    }

    /// Show only the containers of the Compose project of the container at
    /// `index` of the shown, filtered containers table. Returns `false`
    /// when there is no such row or it has no project.
    pub(super) fn open_compose_project_of_row(&mut self, index: usize) -> bool {
        let (Some(place), Some(engine)) =
            (self.place.as_ref(), self.open_engine())
        else {
            return false;
        };
        if place.resource() != ResourceKind::Containers {
            return false;
        }

        let shown = shown_containers(
            engine.containers.items(),
            self.filter.as_str(),
            place.compose_project(),
            self.sorts.get(ResourceKind::Containers),
        );
        let Some(project) = shown
            .get(index)
            .and_then(|container| container.compose_project())
        else {
            return false;
        };

        let compose = Place::compose(place.engine().clone(), project);
        self.open(compose);
        true
    }

    /// Sort the open table by the column at `column`, or turn its order
    /// around when it is sorted by that column already. The selected item
    /// stays selected: a selection is the item's id, not a position.
    pub(super) fn sort_by(&mut self, column: usize) {
        let Some(place) = self.place.as_ref() else {
            return;
        };

        self.sorts = self.sorts.pressed(place.resource(), column);
    }

    /// Replace the name filter.
    pub(super) fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }

    /// Return the id of the row at `index` of the shown, filtered table;
    /// `None` when there is no such row.
    pub(super) fn shown_row_id(&self, index: usize) -> Option<String> {
        let place = self.place.as_ref()?;
        let engine = self.open_engine()?;
        let filter = self.filter.as_str();
        let sort = self.sorts.get(place.resource());

        let id = match place.resource() {
            ResourceKind::Containers => shown_containers(
                engine.containers.items(),
                filter,
                place.compose_project(),
                sort,
            )
            .get(index)
            .map(|container| container.id()),
            ResourceKind::Images => {
                shown_images(engine.images.items(), filter, sort)
                    .get(index)
                    .map(|image| image.id())
            },
            ResourceKind::Volumes => {
                shown_volumes(engine.volumes.items(), filter, sort)
                    .get(index)
                    .map(|volume| volume.name())
            },
            ResourceKind::Networks => {
                shown_networks(engine.networks.items(), filter, sort)
                    .get(index)
                    .map(|network| network.id())
            },
        }?;

        Some(id.to_string())
    }

    /// Select the row at `index` of the shown, filtered table. Returns
    /// `false` when there is no such row.
    pub(super) fn select_shown_row(&mut self, index: usize) -> bool {
        let Some(id) = self.shown_row_id(index) else {
            return false;
        };
        let Some(place) = self.place.as_ref() else {
            return false;
        };
        let Some(engine) = self
            .engines
            .iter_mut()
            .find(|engine| engine.info.address() == place.engine())
        else {
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

#[cfg(test)]
impl DockerState {
    /// Return the position in the shown table of the item the engine listed
    /// at `engine_index` in the open list, so a test can press the row of a
    /// known item whatever the sort is.
    pub(crate) fn shown_position(&self, engine_index: usize) -> usize {
        let place = self.place().expect("an open place");
        let engine = self.open_engine().expect("an open engine");
        let filter = self.filter.as_str();
        let sort = self.sorts.get(place.resource());

        match place.resource() {
            ResourceKind::Containers => {
                let item = &engine.containers.items()[engine_index];
                shown_containers(
                    engine.containers.items(),
                    filter,
                    place.compose_project(),
                    sort,
                )
                .iter()
                .position(|shown| shown.id() == item.id())
                .expect("the item is shown")
            },
            ResourceKind::Images => {
                let item = &engine.images.items()[engine_index];
                shown_images(engine.images.items(), filter, sort)
                    .iter()
                    .position(|shown| shown.id() == item.id())
                    .expect("the item is shown")
            },
            ResourceKind::Volumes => {
                let item = &engine.volumes.items()[engine_index];
                shown_volumes(engine.volumes.items(), filter, sort)
                    .iter()
                    .position(|shown| shown.name() == item.name())
                    .expect("the item is shown")
            },
            ResourceKind::Networks => {
                let item = &engine.networks.items()[engine_index];
                shown_networks(engine.networks.items(), filter, sort)
                    .iter()
                    .position(|shown| shown.id() == item.id())
                    .expect("the item is shown")
            },
        }
    }

    /// Return the actions the engines are running now.
    pub(crate) fn running(&self) -> &[ActionRequest] {
        &self.running
    }
}

/// The open row menu: where it opened and the row it is for. A right press
/// also selects that row; the row "⋯" button does not.
#[derive(Debug)]
struct RowMenu {
    /// Window coordinates.
    position: Point,
    /// The row's id in the open list: container, image or network id, or
    /// volume name.
    target: String,
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

    /// Take one reloaded list; the other lists stay as they are.
    fn apply_list(&mut self, list: LoadedList) {
        match list {
            LoadedList::Containers(result) => {
                self.containers.apply(result, Container::id);
            },
            LoadedList::Images(result) => self.images.apply(result, Image::id),
            LoadedList::Volumes(result) => {
                self.volumes.apply(result, Volume::name);
            },
            LoadedList::Networks(result) => {
                self.networks.apply(result, Network::id);
            },
        }
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

        self.find(selected, id_of)
    }

    /// Return the loaded item with `id`.
    pub(crate) fn find(&self, id: &str, id_of: fn(&T) -> &str) -> Option<&T> {
        self.items().iter().find(|item| id_of(item) == id)
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

/// The selected row of the open list.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SelectedRow<'a> {
    Container(&'a Container),
    Image(&'a Image),
    Volume(&'a Volume),
    Network(&'a Network),
}

impl SelectedRow<'_> {
    /// Return the list the row is in.
    pub(crate) fn resource(&self) -> ResourceKind {
        match self {
            Self::Container(_) => ResourceKind::Containers,
            Self::Image(_) => ResourceKind::Images,
            Self::Volume(_) => ResourceKind::Volumes,
            Self::Network(_) => ResourceKind::Networks,
        }
    }

    /// Return the row's id: container, image or network id, or volume name.
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Container(container) => container.id(),
            Self::Image(image) => image.id(),
            Self::Volume(volume) => volume.name(),
            Self::Network(network) => network.id(),
        }
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
