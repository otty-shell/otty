# RFC: Refactoring Architecture for OTTY App

## 1. Summary
This RFC proposes a layered architecture for the OTTY application, introducing clear boundaries between App, Screen, Widget, Component, and Service layers. The goal is to improve maintainability, testability, and scalability while preserving current behavior. The document also maps current modules (for example, [`app.rs`](otty/src/app.rs:1), [`tab.rs`](otty/src/tab.rs:1), [`widget/tab_bar.rs`](otty/src/widget/tab_bar.rs:1)) into the proposed structure.

## 2. Motivation
The current app mixes responsibilities across UI, layout, and logic. Tight coupling makes changes risky and slows down feature development. A strict layered architecture will:
- reduce cognitive load,
- localize changes,
- improve test coverage (especially for Service-level logic),
- keep UI components small and reusable.

## 3. Goals / Non-goals
### Goals
- Establish strict layers: App → Screen → Widget → Component; Service remains UI-agnostic.
- Standardize contracts: Props, Events, State, Actions/Commands.
- Make event flow predictable and non-spaghetti.
- Keep current behavior unchanged.
- Provide a safe, incremental migration plan.

### Non-goals
- Refactoring external libraries or crates.
- Changing existing behavior or UX.
- Introducing heavy abstractions that do not serve real usage.

## 4. Proposed Architecture
### 4.1 Layers and Responsibilities
#### App
**Purpose**: Top-level application logic: init, routing/screen switching, window/title, global widgets, theme/fonts/config initialization.

**Allowed**:
- Own global state and route between screens.
- Initialize themes, fonts, configs (see [`theme.rs`](otty/src/theme.rs:1) and [`fonts.rs`](otty/src/fonts.rs:1)).
- Dispatch high-level commands to screens/services.

**Forbidden**:
- Any concrete Screen or Component UI logic.
- Direct manipulation of widget internals.

#### Screen
**Purpose**: A group of widgets that solve one UI task (e.g., Terminal screen).

**Allowed**:
- Manage screen-specific UI state.
- Translate screen events to app-level actions.

**Forbidden**:
- Business logic (must be in Service).
- Direct global state mutation (must bubble actions to App).

#### Widget
**Purpose**: Composable, reusable UI blocks (e.g., tab bar). Can be used across multiple screens.

**Allowed**:
- Compose Components.
- Provide local UI state.

**Forbidden**:
- Business logic and global state access.

#### Component
**Purpose**: Smallest UI entity with a single responsibility.

**Allowed**:
- Emit local UI events (e.g., activate tab, close tab).

**Forbidden**:
- Global state mutation.
- Access to services or other layers directly.

#### Service
**Purpose**: Business logic (terminal sessions, shell integration, data IO, etc.).

**Allowed**:
- Provide explicit APIs used by App only.

**Forbidden**:
- Knowledge of UI or layout.
- Direct usage from Screen/Widget/Component (no UI → Service dependency).

### 4.2 Dependencies
Strict dependency direction:

```
App -> Screen -> Widget -> Component
App -> Service
Screen/Widget/Component -> (no Service access)
```

No upward dependencies are allowed. Screen never calls services directly.

### 4.3 Mapping from Current Modules
| Current Module | Target Layer | Proposed Path |
| --- | --- | --- |
| [`app.rs`](otty/src/app.rs:1) | App | `src/app/mod.rs` + `src/app/state.rs` |
| [`main.rs`](otty/src/main.rs:1) | App bootstrap | `src/main.rs` |
| [`theme.rs`](otty/src/theme.rs:1), [`fonts.rs`](otty/src/fonts.rs:1) | App subsystem | `src/app/theme.rs`, `src/app/fonts.rs` |
| [`tab.rs`](otty/src/tab.rs:1) | Split: Widget view + Screen state | `src/widgets/tab.rs` + `src/screens/terminal/tab_state.rs` |
| [`screen/terminal.rs`](otty/src/screen/terminal.rs:1) | Screen | `src/screens/terminal/mod.rs` |
| [`widget/tab_bar.rs`](otty/src/widget/tab_bar.rs:1) | Widget | `src/widgets/tab_bar.rs` |
| [`component/tab_button.rs`](otty/src/component/tab_button.rs:1) | Component | `src/components/tab_button.rs` |
| [`action_bar.rs`](otty/src/action_bar.rs:1) | Widget | `src/widgets/action_bar.rs` |
| [`context_menu.rs`](otty/src/context_menu.rs:1) | Widget/Component mix | Split into `src/widgets/pane_menu.rs` + `src/components/menu_item.rs` |
| [`service/shell_integrations.rs`](otty/src/service/shell_integrations.rs:1) | Service | `src/services/shell.rs` |

## 5. Folder Structure
Proposed structure:
```
src/
  app/
    mod.rs
    state.rs
    theme.rs
    fonts.rs
    config.rs
  screens/
    mod.rs
    terminal/
      mod.rs
      state.rs
      tab_state.rs
      pane_grid.rs
  widgets/
    mod.rs
    tab.rs
    tab_bar.rs
    action_bar.rs
    pane_context_menu.rs
  components/
    mod.rs
    tab_button.rs
    icon_button.rs
    menu_item.rs
  services/
    mod.rs
    registry.rs
    shell.rs
    terminal_sessions.rs
```

Naming rules:
- File names are snake_case, one public widget/component per file.
- Screen-local structs are contained within screen module folders.
- `*_props`, `*_event`, `*_state`, `*_action`, `*_command` modules define contracts.

## 6. Core Interfaces
### 6.1 Naming Convention
- `*Props`: input data passed from parent.
- `*Event`: events emitted from child to parent.
- `*State`: internal UI state.
- `*Action`: screen-level decisions (semantic intent).
- `*Command`: app-level commands and service invocations.

### 6.2 Contracts by Layer
#### Screen
- **Props**: global config, theme, screen route info.
- **Events**: user intent (activate tab, close tab, open context menu).
- **Traits**: `Screen` interface with `update`, `view`, and `subscription`.
- **Theme**: receives `AppTheme` from App and passes it down to Widgets/Components.
- **Service Access**: no direct access; Screen emits actions only.

#### Widget
- **Props**: immutable view inputs (active tab id, list of titles) + theme.
- **Events**: UI events that bubble to Screen.
- **Traits**: `WidgetView` returning element type.
- **Theme**: accepts theme from Screen and can expose style overrides for local variations.
- **Service Access**: no access.

#### Component
- **Props**: minimal data for rendering + theme.
- **Events**: minimal, atomic UI actions.
- **Traits**: `ComponentView` for rendering.
- **Theme**: consumes theme and optional per-instance style overrides.
- **Service Access**: no access.

### 6.3 Global State Mutation Rule
No component, widget, or screen mutates global state directly. Every mutation must be expressed as `*Event` → `*Action` → `*Command` and handled at App/Service boundary.

### 6.4 Code Examples (Traits, Props, Events, Interaction)
The following examples illustrate the intended contracts and flow. These are illustrative, not prescriptive.

```rust
// Common contracts
pub trait Screen<Message> {
    fn update(&mut self, event: Self::Event) -> Self::Action;
    fn view(&self) -> iced::Element<'_, Self::Event>;
    fn subscription(&self) -> iced::Subscription<Self::Event>;

    type Event;
    type Action;
}

pub trait WidgetView<Event> {
    fn view(&self) -> iced::Element<'_, Event>;
}

pub trait ComponentView<Event> {
    fn view(self) -> iced::Element<'static, Event>;
}

// Theme and style override contracts
#[derive(Clone)]
pub struct StyleOverrides {
    pub background: Option<iced::Color>,
    pub foreground: Option<iced::Color>,
    pub border_radius: Option<f32>,
}

#[derive(Clone)]
pub struct ThemeProps<'a> {
    pub theme: &'a AppTheme,
    pub overrides: Option<StyleOverrides>,
}

// Example Component contract
#[derive(Clone)]
pub struct TabButtonProps<'a> {
    pub id: u64,
    pub title: &'a str,
    pub is_active: bool,
    pub theme: ThemeProps<'a>,
}

#[derive(Debug, Clone)]
pub enum TabButtonEvent {
    ActivateTab(u64),
    CloseTab(u64),
}

// Example Widget contract
#[derive(Clone)]
pub struct TabSummary {
    pub id: u64,
    pub title: String,
}

#[derive(Clone)]
pub struct TabBarProps<'a> {
    pub tabs: &'a [TabSummary],
    pub active_tab_id: u64,
    pub theme: ThemeProps<'a>,
}

#[derive(Debug, Clone)]
pub enum TabBarEvent {
    TabButton(TabButtonEvent),
}

// Example Screen contract
#[derive(Clone)]
pub struct TerminalScreenProps<'a> {
    pub theme: ThemeProps<'a>,
    pub config: &'a AppConfig,
}

#[derive(Debug, Clone)]
pub enum TerminalScreenEvent {
    TabBar(TabBarEvent),
    PaneGrid(PaneGridEvent),
}

#[derive(Debug, Clone)]
pub enum TerminalScreenAction {
    ActivateTab(u64),
    CloseTab(u64),
    SplitPane { tab_id: u64, axis: pane_grid::Axis },
}

// Example App-level command
#[derive(Debug, Clone)]
pub enum AppCommand {
    ActivateTab(u64),
    CloseTab(u64),
    CreateShell { tab_id: u64 },
}

// Example interaction mapping
fn map_terminal_event(event: TerminalScreenEvent) -> Option<AppCommand> {
    match event {
        TerminalScreenEvent::TabBar(TabBarEvent::TabButton(TabButtonEvent::ActivateTab(id))) => {
            Some(AppCommand::ActivateTab(id))
        }
        TerminalScreenEvent::TabBar(TabBarEvent::TabButton(TabButtonEvent::CloseTab(id))) => {
            Some(AppCommand::CloseTab(id))
        }
        _ => None,
    }
}
```

## 7. Event Flow
### 7.1 Event Routing Diagram (ASCII)
```
ComponentEvent -> WidgetEvent -> ScreenEvent -> AppEvent
       |              |              |          |
       v              v              v          v
   local UI       UI grouping     screen logic  service commands
```

### 7.2 Example Event Flow
**Activate Tab**
1. Component emits `TabButtonEvent::ActivateTab`.
2. Widget maps to `TabBarEvent::TabButton(TabButtonEvent::ActivateTab)`.
3. Screen maps to `TerminalScreenEvent::TabBar(...)`.
4. App handles `AppCommand::ActivateTab` and updates state.

**Close Tab**
1. Component emits `TabButtonEvent::CloseTab`.
2. Widget maps to `TabBarEvent::TabButton(TabButtonEvent::CloseTab)`.
3. Screen maps to `TerminalScreenEvent::TabBar(...)`.
4. App updates tab collection and issues commands if needed.

**Create Shell**
1. Screen emits `TerminalScreenAction::CreateShell`.
2. App converts to `AppCommand::CreateShell`.
3. Service executes shell integration API.

### 7.3 Avoiding Event Spaghetti
- Each layer translates events into its own enum and only bubbles upward.
- App handles only `AppEvent`; it never sees Component or Widget events.
- Use `*Action` to encode intent and separate UI semantics from execution.

## 8. Service Layer
### 8.1 Service Responsibilities, State, and System Interaction
Services are long-lived, App-owned objects that encapsulate system-facing IO and business rules. They do not render UI. They expose a small synchronous API and hold their own internal state as needed.

**State ownership**:
- Service state lives inside the service instance (e.g., active shell sessions, PTY handles, terminal registry).
- App holds references to services and owns the lifecycle (init, shutdown).
- UI state never lives in services; UI only consumes outputs via App events.

**System interaction**:
- OS/PTY/shell integration is performed inside services only.
- Services do not emit events; App pulls results from synchronous service calls.
- Any external crate usage remains unchanged; only the app-layer wiring changes.

### 8.2 Example Service API (Illustrative)
```rust
pub trait ShellService {
    fn setup_session(&mut self) -> Result<ShellSession, ShellError>;
    fn create_terminal(&mut self, settings: TerminalSettings) -> Result<TerminalId, ShellError>;
    fn close_terminal(&mut self, terminal_id: TerminalId);
}

pub struct ShellServiceImpl {
    sessions: HashMap<TerminalId, ShellSession>,
}

impl ShellService for ShellServiceImpl {
    fn setup_session(&mut self) -> Result<ShellSession, ShellError> {
        // Calls into existing shell integration
        // Example mapping: [`shell_integrations.rs`](otty/src/service/shell_integrations.rs:1)
        todo!()
    }

    fn create_terminal(&mut self, settings: TerminalSettings) -> Result<TerminalId, ShellError> {
        todo!()
    }

    fn close_terminal(&mut self, terminal_id: TerminalId) {
        self.sessions.remove(&terminal_id);
    }
}
```

### 8.3 Service Event Integration
- Services are synchronous APIs and do not emit events.
- App calls services and updates state directly; the UI only responds to App state updates.
- If future eventing is needed, introduce a dedicated event source entity rather than extending services.

### 8.4 Testing
- Unit tests for each service (pure logic, no UI).
- Integration tests around terminal session lifecycle.
- Mock Service implementations for App tests to validate command → service calls.

## 9. Migration Plan
1. **Introduce folder skeleton** with `app/`, `screens/`, `widgets/`, `components/`, `services/` while keeping existing modules intact.
2. **Move components first** (e.g., [`component/tab_button.rs`](otty/src/component/tab_button.rs:1) → `components/`).
3. **Move widgets next** (e.g., [`widget/tab_bar.rs`](otty/src/widget/tab_bar.rs:1)).
4. **Split tab**: move view logic into `widgets/tab.rs` and keep terminal-specific state in `screens/terminal/tab_state.rs`.
5. **Refactor [`app.rs`](otty/src/app.rs:1)** to depend on screens only and convert existing events to `AppEvent`/`AppCommand`.
6. **Move services** to `services/` and add tests.
7. **Cleanup and rename** modules, remove deprecated paths.

No big-bang changes: each step should compile and keep behavior unchanged.

## 10. Risks & Mitigations
- **Risk**: Event mapping introduces regressions.
  - *Mitigation*: Maintain old event variants until mapping is complete; add tests for key flows.
- **Risk**: Refactoring UI layout causes visual drift.
  - *Mitigation*: Keep rendering logic identical; avoid style changes.
- **Risk**: Service abstraction adds overhead.
  - *Mitigation*: Keep services thin and explicit; avoid dynamic dispatch unless needed.

## 11. Open Questions (Resolved)
- Screen traits are generic over message type.
- Shared global UI state (theme, fonts) is owned by App and passed down into all UI components via props.
- Introduce a `ServiceRegistry` owned by App to manage service instances.

---

Appendix: This RFC maps existing modules to the target layers and is designed to preserve current behavior while improving maintainability.
