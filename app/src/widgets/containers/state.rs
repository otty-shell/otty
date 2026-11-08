use std::collections::HashSet;
use std::path::{Path, PathBuf};

use otty_containers::{
    Container, Diagnosis, Discovery, EngineAddress, EngineInfo, Image, Network,
    Volume,
};
use otty_ui_kit::components::tag_search::{
    TagSchema, TagSearch, TagSearchAction,
};

use super::actions::{ActionsCtx, ActionsState};
use super::create::CreateState;
use super::inspector::{InspectorCtx, InspectorState};
use super::resource_rows::port_urls;
use super::run::{RunCtx, RunEngine, RunState};
use super::sort::{TableSort, TableSorts};
use super::table_search::{
    COMPOSE_KEY, TableSearches, container_row, containers_schema, image_row,
    images_schema, network_row, networks_schema, volume_row, volumes_schema,
};
use super::types::{
    ContainersError, InspectorTab, LoadedList, LoadedLists, OpenList, Place,
    ResourceKind, shown_containers, shown_images, shown_networks,
    shown_volumes,
};

/// Runtime state of Containers mode: the engines and their lists, the open
/// place, the selected row of each list, the tag search and sort of each page,
/// the notice of the last failure and the parts: the actions, the create
/// dialogs and the Run form.
#[derive(Debug, Default)]
pub(super) struct ContainersState {
    is_active: bool,
    /// The right panel, where the inspector lives, is off the screen: closed,
    /// or hidden by focus mode. It starts shown, like the sidebar's.
    is_right_panel_hidden: bool,
    search: Search,
    /// A search behind the no-engine card is on its way. It leaves `search` as
    /// it is, so the card stays on screen with its buttons.
    is_background_search_running: bool,
    /// Whether SELinux is enabled, from the last finished search. Kept apart
    /// from `search` because a new search forgets its result while engines
    /// found before stay shown and can open a Run form.
    is_selinux_enabled: bool,
    /// The home folder, from the last finished search; kept apart from `search`
    /// for the same reason.
    home_folder: Option<PathBuf>,
    engines: Vec<EngineState>,
    /// Goes up whenever a list of any engine may have changed; what is built
    /// from the lists compares it to know when to build again.
    lists_version: u64,
    /// The list the main area shows. Its engine may be closed in the tree:
    /// closing an engine does not change what the main area shows.
    place: Option<Place>,
    /// The engines closed in the tree. Every engine is open by default,
    /// also one found later, so the closed ones are kept. Lives as long as
    /// the app runs; not saved to disk.
    closed_engines: HashSet<EngineAddress>,
    /// See [`Self::age_clock`].
    age_clock: i64,
    /// The kept sort of each page; not saved to disk.
    sorts: TableSorts,
    /// The kept tag search of each page; not saved to disk.
    searches: TableSearches,
    /// The open inspector tab. It is kept here and not in the inspector,
    /// because only opening a list goes back to Overview: a place that moves
    /// on by itself, when its engine goes away, keeps the tab.
    inspector_tab: InspectorTab,
    inspector: InspectorState,
    /// The actions: the ones that run, the open confirmation and the row
    /// menu.
    actions: ActionsState,
    notice: Option<String>,
    /// The create dialogs: Pull image, Create volume and Create network.
    create: CreateState,
    /// The Run container form; it replaces the table while it is open.
    run: RunState,
    /// What the main area showed when the last engine went away; it comes back
    /// with that engine. Not saved to disk.
    lost_view: Option<LostView>,
}

impl ContainersState {
    /// Return whether Containers mode is the active mode.
    pub(crate) fn is_active(&self) -> bool {
        self.is_active
    }

    /// Return whether the right panel is off the screen.
    pub(crate) fn is_right_panel_hidden(&self) -> bool {
        self.is_right_panel_hidden
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

    /// Return whether the tree hides the lists of the engine at `address`.
    pub(crate) fn is_engine_closed(&self, address: &EngineAddress) -> bool {
        self.closed_engines.contains(address)
    }

    /// Return how the table of the page `place` is sorted.
    pub(crate) fn sort(&self, place: &Place) -> TableSort {
        self.sorts.get(place)
    }

    /// Return the tag search of the table of the page `place`.
    pub(crate) fn table_search(&self, place: &Place) -> &TagSearch {
        self.searches.get(place)
    }

    /// Return the time (Unix seconds) the tag search counts ages with. It
    /// moves only between intents: the rows the view filtered and the rows a
    /// press is looked up in must be the same, or a press on a row could reach
    /// another row when an `age` tag is on.
    pub(crate) fn age_clock(&self) -> i64 {
        self.age_clock
    }

    /// Return what the tag search of the open table knows; `None` with no
    /// open table.
    pub(crate) fn open_table_schema(&self) -> Option<TagSchema> {
        let place = self.place.as_ref()?;
        let engine = self.open_engine()?;
        let now = self.age_clock;
        let sort = self.sorts.get(place);

        let schema = match place.resource() {
            ResourceKind::Containers => {
                containers_schema(engine.containers.items(), now, sort)
            },
            ResourceKind::Images => {
                images_schema(engine.images.items(), now, sort)
            },
            ResourceKind::Volumes => {
                volumes_schema(engine.volumes.items(), now, sort)
            },
            ResourceKind::Networks => {
                networks_schema(engine.networks.items(), now, sort)
            },
        };

        Some(schema)
    }

    /// Return the state of the inspector tabs.
    pub(crate) fn inspector(&self) -> &InspectorState {
        &self.inspector
    }

    /// Return what the inspector reads from this state, for its view models.
    pub(crate) fn inspector_ctx(&self) -> InspectorCtx<'_> {
        inspector_ctx_of(
            self.is_inspector_on_screen(),
            self.inspector_tab,
            self.lists_version,
            self.place.as_ref(),
            &self.engines,
        )
    }

    /// Return whether the inspector is on screen, as far as the mode, the right
    /// panel and the Run form go: the form shows the image panel in its place.
    fn is_inspector_on_screen(&self) -> bool {
        self.is_active && !self.is_right_panel_hidden && !self.run.is_open()
    }

    /// Return the id of the selected row of the open list; an image row goes
    /// by its row key.
    pub(crate) fn selected_id(&self) -> Option<&str> {
        open_list_of(self.place.as_ref(), &self.engines)?.selected_id
    }

    /// Return the notice of the last failed action.
    pub(crate) fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Return the state of the create dialogs.
    pub(crate) fn create(&self) -> &CreateState {
        &self.create
    }

    /// Return the state of the Run container form.
    pub(crate) fn run(&self) -> &RunState {
        &self.run
    }

    /// Return what the Run container form reads from this state, for its
    /// view models.
    pub(crate) fn run_ctx(&self) -> RunCtx<'_> {
        run_ctx_of(
            &self.engines,
            self.is_selinux_enabled,
            self.home_folder.as_deref(),
        )
    }

    /// Return the state of the actions.
    pub(crate) fn actions(&self) -> &ActionsState {
        &self.actions
    }

    /// Return what the actions read from this state, for their view models.
    pub(crate) fn actions_ctx(&self) -> ActionsCtx<'_> {
        actions_ctx_of(self.place.as_ref(), &self.engines, &self.search)
    }

    /// Return whether the no-engine card is what the main area shows: the last
    /// search finished without an engine.
    pub(crate) fn shows_no_engine_card(&self) -> bool {
        let Search::Done(discovery) = &self.search else {
            return false;
        };

        self.open_engine().is_none()
            && !matches!(discovery.diagnosis(), Diagnosis::Engines(_))
    }

    /// Return whether a search behind the no-engine card is on its way.
    pub(crate) fn is_background_search_running(&self) -> bool {
        self.is_background_search_running
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

    /// Open the inspector `tab` when the open list has it (only containers
    /// have Logs).
    pub(super) fn select_inspector_tab(&mut self, tab: InspectorTab) {
        let Some(place) = self.place.as_ref() else {
            return;
        };
        if !InspectorTab::of(place.resource()).contains(&tab) {
            return;
        }

        self.inspector_tab = tab;
    }

    /// Mark Containers mode as active or not.
    pub(super) fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
    }

    /// Note whether the right panel is off the screen.
    pub(super) fn set_right_panel_hidden(&mut self, is_hidden: bool) {
        self.is_right_panel_hidden = is_hidden;
    }

    /// Start a new engine search. Engines found before stay shown until it
    /// finishes.
    pub(super) fn start_search(&mut self) {
        self.search = Search::Searching;
    }

    /// Remember whether a search behind the no-engine card is on its way.
    pub(super) fn set_background_search_running(&mut self, is_running: bool) {
        self.is_background_search_running = is_running;
    }

    /// Take the search result: keep the lists of engines found again, add
    /// new engines, drop engines that are gone. The open place stays when
    /// its engine is still there; see [`Self::leave_lost_place`] for when it
    /// is not. Each page keeps its search and sort. An engine the user closed
    /// stays closed, also when it comes back while the no-engine page shows:
    /// the table shows its list and its tree row stays closed. Returns `true`
    /// when the open place was lost with its engine.
    pub(super) fn finish_search(&mut self, discovery: Discovery) -> bool {
        let reached = match discovery.diagnosis() {
            Diagnosis::Engines(infos) => infos.clone(),
            _ => Vec::new(),
        };
        let shown_row = self.selected_id().map(str::to_string);

        self.lists_version += 1;
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
        let is_place_lost = self.place.is_some() && !place_is_kept;
        if !place_is_kept {
            self.leave_lost_place(shown_row);
        }

        self.is_selinux_enabled = discovery.is_selinux_enabled();
        self.home_folder = discovery.home_folder().map(Path::to_path_buf);
        self.search = Search::Done(discovery);

        is_place_lost
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
        self.lists_version += 1;
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
        self.lists_version += 1;
        true
    }

    /// Show `place`. Its engine opens in the tree, so a link or a finished
    /// Run that lands on a closed engine shows the list it went to. Opening
    /// another place keeps the search and the sort of each page, closes the
    /// suggestions of the page left and goes back to the Overview tab. It also
    /// wins over a page waiting to come back with its engine.
    pub(super) fn open(&mut self, place: Place) {
        self.closed_engines.remove(place.engine());
        self.lost_view = None;

        if self.place.as_ref() == Some(&place) {
            return;
        }

        if let Some(left) = self.place.as_ref() {
            self.searches.get_mut(left).close_suggestions();
        }
        self.place = Some(place);
        self.inspector_tab = InspectorTab::Overview;
        self.notice = None;
    }

    /// Close the engine at `address` in the tree, or open it when it is
    /// closed. The main area keeps showing what it shows.
    pub(super) fn toggle_engine(&mut self, address: EngineAddress) {
        if !self.closed_engines.remove(&address) {
            self.closed_engines.insert(address);
        }
    }

    /// Show `notice` above the table until it is dismissed.
    pub(super) fn set_notice(&mut self, notice: String) {
        self.notice = Some(notice);
    }

    /// Hide the notice.
    pub(super) fn dismiss_notice(&mut self) {
        self.notice = None;
    }

    /// Return the state of the create dialogs for their reducer.
    pub(super) fn create_mut(&mut self) -> &mut CreateState {
        &mut self.create
    }

    /// Return the state of the Run container form for its reducer, with
    /// what the form reads from this state.
    pub(super) fn run_and_ctx(&mut self) -> (&mut RunState, RunCtx<'_>) {
        let ctx = run_ctx_of(
            &self.engines,
            self.is_selinux_enabled,
            self.home_folder.as_deref(),
        );

        (&mut self.run, ctx)
    }

    /// Return the state of the actions for their reducer, with what the
    /// actions read from this state.
    pub(super) fn actions_and_ctx(
        &mut self,
    ) -> (&mut ActionsState, ActionsCtx<'_>) {
        let ctx =
            actions_ctx_of(self.place.as_ref(), &self.engines, &self.search);

        (&mut self.actions, ctx)
    }

    /// Return the state of the inspector for its reducer, with what the
    /// inspector reads from this state.
    pub(super) fn inspector_and_ctx(
        &mut self,
    ) -> (&mut InspectorState, InspectorCtx<'_>) {
        let ctx = inspector_ctx_of(
            self.is_inspector_on_screen(),
            self.inspector_tab,
            self.lists_version,
            self.place.as_ref(),
            &self.engines,
        );

        (&mut self.inspector, ctx)
    }

    /// Show the `resource` list of `engine` and select the item with `id` in
    /// it: a container or network id, an image row key, or a volume name. The
    /// text in the field of that page goes away, and so do its tags when they
    /// would hide the item. A new container is selected before the reload that
    /// brings it into the list, so a tag cannot be checked against it; the tags
    /// go away then too.
    pub(super) fn show_item(
        &mut self,
        engine: &EngineAddress,
        resource: ResourceKind,
        id: String,
    ) {
        let place = Place::new(engine.clone(), resource);
        self.open(place.clone());

        let passes = self.item_passes_tags(&place, &id);
        let search = self.searches.get_mut(&place);
        search.clear_input();
        if !passes {
            let _ = search.clear_tags();
        }

        if let Some(engine) = self
            .engines
            .iter_mut()
            .find(|state| state.info.address() == engine)
        {
            engine.select(resource, id);
        }
    }

    /// Show the containers of the open engine with the tag
    /// `compose:<project>`, replacing another `compose` tag on that page only.
    /// The text in the field goes away, so the project shows.
    pub(super) fn open_compose_project(&mut self, project: &str) {
        let Some(place) = self.place.as_ref() else {
            return;
        };

        let containers =
            Place::new(place.engine().clone(), ResourceKind::Containers);
        self.open(containers.clone());

        let search = self.searches.get_mut(&containers);
        search.clear_input();
        let _ = search.set_tag(COMPOSE_KEY, project);
    }

    /// Open the Compose project of the container at `index` of the shown,
    /// filtered containers table. Returns `false` when there is no such row
    /// or it has no project.
    pub(super) fn open_compose_project_of_row(&mut self, index: usize) -> bool {
        let Some(project) = self
            .shown_container(index)
            .and_then(|container| container.compose_project())
            .map(str::to_string)
        else {
            return false;
        };

        self.open_compose_project(&project);
        true
    }

    /// The page opened by the port link at position `link` among the links
    /// of the container at `index` of the shown, filtered containers table.
    /// `None` when there is no such row or link.
    pub(super) fn port_url_of_row(
        &self,
        index: usize,
        link: usize,
    ) -> Option<String> {
        let container = self.shown_container(index)?;

        port_urls(container.ports()).into_iter().nth(link)
    }

    /// Sort the open table by the column at `column`, or turn its order
    /// around when it is sorted by that column already. The selected item
    /// stays selected: a selection is the item's id, not a position.
    pub(super) fn sort_by(&mut self, column: usize) {
        let Some(place) = self.place.as_ref() else {
            return;
        };

        self.sorts.press(place, column);
    }

    /// Do what the user did in the tag search of the open table.
    pub(super) fn perform_search(&mut self, action: TagSearchAction) {
        let Some(schema) = self.open_table_schema() else {
            return;
        };
        let Some(place) = self.place.as_ref() else {
            return;
        };

        let _ = self.searches.get_mut(place).perform(action, &schema);
    }

    /// Close the suggestions of the open table's search; its text and tags
    /// stay. No other page can have them open: going to another page closes
    /// them.
    pub(super) fn close_search_suggestions(&mut self) {
        if let Some(place) = self.place.as_ref() {
            self.searches.get_mut(place).close_suggestions();
        }
    }

    /// Set the time the tag search counts ages with; see [`Self::age_clock`].
    pub(super) fn set_age_clock(&mut self, now: i64) {
        self.age_clock = now;
    }

    /// Return the id of the row at `index` of the shown, filtered table;
    /// `None` when there is no such row.
    pub(super) fn shown_row_id(&self, index: usize) -> Option<String> {
        let place = self.place.as_ref()?;
        let engine = self.open_engine()?;
        let search = self.searches.get(place);
        let now = self.age_clock;
        let sort = self.sorts.get(place);

        let id = match place.resource() {
            ResourceKind::Containers => {
                shown_containers(engine.containers.items(), search, now, sort)
                    .get(index)
                    .map(|container| container.id())
            },
            ResourceKind::Images => {
                shown_images(engine.images.items(), search, now, sort)
                    .get(index)
                    .map(|image| image.row_key())
            },
            ResourceKind::Volumes => {
                shown_volumes(engine.volumes.items(), search, now, sort)
                    .get(index)
                    .map(|volume| volume.name())
            },
            ResourceKind::Networks => {
                shown_networks(engine.networks.items(), search, now, sort)
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

        engine.select(place.resource(), id);
        true
    }

    /// The engine of the open place is gone, so the notice of the old list
    /// goes away and its suggestions close; the parent closes the row menu.
    ///
    /// When no engine is left, the place and `shown_row` wait in
    /// [`Self::lost_view`] for their engine: they come back with it, the row as
    /// the selection its new list starts with, so a row gone meanwhile falls
    /// back as any missing selection does. When another engine answers without
    /// that one, the view moves on to the first engine's containers and does
    /// not jump back later. Only a place that moves from a lost engine to
    /// another engine opens that engine in the tree, so the user can see whose
    /// list is on screen.
    fn leave_lost_place(&mut self, shown_row: Option<String>) {
        let left = self.place.take();
        self.notice = None;
        if let Some(left) = &left {
            self.searches.get_mut(left).close_suggestions();
        }

        if self.engines.is_empty() {
            if let Some(place) = left {
                self.lost_view = Some(LostView {
                    place,
                    row: shown_row,
                });
            }
            return;
        }

        let lost_view = self.lost_view.take();
        if let Some(view) = lost_view
            && let Some(engine) = self
                .engines
                .iter_mut()
                .find(|engine| engine.info.address() == view.place.engine())
        {
            if let Some(row) = view.row {
                engine.select(view.place.resource(), row);
            }
            self.place = Some(view.place);
            return;
        }

        self.place = self.engines.first().map(|engine| {
            Place::new(engine.info.address().clone(), ResourceKind::Containers)
        });
        if left.is_some()
            && let Some(place) = &self.place
        {
            self.closed_engines.remove(place.engine());
        }
    }

    /// Return the container at `index` of the shown, filtered containers
    /// table; `None` when another list is open or there is no such row.
    fn shown_container(&self, index: usize) -> Option<&Container> {
        let (place, engine) = (self.place.as_ref()?, self.open_engine()?);
        if place.resource() != ResourceKind::Containers {
            return None;
        }

        let shown = shown_containers(
            engine.containers.items(),
            self.searches.get(place),
            self.age_clock,
            self.sorts.get(place),
        );

        shown.get(index).copied()
    }

    /// Return whether the tags of the page `place` let the item with `id`
    /// through; `false` when it is not listed.
    fn item_passes_tags(&self, place: &Place, id: &str) -> bool {
        let Some(engine) = self.engine(place.engine()) else {
            return false;
        };
        let now = self.age_clock;

        let row = match place.resource() {
            ResourceKind::Containers => engine
                .containers
                .find(id, Container::id)
                .map(|container| container_row(container, now)),
            ResourceKind::Images => engine
                .images
                .find(id, Image::row_key)
                .map(|image| image_row(image, now)),
            ResourceKind::Volumes => engine
                .volumes
                .find(id, Volume::name)
                .map(|volume| volume_row(volume, now)),
            ResourceKind::Networks => engine
                .networks
                .find(id, Network::id)
                .map(|network| network_row(network, now)),
        };

        row.is_some_and(|row| self.searches.get(place).passes_tags(&row))
    }
}

#[cfg(test)]
impl ContainersState {
    /// Return the open inspector tab.
    pub(crate) fn inspector_tab(&self) -> InspectorTab {
        self.inspector_tab
    }

    /// Return the selected row of the open list; `None` when nothing is
    /// selected or the row is not in the loaded list.
    pub(crate) fn selected_row(&self) -> Option<super::types::ListRow<'_>> {
        open_list_of(self.place.as_ref(), &self.engines)?.selected_row()
    }

    /// Return the position in the shown table of the item the engine listed
    /// at `engine_index` in the open list, so a test can press the row of a
    /// known item whatever the sort is.
    pub(crate) fn shown_position(&self, engine_index: usize) -> usize {
        let place = self.place().expect("an open place");
        let engine = self.open_engine().expect("an open engine");
        let search = self.searches.get(place);
        let now = self.age_clock;
        let sort = self.sorts.get(place);

        match place.resource() {
            ResourceKind::Containers => {
                let item = &engine.containers.items()[engine_index];
                shown_containers(engine.containers.items(), search, now, sort)
                    .iter()
                    .position(|shown| shown.id() == item.id())
                    .expect("the item is shown")
            },
            ResourceKind::Images => {
                let item = &engine.images.items()[engine_index];
                shown_images(engine.images.items(), search, now, sort)
                    .iter()
                    .position(|shown| shown.row_key() == item.row_key())
                    .expect("the item is shown")
            },
            ResourceKind::Volumes => {
                let item = &engine.volumes.items()[engine_index];
                shown_volumes(engine.volumes.items(), search, now, sort)
                    .iter()
                    .position(|shown| shown.name() == item.name())
                    .expect("the item is shown")
            },
            ResourceKind::Networks => {
                let item = &engine.networks.items()[engine_index];
                shown_networks(engine.networks.items(), search, now, sort)
                    .iter()
                    .position(|shown| shown.id() == item.id())
                    .expect("the item is shown")
            },
        }
    }
}

/// What the main area showed when the last engine went away.
#[derive(Debug)]
struct LostView {
    place: Place,
    /// The selected row's id in the list of `place`; an image row goes by its
    /// row key.
    row: Option<String>,
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

    /// Select the row with `id` in the `resource` list.
    fn select(&mut self, resource: ResourceKind, id: String) {
        match resource {
            ResourceKind::Containers => self.containers.select(id),
            ResourceKind::Images => self.images.select(id),
            ResourceKind::Volumes => self.volumes.select(id),
            ResourceKind::Networks => self.networks.select(id),
        }
    }

    /// Take each list of one load on its own.
    fn apply_lists(&mut self, lists: LoadedLists) {
        self.containers.apply(lists.containers, Container::id);
        self.images.apply(lists.images, Image::row_key);
        self.volumes.apply(lists.volumes, Volume::name);
        self.networks.apply(lists.networks, Network::id);
    }

    /// Take one reloaded list; the other lists stay as they are.
    fn apply_list(&mut self, list: LoadedList) {
        match list {
            LoadedList::Containers(result) => {
                self.containers.apply(result, Container::id);
            },
            LoadedList::Images(result) => {
                self.images.apply(result, Image::row_key);
            },
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
    fn apply(
        &mut self,
        result: Result<Vec<T>, ContainersError>,
        id_of: fn(&T) -> &str,
    ) {
        let items = match result {
            Ok(items) => items,
            Err(err) => {
                self.load = ListLoad::Failed(err.to_string());
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

/// What the Run container form reads from the state: every reached engine
/// with its networks, and the facts of the last finished search.
fn run_ctx_of<'a>(
    engines: &'a [EngineState],
    is_selinux_enabled: bool,
    home_folder: Option<&'a Path>,
) -> RunCtx<'a> {
    let engines = engines
        .iter()
        .map(|engine| RunEngine {
            info: &engine.info,
            networks: engine.networks.items(),
        })
        .collect();

    RunCtx {
        engines,
        is_selinux_enabled,
        home_folder,
    }
}

/// The list the main area shows, lent out of the state: the open place's
/// engine with its lists; `None` without an open place.
fn open_list_of<'a>(
    place: Option<&Place>,
    engines: &'a [EngineState],
) -> Option<OpenList<'a>> {
    let place = place?;
    let engine = engines
        .iter()
        .find(|engine| engine.info.address() == place.engine())?;

    let selected_id = match place.resource() {
        ResourceKind::Containers => engine.containers.selected_id(),
        ResourceKind::Images => engine.images.selected_id(),
        ResourceKind::Volumes => engine.volumes.selected_id(),
        ResourceKind::Networks => engine.networks.selected_id(),
    };

    Some(OpenList {
        engine: &engine.info,
        resource: place.resource(),
        containers: engine.containers.items(),
        images: engine.images.items(),
        volumes: engine.volumes.items(),
        networks: engine.networks.items(),
        selected_id,
    })
}

/// What the inspector reads from the state: whether it is on screen, the open
/// tab, the version of the lists and the open list.
fn inspector_ctx_of<'a>(
    is_on_screen: bool,
    tab: InspectorTab,
    lists_version: u64,
    place: Option<&Place>,
    engines: &'a [EngineState],
) -> InspectorCtx<'a> {
    InspectorCtx {
        is_on_screen,
        tab,
        lists_version,
        open: open_list_of(place, engines),
    }
}

/// What the actions read from the state: the open list and the command line
/// tools of the last finished search.
fn actions_ctx_of<'a>(
    place: Option<&Place>,
    engines: &'a [EngineState],
    search: &'a Search,
) -> ActionsCtx<'a> {
    let cli = match search {
        Search::Done(discovery) => Some(discovery.cli()),
        Search::Searching => None,
    };

    ActionsCtx {
        open: open_list_of(place, engines),
        cli,
    }
}

#[cfg(test)]
mod tests {
    use otty_containers::{EngineKind, fixtures};

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

        list.apply(Err(ContainersError::new("engine gone")), |item| item);

        assert_eq!(list.load(), &ListLoad::Failed(String::from("engine gone")));
        assert!(list.items().is_empty());
        assert_eq!(list.selected(|item| item), None);
    }

    #[test]
    fn a_new_state_searches_and_shows_nothing() {
        let state = ContainersState::default();

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

    #[test]
    fn finish_search_says_whether_the_open_place_was_lost() {
        let both =
            || fixtures::discovery(&[EngineKind::Docker, EngineKind::Podman]);
        let docker = fixtures::engine(EngineKind::Docker);
        let mut state = ContainersState::default();

        let first = state.finish_search(both());
        state.open(Place::new(docker.address().clone(), ResourceKind::Images));
        let kept = state.finish_search(both());
        let lost =
            state.finish_search(fixtures::discovery(&[EngineKind::Podman]));
        let moved_on =
            state.finish_search(fixtures::discovery(&[EngineKind::Podman]));
        let last_gone = state.finish_search(fixtures::discovery(&[]));
        let none_left = state.finish_search(fixtures::discovery(&[]));

        assert!(!first);
        assert!(!kept);
        assert!(lost);
        assert!(!moved_on);
        assert!(last_gone);
        assert!(!none_left);
    }
}
