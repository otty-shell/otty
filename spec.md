# RFC: Stateless Screens, Global State, Generic Tabs

## Статус

Draft.

## Коротко

Цель этого RFC — сделать экраны (screens) полностью stateless: они должны
только рисовать и отправлять события наверх, а все состояние и логика
живут в глобальном состоянии приложения и редьюсерах. Также вкладки должны
стать универсальными контейнерами, которые могут содержать не только
терминал, но и любой другой контент в будущем.

## Проблемы сегодня (почему нужно менять)

- `TerminalScreen` содержит слишком много логики и состояния: табы,
  панели, контекстные меню, селекцию, клейпборд, обработку событий
  терминала. Это делает экран "толстым" и плохо масштабируемым.
- Таб сейчас жестко привязан к терминалу. Добавить другую вкладку
  (например, настройки) сложно.
- Эффекты (PTY, clipboard, файловые операции) мешаются с обновлением
  состояния — тяжелее тестировать и поддерживать.

## Цели

- Экраны stateless: только view + mapping событий.
- Все данные — в едином `AppState`/`WorkspaceState`.
- Вкладка — универсальный контейнер (терминал, настройки, что угодно).
- Эффекты вынесены, логика становится тестируемой.

## Не-цели

- Не меняем архитектуру движка терминала.
- Не переписываем UI.
- Не добавляем новые фичи, кроме архитектурных изменений.

## Термины

- **Screen** — UI слой, который рисует данные и генерирует события.
- **Reducer** — функция, которая принимает событие и мутирует состояние.
- **Effect** — побочный эффект (I/O, clipboard, PTY, window API).
- **Tab** — контейнер, который хранит `TabContent`.
- **TabContent** — содержимое вкладки: терминал, настройки, текст, и т.д.

## Предлагаемая архитектура

### 1) Глобальный state и редьюсеры

Все данные, которые раньше жили в `TerminalScreen`/`TabState`, переезжают
в глобальное состояние.

Пример структур:

```rust
pub type TabId = u64;
pub type TerminalId = u64;

pub struct AppState {
    pub workspace: WorkspaceState,
    pub theme: ThemeManager,
    pub fonts: FontsConfig,
    pub config: AppConfig,
}

pub struct WorkspaceState {
    pub tabs: Vec<TabId>,
    pub active_tab_id: Option<TabId>,
    pub tab_items: HashMap<TabId, TabItem>,
    pub next_tab_id: TabId,
    pub next_terminal_id: TerminalId,
    pub window_size: Size,
    pub screen_size: Size,
}
```

Примечание: на этом этапе `window_size` и `screen_size` остаются в
`WorkspaceState`, чтобы не дробить структуру слишком рано. Если со временем
`WorkspaceState` начнет разрастаться, эти поля можно вынести в отдельные
`WindowState`/`ScreenState` без изменения внешнего API событий.

События:

```rust
#[derive(Debug, Clone)]
pub enum AppEvent {
    Window(iced::window::Event),
    Tab(TabEvent),
    Terminal(TerminalEvent),
}
```

Пример `WorkspaceEvent`:

```rust
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    NewTab { kind: TabKind }
}

#[derive(Debug, Clone, Copy)]
pub enum TabKind {
    Terminal,
    Settings,
}
```

Тогда событие выглядит так (вызов напрямую из `App`):

```rust
workspace_reducer(&mut state.workspace, &state.config, WorkspaceEvent::NewTab {
    kind: TabKind::Terminal,
});
```

А события, привязанные к конкретной вкладке, логичнее держать в `TabEvent`:

```rust
#[derive(Debug, Clone)]
pub enum TabEvent {
    CloseTab { tab_id: TabId },
    ActivateTab { tab_id: TabId },
}
```

Обновление состояния через редьюсер:

```rust
pub fn update(state: &mut AppState, event: AppEvent) -> Task<AppEvent> {
    match event {
        AppEvent::Tab(tab_event) => {
            tab_reducer(&mut state.workspace, &state.config, tab_event)
        }
        AppEvent::Terminal(term_event) => {
            terminal_reducer(&mut state.workspace, &state.config, term_event)
        }
        AppEvent::Window(win_event) => window_reducer(state, win_event),
    }
}
```

Примечание по именам:
- `tab_reducer` — реагирует на `TabEvent` и меняет только `WorkspaceState`.
- `workspace_reducer` — обрабатывает высокоуровневые события
  (`WorkspaceEvent`: создать таб, закрыть таб, сменить активный таб),
  и может трогать `AppState`, если нужен доступ к `config/services`.
- Если разница между ними не нужна, можно оставить только
  `workspace_reducer` и не вводить второй редьюсер.

Идея: редьюсер меняет только данные и возвращает `Task` (или список
`Effect`), которые потом преобразуются в реальный I/O.

### 2) Экраны становятся stateless

Экран — это просто функция `view`:

```rust
pub fn terminal_screen_view<'a>(
    state: &'a WorkspaceState,
    theme: ThemeProps<'a>,
) -> Element<'a, AppEvent> {
    let tabs = build_tab_summaries(state);
    let active = active_tab(state);

    let content: Element<'a, AppEvent> = match active {
        Some(TabItem { content: TabContent::Terminal(term), .. }) => {
            terminal_tab_view(term, theme).map(AppEvent::Tab)
        }
        Some(TabItem { content: TabContent::Settings(settings), .. }) => {
            settings_view(settings, theme).map(AppEvent::Tab)
        }
        None => empty_view(theme),
    };

    column![tab_bar_view(&tabs).map(AppEvent::Tab), content]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
```

В экране нет собственного состояния. Он читает глобальный `WorkspaceState`
и генерирует события наверх.

### 3) Вкладки становятся универсальными

`TabItem` хранит содержимое через `TabContent`:

```rust
pub struct TabItem {
    pub id: TabId,
    pub title: String,
    pub content: TabContent,
}

pub enum TabContent {
    Terminal(TerminalTabState),
    Settings(SettingsTabState),
    Empty,
}
```

`TerminalTabState` — это бывший `TabState`, только живет в доменном
пространстве `tabs/terminal.rs` и не зависит от экранов.

Теперь таб — контейнер. В будущем можно добавить:

```rust
pub enum TabContent {
    Terminal(TerminalTabState),
    Settings(SettingsTabState),
    TextViewer(TextViewerState),
}
```

### 4) Сервисный слой (Services)

Вопрос: нужен ли `ServiceRegistry`?

Ответ: да, но тонкий. Сервисы нужны для I/O (PTY, shell scripts,
clipboard, window actions). Их нельзя смешивать с редьюсером.

Подход:

- Редьюсер меняет состояние и возвращает `Task<AppEvent>`.
- Сервисы используют `Task`/`Command` Iced.

Пример эффекта:

```rust
fn focus_terminal(widget_id: iced::widget::Id) -> Task<AppEvent> {
    otty_ui_term::TerminalView::focus(widget_id).map(AppEvent::Terminal)
}
```

Если сервисов станет много — можно ввести `AppEffect` и отдельный
`effect_handler`:

```rust
pub enum AppEffect {
    FocusTerminal(iced::widget::Id),
    ClipboardWrite(String),
    CloseWindow,
}

pub fn handle_effect(effect: AppEffect) -> Task<AppEvent> {
    match effect {
        AppEffect::FocusTerminal(id) => TerminalView::focus(id).map(AppEvent::Terminal),
        AppEffect::ClipboardWrite(text) => iced::clipboard::write(text),
        AppEffect::CloseWindow => window::latest().and_then(window::close),
    }
}
```

### 5) Пример обработки события (Split Pane)

UI -> reducer -> state change -> task:

```rust
pub fn workspace_reducer(
    state: &mut WorkspaceState,
    event: TabEvent,
) -> Task<AppEvent> {
    match event {
        TabEvent::SplitPane { tab_id, pane, axis } => {
            let Some(tab) = state.tab_items.get_mut(&tab_id) else {
                return Task::none();
            };

            let TabContent::Terminal(term_state) = &mut tab.content else {
                return Task::none();
            };

            let terminal_id = state.next_terminal_id;
            state.next_terminal_id += 1;

            let task = term_state.split_pane(pane, axis, terminal_id);
            task.map(AppEvent::Tab)
        }
        _ => Task::none(),
    }
}
```

### 6) Подписки (Subscriptions)

Каждый `TabContent` сам дает subscription.

```rust
pub fn subscription(state: &WorkspaceState) -> Subscription<AppEvent> {
    let mut subs = Vec::new();

    for id in &state.tabs {
        if let Some(tab) = state.tab_items.get(id) {
            if let TabContent::Terminal(term) = &tab.content {
                subs.push(term.subscription().map(AppEvent::Terminal));
            }
        }
    }

    Subscription::batch(subs)
}
```

## Поток событий (пример)

**Split pane**

1. UI: клик по split -> `TabEvent::SplitPane`.
2. Reducer: меняет `TerminalTabState`, создает новый терминал, фокусирует.
3. Возвращает `Task` (focus).
4. UI рендерится по новому state.

**Copy selection**

1. UI: клик "Copy" -> `TabEvent::CopySelection`.
2. Reducer: проверяет активный терминал, возвращает `Task` для `TerminalView`.
3. Clipboard effect выполняется в сервисах или напрямую через `Task`.

## Предлагаемый план миграции

1. Вынести `WorkspaceState` из `TerminalScreen`.
2. Перевести `TerminalScreen` в `terminal/view.rs` (stateless).
3. Переместить `TabState` в `tabs/terminal.rs` и переименовать в
   `TerminalTabState`.
4. Ввести `TabItem` + `TabContent` enum.
5. Перекинуть обработку событий в редьюсеры.
6. Добавить эффект-слой (если требуется).

## Риски и вопросы

- Нужен ли trait-based `TabContent` сразу, или достаточно enum?
- Как удобно прокидывать подписки и команды для разных типов контента.
- Где провести границу между редьюсером и effect handler.

Ответы:

- `TabContent` достаточно оставить `enum` (trait пока не нужен).
- Подписки и команды удобнее собирать на уровне `App`, а не в экранах.
  Экран остается stateless, `App` батчит `Subscription::batch(...)` для
  активных табов и типов контента.
- Граница reducer/effect: редьюсер меняет state и возвращает `Task`/`Effect`,
  а любая I/O операция (clipboard, window, PTY, filesystem) выполняется
  только через `Task` или effect handler. Это дает чистую логику и тесты.

## Ожидаемый результат

- Экраны тонкие и stateless.
- Все состояние централизовано.
- Вкладки гибкие и расширяемые.
- Логика тестируется отдельно от UI.
