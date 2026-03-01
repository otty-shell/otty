# PRD: Переход `otty` на Widgets-First архитектуру

## 1. Идея

Сделать `widget` основной единицей приложения.

- `widgets` — вертикальные срезы: `view + state + model + event + reducer + services/storage`.
- `components` — только shared UI primitives (кнопки, menu items, иконки, простые layout-обертки).
- `app` (логический слой в корневых модулях) — только композиция и маршрутизация между widget-ами.

Это заменяет текущую двойную модель `features + ui/widgets` на одну: `widgets-first`.

## 2. Почему меняем

Текущая архитектура дублирует boundaries:

- Бизнес-логика лежит в `features/*`, а UI-часть в `ui/widgets/*`.
- Связи между `features` и `ui` местами пересекаются.
- При изменении одной фичи приходится ходить по нескольким деревьям файлов.

Widgets-first уменьшает когнитивную нагрузку: один use-case = один widget-модуль.

### 2.1 Статус текущих конвенций

Текущие архитектурные конвенции (`CONVENTIONS.md`), где `features/*` зафиксированы как MUST, считаются переходными и не используются как источник истины для этого rewrite.

До переписывания конвенций под `widgets-first` нормативным архитектурным документом является этот PRD.

## 3. Цели

1. Повысить читаемость: все, что относится к `sidebar`/`quick_launch`/`settings`, лежит рядом.
2. Упростить расширение: новая фича добавляется как новый widget-срез.
3. Снизить связанность: `components` не знают про `app` и бизнес-логику.
4. Сохранить текущее поведение приложения без функциональных регрессий.

## 4. Non-goals

1. Полный UI-редизайн.
2. Переписывание низкоуровневых crates (`otty-libterm`, `otty-ui-term`, `otty-pty`, и т.д.).
3. Смена event loop/framework (`iced` остается).

## 5. Целевая архитектура

## 5.1 Высокоуровневая схема

`components -> widgets -> app`

Где:

1. `components`
- Чистые переиспользуемые визуальные блоки.
- Без бизнес-состояния.
- Без `crate::app::Event`.
- Без I/O и side effects.

2. `widgets/<name>`
- Самодостаточная единица предметной области.
- Внутри допускаются:
  - `view/`
    - `mod.rs` (entrypoint/aggregator presentation-слоя)
  - `event.rs`
  - `command.rs`
  - `state.rs`
  - `model.rs`
  - `reducer.rs`
  - `services.rs` (интеграции)
  - `storage.rs` (постоянное хранение)
- Один публичный интерфейс через `mod.rs`.

3. `app` (корневые модули)
- Собирает экран из widget-ов.
- Маршрутизирует два отдельных канала событий: `UiEvent` и `EffectEvent`.
- Использует типизированные маршруты `AppEvent::<Widget>Ui(WidgetUiEvent)` и `AppEvent::<Widget>Effect(WidgetEffectEvent)`.
- Обрабатывает `UiEvent` только через `WidgetCommand`, а `EffectEvent` только как app task/action.
- Применяет правило: одно событие (`Ui` или `Effect`) — одно действие, независимо от источника.
- `src/update.rs` остается thin-dispatch слоем: верхнеуровневый `match` по `AppEvent`; допускается app-wide pre-dispatch guard-блок перед dispatch, если он не выполняет widget-specific `map_*` и не содержит бизнес-логики конкретного widget-а.
- Доменная маршрутизация и `map_*` (`Ui -> Command`, `Effect -> AppEvent/Task`) выносится в owning router-модули с entrypoint-ами `route_event/route_effect`.
- Кросс-widget orchestration выполняется в слое `routers/*`: для простых one-hop сценариев допускается прямой `Task<AppEvent::<Target>...>` из owning router-а, для сложных/многошаговых/переиспользуемых сценариев используется `AppFlowEvent` + `routers/flow/*` (где `flow/mod.rs` делает только thin-dispatch по use-case).
- Управляет общесистемными событиями окна/клавиатуры/subscriptions.
- Не содержит бизнес-правил конкретного widget-а.

## 5.2 Целевая структура каталогов и модулей

```text
otty/src/
  app.rs
  update.rs
  routers/
    mod.rs
    sidebar.rs
    tabs.rs
    flow/                              # optional: shared cross-widget use-cases
      mod.rs
      tabs.rs
      quick_launch.rs
      settings.rs
    window.rs
  view.rs
  subscription.rs
  components/
    mod.rs
    primitive/
      mod.rs
      icon_button.rs
      menu_item.rs
      resize_grips.rs
      ...
    composed/
      mod.rs
      sidebar_workspace_panel.rs
  widgets/
    sidebar/
      mod.rs
      view/
        mod.rs
      event.rs
      command.rs
      state.rs
      model.rs
      reducer.rs
    tabs/
      mod.rs
      view/
        mod.rs
      event.rs
      command.rs
      state.rs
      model.rs
      reducer.rs
    quick_launch/
      mod.rs
      view/
        mod.rs
      event.rs
      command.rs
      state.rs
      model.rs
      reducer.rs
      services.rs
      storage.rs
    explorer/
    terminal_workspace/
    settings/
  shared/
    (только реально общие модели/типы, если нужны)
```

Примечание: отдельная папка `app/` не требуется. Для декомпозиции orchestration-логики используется корневая `src/routers/` при сохранении thin `src/update.rs`.

## 5.3 Подробная классификация: `component` vs `widget` vs `shared`

### Component

`Component` — переиспользуемый UI-слой без продуктовой логики.

Обязательные признаки:

1. Не хранит бизнес-state.
2. Не импортирует `crate::app::Event`.
3. Не импортирует `widgets/*`.
4. Делится на два подслоя: `components/primitive/*` и `components/composed/*`.
5. `primitive` — leaf: не импортирует другие `components/*`.
6. `composed` может импортировать только `components/primitive/*`.
7. `composed` не импортирует `components/composed/*` (запрещены цепочки composed -> composed).
8. Композиция компонентов допускается в `components/composed/*`, `widgets/*` и root `src/view.rs`.
9. Не делает I/O и не создает `Task`.
10. Имеет маленький и стабильный API (`Props`, локальный `Event`, `view`).

Текущие файлы, которые относятся к `components/primitive`:

1. `otty/src/ui/components/icon_button.rs`
2. `otty/src/ui/components/menu_item.rs`
3. `otty/src/ui/components/resize_grips.rs` (после отвязки от `crate::app::Event`)

Текущие файлы, которые относятся к `components/composed`:

1. `otty/src/ui/components/sidebar_workspace_panel.rs`

### Widget

`Widget` — самостоятельный vertical slice предметной области.

Обязательные признаки:

1. Владеет своим `WidgetState` внутри `Widget`-структуры.
2. Имеет `reduce(&mut self, command, ctx) -> Task<WidgetEffectEvent>`.
3. Имеет свои `model/event/view`.
4. Имеет свой `command`.
5. Может иметь `services/storage` для side effects этого же widget-а.
6. Не импортирует другие widget-ы напрямую.

Текущие корневые widget-срезы (по бизнес-доменам):

1. `sidebar`
2. `tabs`
3. `terminal_workspace`
4. `quick_launch` (включая editor/wizard + error tab)
5. `explorer`
6. `settings`
7. `chrome` (window controls/action bar)

### Shared

`Shared` — общее, но не UI-примитив и не отдельный widget.

Обязательные признаки:

1. Используется минимум двумя widget-ами или app orchestration.
2. Не содержит ownership конкретного widget-а.
3. Имеет узкую ответственность (тип/утилита/тема/assets).

Текущие файлы, которые должны считаться `shared`:

1. `otty/src/theme.rs`
2. `otty/src/fonts.rs`
3. `otty/src/icons.rs`
4. `otty/src/ui/widgets/services.rs` (нужно разнести в `shared/ui/*` helper-модули)

Файлы orchestration (не `shared`, не `widget`, не `component`):

1. `otty/src/app.rs`
2. `otty/src/update.rs`
3. `otty/src/view.rs`
4. `otty/src/subscription.rs`
5. `otty/src/guards.rs`
6. `otty/src/state.rs`
7. `otty/src/routers/*`

## 5.4 Подробное целевое дерево (ориентация на текущий код)

```text
otty/src/
  app.rs                              # root app state + Event enum (из текущего app.rs)
  update.rs                           # thin dispatch only (AppEvent -> routers::*::route_event/route_effect)
  routers/
    mod.rs                            # domain router registrations
    sidebar.rs                        # sidebar routing + map_event/map_effect
    tabs.rs                           # tabs routing + map_event/map_effect
    flow/                              # optional package for multi-step/reusable cross-widget orchestration
      mod.rs                          # thin flow dispatch only (AppFlowEvent -> flow::<use_case>::route_event)
      tabs.rs                         # example flow use-cases: tabs
      quick_launch.rs                 # example flow use-cases: quick launch
      settings.rs                     # example flow use-cases: settings
    window.rs                         # app-global window/runtime events
  view.rs                             # root layout composition (из текущего view.rs)
  subscription.rs                     # subscriptions wiring (из текущего subscription.rs)
  guards.rs                           # app-level policies/guards (из текущего guards.rs)
  state.rs                            # app runtime geometry/window state (из текущего state.rs)

  components/
    mod.rs
    primitive/
      mod.rs
      icon_button.rs                  # from ui/components/icon_button.rs
      menu_item.rs                    # from ui/components/menu_item.rs
      resize_grips.rs                 # from ui/components/resize_grips.rs (decoupled event)
    composed/
      mod.rs
      sidebar_workspace_panel.rs      # from ui/components/sidebar_workspace_panel.rs

  shared/
    ui/
      theme.rs                        # from theme.rs (может остаться физически в корне на этапе 1)
      fonts.rs                        # from fonts.rs
      icons.rs                        # from icons.rs
      menu_geometry.rs                # from ui/widgets/services.rs: anchor_position/menu_height_for_items
      menu_style.rs                   # from ui/widgets/services.rs: menu_panel_style
      tree_style.rs                   # from ui/widgets/services.rs: tree_row_style/thin_scroll_style

  widgets/
    chrome/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      view/
        mod.rs                        # entrypoint/aggregator for chrome view
        action_bar.rs                 # from ui/widgets/action_bar.rs

    sidebar/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      view/
        mod.rs                        # entrypoint/aggregator for sidebar view
        menu_rail.rs                  # from ui/widgets/sidebar_menu.rs
        workspace_host.rs             # from ui/widgets/sidebar_workspace.rs
        add_menu_overlay.rs           # from ui/widgets/sidebar_workspace_add_menu.rs
        workspace_header.rs           # split from ui/widgets/sidebar_workspace_terminal.rs

    tabs/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      view/
        mod.rs                        # entrypoint/aggregator for tabs view
        tab_bar.rs                    # from ui/widgets/tab_bar.rs
        tab_content.rs                # from ui/widgets/tab_content.rs (active tab content + event mapping)

    terminal_workspace/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      services.rs
      view/
        mod.rs                        # entrypoint/aggregator for terminal workspace view
        pane_grid.rs                  # from ui/widgets/terminal_tab.rs
        pane_context_menu.rs          # from ui/widgets/terminal_pane_context_menu.rs

    quick_launch/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      services.rs
      storage.rs
      view/
        mod.rs                        # entrypoint/aggregator for quick launch view
        sidebar_tree.rs               # from ui/widgets/quick_launches_sidebar.rs
        context_menu.rs               # from ui/widgets/quick_launches_context_menu.rs
        wizard_form.rs                # from ui/widgets/quick_launches_wizard.rs
        error_tab.rs                  # from ui/widgets/quick_launches_error.rs
        sidebar_panel.rs              # split from ui/widgets/sidebar_workspace_terminal.rs body

    explorer/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      services.rs
      view/
        mod.rs                        # entrypoint/aggregator for explorer view
        sidebar_tree.rs               # from ui/widgets/sidebar_workspace_explorer.rs

    settings/
      mod.rs
      event.rs
      command.rs
      model.rs
      state.rs
      reducer.rs
      services.rs
      storage.rs
      view/
        mod.rs                        # entrypoint/aggregator for settings view
        settings_form.rs              # from ui/widgets/settings.rs
```

## 5.5 Маппинг текущих модулей в целевые widget-срезы

### Текущие `features/*` -> целевые `widgets/*`

1. `features/sidebar/*` -> `widgets/sidebar/*`
2. `features/tab/*` -> `widgets/tabs/*`
3. `features/terminal/*` -> `widgets/terminal_workspace/*`
4. `features/quick_launch/*` -> `widgets/quick_launch/*`
5. `features/quick_launch_wizard/*` -> `widgets/quick_launch/*` (внутренний submodule `wizard/*`)
6. `features/explorer/*` -> `widgets/explorer/*`
7. `features/settings/*` -> `widgets/settings/*`

### Текущие `ui/widgets/*` -> целевые `widgets/*` или `app/*`

1. `ui/widgets/action_bar.rs` -> `widgets/chrome/view/action_bar.rs`
2. `ui/widgets/sidebar_menu.rs` -> `widgets/sidebar/view/menu_rail.rs`
3. `ui/widgets/sidebar_workspace.rs` -> `widgets/sidebar/view/workspace_host.rs`
4. `ui/widgets/sidebar_workspace_add_menu.rs` -> `widgets/sidebar/view/add_menu_overlay.rs`
5. `ui/widgets/sidebar_workspace_terminal.rs` -> split:
- header -> `widgets/sidebar/view/workspace_header.rs`
- quick-launch body -> `widgets/quick_launch/view/sidebar_panel.rs`
6. `ui/widgets/sidebar_workspace_explorer.rs` -> `widgets/explorer/view/sidebar_tree.rs`
7. `ui/widgets/tab_bar.rs` -> `widgets/tabs/view/tab_bar.rs`
8. `ui/widgets/tab_content.rs` -> `widgets/tabs/view/tab_content.rs`
9. final composition route остаётся в `src/view.rs`
10. `ui/widgets/terminal_tab.rs` -> `widgets/terminal_workspace/view/pane_grid.rs`
11. `ui/widgets/terminal_pane_context_menu.rs` -> `widgets/terminal_workspace/view/pane_context_menu.rs`
12. `ui/widgets/quick_launches_sidebar.rs` -> `widgets/quick_launch/view/sidebar_tree.rs`
13. `ui/widgets/quick_launches_context_menu.rs` -> `widgets/quick_launch/view/context_menu.rs`
14. `ui/widgets/quick_launches_wizard.rs` -> `widgets/quick_launch/view/wizard_form.rs`
15. `ui/widgets/quick_launches_error.rs` -> `widgets/quick_launch/view/error_tab.rs`
16. `ui/widgets/settings.rs` -> `widgets/settings/view/settings_form.rs`
17. `ui/widgets/services.rs` -> `shared/ui/menu_geometry.rs`, `shared/ui/menu_style.rs`, `shared/ui/tree_style.rs`

## 6. Правила зависимостей

1. `components/primitive/*` могут зависеть только от `iced`, `theme`, `icons`, `fonts`.
2. `components/composed/*` могут зависеть только от `iced`, `theme`, `icons`, `fonts` и `components/primitive/*`.
3. Все `components/*` не импортируют `app`, `widgets`.
4. `components/primitive/*` не импортируют другие `components/*` (кроме `mod.rs`, который только реэкспортирует).
5. `components/composed/*` не импортируют `components/composed/*` (запрещены цепочки composed -> composed).
6. `widgets` могут зависеть от `components`, `shared`, `theme`, `icons`, `fonts`.
7. `widgets` **не импортируют другие `widgets` напрямую** (ни публичные API, ни внутренности).
8. `widgets` не импортируют `crate::app::Event`/`AppEvent` (и любые app-specific типы).
9. Кросс-widget взаимодействие только через `app` (роутинг `AppEvent::<Widget>Ui(WidgetUiEvent)` / `AppEvent::<Widget>Effect(WidgetEffectEvent)`).
10. Если нужно переиспользование UI — выносить в `components`.
11. Если нужна общая бизнес-модель/тип — выносить в `shared`.
12. Side effects только в `services/storage` текущего widget-а.
13. Состояние widget-а мутируется только в его `reducer`.
14. Использовать passthrough из `view` в `AppEvent::<Widget>Ui(WidgetUiEvent)`, а `Ui -> Command` и `Effect -> AppEvent/Task` выполнять в owning router-модуле через `route_event/route_effect` для каждого widget-а (даже при тривиальном mapping).
15. Каждый `WidgetUiEvent` и `WidgetEffectEvent` всегда имеет один и тот же handler/action в owning router-модуле (`route_event/route_effect`), независимо от источника события.
16. `Ui`-события маппятся только в `WidgetCommand`; `Effect`-события маппятся только в app task/action.
17. Анти-цикл инвариант: запрещено `WidgetUiEvent::A -> WidgetCommand::A -> WidgetUiEvent::A -> ...`; `reduce` типизированно возвращает только `Effect`-события и не может эмитить `Ui`.
18. Для многосоставных реакций допускается `Task::batch`, но каждое событие в batch должно оставаться детерминированным (один event -> одно действие).
19. Чтобы оркестрация не разрасталась, `Ui` и `Effect` перечисления должны быть сгруппированы по поддоменам (`launch/*`, `tabs/*`) при росте и вынесены в `map_*` функции внутри router-модулей.
20. `src/update.rs` НЕ содержит widget-specific mapping/бизнес-ветвления: верхнеуровневая диспетчеризация `AppEvent -> routers::<domain>::route_event/route_effect`; допускается app-wide pre-dispatch guard-блок (например interaction/menu gate), если он кросс-доменный и не выполняет `Ui -> Command` / `Effect -> AppEvent` mapping.
21. Кросс-widget сценарии обрабатываются в `routers/*` через app-level события; для простых one-hop переходов допустим прямой dispatch в `AppEvent::<Target>...`, а для сложных/многошаговых/переиспользуемых сценариев используется `AppFlowEvent` + `routers/flow/*`.
22. Роутеры могут знать несколько widget-ов и вызывать только их публичный API (например, `reduce/vm`); прямые зависимости `router -> router` запрещены.
23. В `map_*_effect_event_to_app_task` допускается и рекомендуется wildcard fallback (`_ => Task::none()`) для неиспользуемых/будущих `Effect`-вариантов; это считается явным no-op, а не потерей события.
24. Для widget-ов с async side effects обязателен lifecycle-контракт операции (`request_id`, single-flight per resource key, stale-result guard, cancel/retry/error semantics), описанный в разделе 7.

## 7. Контракт widget-модуля

Каждый widget обязан иметь:

1. Два отдельных enum-типа: `WidgetUiEvent` и `WidgetEffectEvent` (без общей enum-обертки `WidgetEvent`).
2. `WidgetState` — приватное состояние, owned внутри `Widget`-структуры.
3. Отдельный `WidgetCommand` enum для входа в reducer.
4. `Widget::reduce(&mut self, command, ctx) -> Task<WidgetEffectEvent>` — единственная write-точка.
5. `Widget::vm(&self) -> WidgetViewModel` — read-only API для presentation слоя.
6. `view(props) -> Element<WidgetUiEvent>` — presentation для данного среза.
7. Формальный контракт по типам: `view` может эмитить только `WidgetUiEvent`, `reduce` может эмитить только `WidgetEffectEvent`.
8. На app-уровне используются два маршрута: `AppEvent::<Widget>Ui(WidgetUiEvent)` и `AppEvent::<Widget>Effect(WidgetEffectEvent)`.
9. Для каждого события разрешена ровно одна ветка обработки: либо `Ui -> Command`, либо `Effect -> AppEvent/Task`.
10. Для flow с несколькими шагами использовать цепочку `Effect`-событий: `Prepare* -> Run* -> Done*`, где каждый шаг порождает следующий конечным событием `Task::perform`.
11. Presentation-слой widget-а оформляется только как директория `view/`; одиночный `view.rs` в корне widget-а запрещен.
12. Если в `view/` больше одного файла, обязателен `view/mod.rs` как aggregator (root `view(...)`, re-export и композиция подвидов).
13. Для `Effect`-маршрутизации разрешен fallback `match _ => Task::none()`; это нормативный способ явно игнорировать неприменимые варианты.
14. Каждая async-операция внутри widget-а имеет уникальный `request_id` (u64/UUID) и привязку к ключу ресурса (`resource_key`, например `server_id`).
15. Для операций с эксклюзивностью по ресурсу state хранит `in_flight`-таблицу (`HashMap<resource_key, request_id>` или эквивалент).
16. Действует single-flight per `resource_key`: повторный запуск для того же ключа запрещен, пока текущая операция не завершена (`Done`/`Failed`) или не отменена (`Kill`/`Cancel`).
17. Операции с разными `resource_key` могут выполняться параллельно; UI при этом не блокируется.
18. Для многошаговых async-flow используются события с `request_id`: `Prepare*`, `Run*`, `Done*`, `Failed*`, `Cancelled*`.
19. Результаты `Done/Failed/Cancelled` применяются только если `request_id` совпадает с актуальным значением в `in_flight[resource_key]`; иначе событие считается stale и игнорируется (`Task::none()`/no-op).
20. Отмена (`Kill`/`Cancel`) должна освобождать `resource_key` в `in_flight`, после чего новый запуск для этого же ключа разрешен.
21. Retry разрешен только из terminal-состояний (`Failed`/`Cancelled`) и всегда создает новый `request_id`.
22. Ошибки async-операций должны иметь явную типизацию (например `Transient`/`Permanent`/`Validation`) и маппинг в UI state.
23. Для каждого widget-а с async-операциями обязательны тесты на lifecycle: stale completion, cancel during run, retry после failure, запрет double-start на одном `resource_key`, параллельный запуск на разных `resource_key`.

`ctx` допускается read-only, зависимые runtime-сервисы передаются через него.
`app` не получает `&mut WidgetState` и не мутирует state напрямую.
Поток данных: `view -> WidgetUiEvent -> AppEvent::<Widget>Ui(event) -> route_event -> WidgetCommand -> reduce -> Task<WidgetEffectEvent> -> AppEvent::<Widget>Effect(event) -> route_effect -> app task/action`.

## 8. Примеры

## 8.1 Пример целевого `sidebar` widget-а

```rust
// widgets/sidebar/event.rs
#[derive(Debug, Clone)]
pub(crate) enum SidebarEvent {
    SelectTerminal,
    SelectExplorer,
    ToggleWorkspace,
    OpenSettings,
    AddMenuOpen,
    AddMenuDismiss,
    AddMenuCreateTab,
}

#[derive(Debug, Clone)]
pub(crate) enum SidebarEffect {
    SyncTerminalGridSizes,
    OpenSettingsTab,
}
```

```rust
// widgets/sidebar/mod.rs
pub(crate) struct SidebarWidget {
    state: SidebarState,
}

impl SidebarWidget {
    pub(crate) fn reduce(
        &mut self,
        command: SidebarCommand,
        _ctx: &SidebarCtx<'_>,
    ) -> Task<SidebarEffect> {
        match command {
            SidebarCommand::ToggleWorkspace => {
                self.state.toggle_workspace();
                Task::done(SidebarEffect::SyncTerminalGridSizes)
            },
            SidebarCommand::OpenSettings => {
                Task::done(SidebarEffect::OpenSettingsTab)
            },
            _ => Task::none(),
        }
    }

    pub(crate) fn vm(&self) -> SidebarViewModel {
        SidebarViewModel::from_state(&self.state)
    }
}
```

```rust
// src/update.rs (thin dispatch)
match event {
    AppEvent::SidebarUi(ui_event) => {
        routers::sidebar::route_event(app, ui_event)
    }
    AppEvent::SidebarEffect(effect_event) => {
        routers::sidebar::route_effect(effect_event)
    }
    AppEvent::Flow(flow_event) => routers::flow::route_event(app, flow_event),
    _ => Task::none(),
}
```

```rust
// src/routers/sidebar.rs
pub(crate) fn route_event(
    app: &mut App,
    event: widgets::sidebar::SidebarEvent,
) -> Task<AppEvent> {
    let command = map_sidebar_ui_event_to_command(event);
    app.widgets
        .sidebar
        .reduce(command, &app::SidebarCtx {
            is_workspace_resizing: &app.is_workspace_resizing,
        })
        .map(AppEvent::SidebarEffect)
}

pub(crate) fn route_effect(
    event: widgets::sidebar::SidebarEffect,
) -> Task<AppEvent> {
    map_sidebar_effect_event_to_app_task(event)
}

fn map_sidebar_ui_event_to_command(
    event: widgets::sidebar::SidebarEvent,
) -> widgets::sidebar::SidebarCommand {
    use widgets::sidebar::SidebarCommand as C;
    use widgets::sidebar::SidebarEvent as E;

    match event {
        E::SelectTerminal => C::SelectTerminal,
        E::SelectExplorer => C::SelectExplorer,
        E::ToggleWorkspace => C::ToggleWorkspace,
        E::OpenSettings => C::OpenSettings,
        E::AddMenuOpen => C::AddMenuOpen,
        E::AddMenuDismiss => C::AddMenuDismiss,
        E::AddMenuCreateTab => C::AddMenuCreateTab,
    }
}

fn map_sidebar_effect_event_to_app_task(
    event: widgets::sidebar::SidebarEffect,
) -> Task<AppEvent> {
    use widgets::sidebar::SidebarEffect as E;

    match event {
        E::SyncTerminalGridSizes => Task::done(AppEvent::SyncTerminalGridSizes),
        E::OpenSettingsTab => {
            Task::done(AppEvent::Flow(AppFlowEvent::OpenSettingsTabRequested))
        }
        _ => Task::none(),
    }
}
```

## 8.2 Пример `resize_grips` как component

До (нецелевой вариант):

```rust
use crate::app::Event;
pub(crate) fn view() -> Element<'static, Event> { ... }
```

После (целевой):

```rust
pub(crate) enum ResizeGripEvent {
    Resize(iced::window::Direction),
}

pub(crate) fn view() -> Element<'static, ResizeGripEvent> { ... }
```

Mapping на уровне `src/view.rs`:

```rust
resize_grips::view().map(|event| match event {
    ResizeGripEvent::Resize(dir) => AppEvent::ResizeWindow(dir),
})
```

## 8.3 Полный пример widget-а (state -> vm -> view -> reducer -> app)

Ниже полный сквозной пример, показывающий:

1. где и зачем нужен `vm` mapping;
2. как `view` эмитит `WidgetUiEvent`;
3. как `WidgetUiEvent` маппится в `WidgetCommand`;
4. как `reduce(command, ctx)` возвращает `Task<WidgetEffectEvent>`;
5. как `WidgetEffectEvent` маппится в app-действие.

### 8.3.1 Файлы widget-а

```text
widgets/sidebar/
  mod.rs
  model.rs
  state.rs
  event.rs
  command.rs
  reducer.rs
  view/
    mod.rs
```

### 8.3.2 model.rs

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarItem {
    Terminal,
    Explorer,
}

#[derive(Debug, Clone)]
pub(crate) struct SidebarViewModel {
    pub(crate) active_item: SidebarItem,
    pub(crate) is_workspace_open: bool,
    pub(crate) can_open_add_menu: bool,
}
```

### 8.3.3 state.rs

```rust
use super::model::SidebarItem;

#[derive(Debug)]
pub(crate) struct SidebarState {
    active_item: SidebarItem,
    workspace_open: bool,
    add_menu_open: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            active_item: SidebarItem::Terminal,
            workspace_open: true,
            add_menu_open: false,
        }
    }
}

impl SidebarState {
    pub(crate) fn active_item(&self) -> SidebarItem {
        self.active_item
    }

    pub(crate) fn is_workspace_open(&self) -> bool {
        self.workspace_open
    }

    pub(crate) fn is_add_menu_open(&self) -> bool {
        self.add_menu_open
    }

    pub(crate) fn set_active_item(&mut self, item: SidebarItem) {
        self.active_item = item;
    }

    pub(crate) fn toggle_workspace(&mut self) {
        self.workspace_open = !self.workspace_open;
    }

    pub(crate) fn open_add_menu(&mut self) {
        self.add_menu_open = true;
    }

    pub(crate) fn dismiss_add_menu(&mut self) {
        self.add_menu_open = false;
    }
}
```

### 8.3.4 event.rs

```rust
#[derive(Debug, Clone)]
pub(crate) enum SidebarEvent {
    SelectTerminal,
    SelectExplorer,
    ToggleWorkspace,
    AddMenuOpen,
    AddMenuDismiss,
    AddMenuCreateTab,
    OpenSettings,
}

#[derive(Debug, Clone)]
pub(crate) enum SidebarEffect {
    SyncTerminalGridSizes,
    OpenTerminalTab,
    OpenSettingsTab,
}
```

### 8.3.5 command.rs

```rust
#[derive(Debug, Clone)]
pub(crate) enum SidebarCommand {
    SelectTerminal,
    SelectExplorer,
    ToggleWorkspace,
    AddMenuOpen,
    AddMenuDismiss,
    AddMenuCreateTab,
    OpenSettings,
}
```

### 8.3.6 reducer.rs

```rust
use iced::Task;

use super::command::SidebarCommand;
use super::event::SidebarEffect;
use super::model::SidebarItem;
use super::state::SidebarState;

pub(crate) struct SidebarCtx<'a> {
    pub(crate) is_workspace_resizing: &'a bool,
}

pub(crate) fn reduce(
    state: &mut SidebarState,
    command: SidebarCommand,
    ctx: &SidebarCtx<'_>,
) -> Task<SidebarEffect> {
    match command {
        SidebarCommand::SelectTerminal => {
            state.set_active_item(SidebarItem::Terminal);
            Task::none()
        },
        SidebarCommand::SelectExplorer => {
            state.set_active_item(SidebarItem::Explorer);
            Task::none()
        },
        SidebarCommand::ToggleWorkspace => {
            state.toggle_workspace();
            Task::done(SidebarEffect::SyncTerminalGridSizes)
        },
        SidebarCommand::AddMenuOpen => {
            if *ctx.is_workspace_resizing {
                return Task::none();
            }
            state.open_add_menu();
            Task::none()
        },
        SidebarCommand::AddMenuDismiss => {
            state.dismiss_add_menu();
            Task::none()
        },
        SidebarCommand::AddMenuCreateTab => {
            Task::batch([
                Task::done(SidebarEffect::OpenTerminalTab),
                Task::done(SidebarEffect::SyncTerminalGridSizes),
            ])
        },
        SidebarCommand::OpenSettings => Task::done(SidebarEffect::OpenSettingsTab),
    }
}
```

### 8.3.7 view/mod.rs

```rust
use iced::Element;

use super::event::SidebarEvent;
use super::model::{SidebarItem, SidebarViewModel};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SidebarProps {
    pub(crate) vm: SidebarViewModel,
}

pub(crate) fn view(props: SidebarProps) -> Element<'static, SidebarEvent> {
    let _is_terminal = props.vm.active_item == SidebarItem::Terminal;
    let _is_explorer = props.vm.active_item == SidebarItem::Explorer;
    let _open = props.vm.is_workspace_open;
    let _can_add = props.vm.can_open_add_menu;

    // Упрощенно: в реальном коде здесь iced layout,
    // а кнопки отправляют SidebarEvent напрямую.
    iced::widget::button("Terminal")
        .on_press(SidebarEvent::SelectTerminal)
        .into()
}
```

### 8.3.8 mod.rs (ownership + vm mapping)

```rust
mod event;
mod command;
mod model;
mod reducer;
mod state;
mod view;

pub(crate) use event::{SidebarEffect, SidebarEvent};
pub(crate) use command::SidebarCommand;
pub(crate) use reducer::SidebarCtx;
pub(crate) use view::SidebarProps;

use model::SidebarViewModel;
use state::SidebarState;

pub(crate) struct SidebarWidget {
    state: SidebarState,
}

impl SidebarWidget {
    pub(crate) fn new() -> Self {
        Self {
            state: SidebarState::default(),
        }
    }

    pub(crate) fn reduce(
        &mut self,
        command: SidebarCommand,
        ctx: &SidebarCtx<'_>,
    ) -> Task<SidebarEffect> {
        reducer::reduce(&mut self.state, command, ctx)
    }

    pub(crate) fn vm(&self) -> SidebarViewModel {
        SidebarViewModel {
            active_item: self.state.active_item(),
            is_workspace_open: self.state.is_workspace_open(),
            can_open_add_menu: !self.state.is_add_menu_open(),
        }
    }
}
```

### 8.3.9 app wiring (где происходит mapping)

```rust
// src/view.rs
let sidebar = widgets.sidebar.vm();
let sidebar_element = widgets::sidebar::view(widgets::sidebar::SidebarProps {
    vm: sidebar,
})
.map(AppEvent::SidebarUi);
```

```rust
// src/update.rs (thin dispatch)
match event {
    AppEvent::SidebarUi(event) => routers::sidebar::route_event(app, event),
    AppEvent::SidebarEffect(event) => routers::sidebar::route_effect(event),
    AppEvent::Flow(event) => routers::flow::route_event(app, event),
    _ => Task::none(),
}
```

```rust
// src/routers/sidebar.rs
fn route_event(
    app: &mut App,
    event: widgets::sidebar::SidebarEvent,
) -> Task<AppEvent> {
    let command = map_sidebar_ui_event_to_command(event);
    app.widgets
        .sidebar
        .reduce(
            command,
            &widgets::sidebar::SidebarCtx {
                is_workspace_resizing: &app.is_workspace_resizing,
            },
        )
        .map(AppEvent::SidebarEffect)
}

fn route_effect(
    event: widgets::sidebar::SidebarEffect,
) -> Task<AppEvent> {
    map_sidebar_effect_event_to_app_task(event)
}

fn map_sidebar_ui_event_to_command(
    event: widgets::sidebar::SidebarEvent,
) -> widgets::sidebar::SidebarCommand {
    use widgets::sidebar::SidebarCommand as C;
    use widgets::sidebar::SidebarEvent as E;

    match event {
        E::SelectTerminal => C::SelectTerminal,
        E::SelectExplorer => C::SelectExplorer,
        E::ToggleWorkspace => C::ToggleWorkspace,
        E::AddMenuOpen => C::AddMenuOpen,
        E::AddMenuDismiss => C::AddMenuDismiss,
        E::AddMenuCreateTab => C::AddMenuCreateTab,
        E::OpenSettings => C::OpenSettings,
    }
}

fn map_sidebar_effect_event_to_app_task(
    event: widgets::sidebar::SidebarEffect,
) -> Task<AppEvent> {
    use widgets::sidebar::SidebarEffect as E;

    match event {
        E::SyncTerminalGridSizes => Task::done(AppEvent::SyncTerminalGridSizes),
        E::OpenTerminalTab => Task::done(AppEvent::OpenTerminalTab),
        E::OpenSettingsTab => {
            Task::done(AppEvent::Flow(AppFlowEvent::OpenSettingsTabRequested))
        }
        _ => Task::none(),
    }
}
```

### 8.3.10 Когда `vm` mapping нужен, а когда нет

Нужен:

1. когда во `state` есть поля, которые view не должна видеть напрямую (внутренние флаги/инварианты);
2. когда view нужны вычисленные/агрегированные поля;
3. когда нужно стабилизировать контракт view при изменениях внутреннего state.

Можно упростить:

1. если widget очень маленький и state уже безопасен для чтения;
2. если нет derived данных и нет риска раскрытия внутренних деталей.

Рекомендация для `otty`: оставлять `vm()` по умолчанию для средних/крупных widget-ов, чтобы упростить дальнейшую эволюцию модели.

### 8.3.11 Как избежать дублирования действий

1. Используются два входа в роутер: `AppEvent::SidebarUi(event)` и `AppEvent::SidebarEffect(event)`.
2. `SidebarEvent` обрабатываются только как `WidgetCommand`.
3. `SidebarEffect` обрабатываются только как app task/action.
4. Событие не может одновременно существовать в `map_sidebar_ui_event_to_command` и в `map_sidebar_effect_event_to_app_task`.
5. События, возвращаемые из `reduce`, не маппятся обратно в `WidgetCommand`.
6. Для нескольких независимых реакций из одного command использовать `Task::batch`.
7. Для многошаговых async flow использовать цепочку `Effect`-событий: `Prepare* -> Run* -> Done*` (подготовка и запуск отдельными шагами, каждый шаг завершает `Task::perform(..., |res| final_event)`).
8. Для простого one-hop кросс-widget перехода роутер может отправлять целевой `AppEvent` напрямую; `AppFlowEvent` используется, когда оркестрация становится многошаговой или должна переиспользоваться в нескольких местах.

## 9. План реализации и phase gates

### 9.1 Стратегия реализации (rewrite-only)

1. Единственный трек реализации: `Full Rewrite` в целевой структуре `widgets-first`.
2. Перенос внутренней реализации из `features/*` и `ui/widgets/*` не обязателен; legacy-код используется только как reference.
3. Любой новый или изменяемый домен реализуется сразу в `widgets/*` + `routers/*`.
4. Функциональный baseline из раздела 13 обязателен для проверки паритета поведения.

### 9.2 Gate 0: Старт фазы реализации

Выход из Gate 0:

1. Зафиксированы scope и owner ближайшей фазы rewrite (какие widget-ы и сценарии входят в фазу).
2. Зафиксирован baseline из раздела 13 (включая дату snapshot и ожидаемый функциональный паритет).
3. Принят критерий DoD фазы: какие widget-ы входят в фазу и какие сценарии считаются обязательными.

Blockers Gate 0:

1. Нет зафиксированных scope/owner для ближайшей фазы rewrite.
2. Нет подтвержденного baseline для регрессионной проверки.

### 9.3 Phase 1: Каркас и маршрутизация

Работы:

1. Создать/достроить `widgets/` как основной root.
2. Создать `routers/*` и перевести `src/update.rs` в thin-dispatch режим.
3. Зафиксировать governance: новые изменения в затронутых доменах идут только через `widgets-first`.

Exit criteria:

1. `src/update.rs` содержит только top-level dispatch `AppEvent -> routers::<domain>::route_event/route_effect`.
2. Для каждого уже затронутого widget-а существует owning router с entrypoint-ами `route_event/route_effect` и `Ui -> Command` / `Effect -> AppEvent/Task` mapping.
3. Добавлен хотя бы один полностью рабочий reference widget в целевой структуре (`mod/event/command/state/model/reducer/view/mod.rs`).

Blockers:

1. В `src/update.rs` остается widget-specific mapping/бизнес-ветвление.
2. Для уже затронутых доменов новые продуктовые изменения продолжают добавляться в legacy-структуру вместо `widgets-first`.

### 9.4 Phase 2: Core Navigation (Sidebar, Tabs, Chrome)

Работы:

1. Реализовать `sidebar`, `tabs`, `chrome` с нуля в `widgets-first`.
2. Подключить роутеры доменов и убрать runtime-зависимость от legacy-реализации для этих доменов.

Exit criteria:

1. Сценарии `sidebar`/`tabs`/`chrome` из baseline работают без регрессий.
2. Для `sidebar`, `tabs`, `chrome` действует правило owning router и запрет прямых импортов между widget-ами.
3. Для этих доменов нет runtime-path/fallback в `features/*` или `ui/widgets/*`.

Blockers:

1. Любой из базовых пользовательских сценариев (toggle sidebar, tab activate/close, window controls) регрессирует.
2. Есть циклы `Event -> Command -> same Event` без тестового покрытия/исправления.

### 9.5 Phase 3: Quick Launch + Wizard

Работы:

1. Реализовать `widgets/quick_launch` и внутренние submodules (`drag_drop`, `launch`, `persistence`, `wizard`) в едином срезе.
2. Привести async lifecycle к контракту `request_id + in_flight + single-flight per resource_key`.

Exit criteria:

1. Сценарии quick launch/wizard из baseline работают без регрессий.
2. Есть тесты lifecycle: stale completion, cancel, retry, single-flight на одном ключе, parallel на разных ключах.
3. Persistence path для quick launch покрыт тестами successful/failed completion.

Blockers:

1. Async-flow не удовлетворяет lifecycle-контракту раздела 7.
2. Wizard flow напрямую мутирует state другого widget-а, обходя app routing.

### 9.6 Phase 4: Explorer, Settings, Terminal Workspace

Работы:

1. Реализовать `explorer`, `settings`, `terminal_workspace` с нуля в `widgets-first`.
2. Довести `app.rs/view.rs/subscription.rs` до роли composer/orchestrator без widget-specific бизнес-правил.

Exit criteria:

1. Сценарии explorer/settings/terminal workspace из baseline работают без регрессий.
2. `app`-слой не получает прямой доступ к `&mut WidgetState`, мутации происходят через `reduce`.
3. Кросс-widget взаимодействие реализовано только через app-level события и роутеры.

Blockers:

1. В orchestration-слое остается доменная бизнес-логика widget-а.
2. Есть прямые импорты `widgets::<other>` из `widgets::<name>`.

### 9.7 Phase 5: Cutover и удаление legacy-слоя

Работы:

1. Выполнить финальный cutover на новую архитектуру и убрать runtime-путь к legacy-реализации.
2. Удалить `features/*`, legacy `ui/widgets/*`, transitional adapters и мертвый код.
3. Обновить `CONVENTIONS.md` и синхронизировать с `widgets-first`.

Exit criteria:

1. Нет runtime-зависимости от legacy-архитектуры (`features-first`) в целевом `otty`.
2. Полный набор проверок из раздела 10 проходит.
3. `CONVENTIONS.md` не содержит конфликтующих правил относительно `widgets-first`.

Blockers:

1. Невозможно собрать/запустить приложение без legacy-ветки.
2. Есть двойной стандарт в конвенциях (`features-first` vs `widgets-first`).

## 10. Критерии приемки

1. В кодовой базе нет новых бизнес-изменений вне `widgets/*`.
2. `components/*` не импортируют `crate::app::Event`.
3. `components/primitive/*` не импортируют другие `components/*` (кроме `components/mod.rs` и `components/primitive/mod.rs`).
4. `components/composed/*` импортируют только `components/primitive/*` (без `composed -> composed` цепочек).
5. Для каждого widget-а файлы `event.rs/command.rs/state.rs/model.rs/reducer.rs` лежат в корне `widgets/<name>/`, а presentation-слой лежит только в `widgets/<name>/view/`.
6. Прямых импортов `widgets::<other>` из `widgets::<name>` нет.
7. `widgets/*` не импортируют `crate::app::Event`/`AppEvent`.
8. Кросс-widget взаимодействие идет только через `app`.
9. Все проверки проходят:
- `cargo +nightly fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo deny check`
- `cargo test --workspace --all-features`
- `cargo llvm-cov --workspace --all-features --fail-under-lines 80`
10. Для каждого widget есть тест/проверка маршрутизации, подтверждающая отсутствие цикла `UiEvent -> Command -> same UiEvent`.
11. В каждом widget-е есть отдельные типы `UiEvent` и `EffectEvent`; `view` возвращает `Element<UiEvent>`, а `reduce` возвращает `Task<EffectEvent>`.
12. Для сценариев с несколькими side-effect шагами есть отдельные `Effect`-события подготовки и запуска (`Prepare*`, `Run*`, `Done*`).
13. `src/update.rs` содержит top-level dispatch (`AppEvent -> routers::<domain>::route_event/route_effect`) и не содержит widget-specific `map_*`; допускается app-wide pre-dispatch guard-блок, если он не содержит доменной бизнес-логики widget-ов и не делает widget-specific routing mapping.
14. Для каждого widget-а существует owning router модуль с явным `Ui -> Command` и `Effect -> AppEvent/Task` mapping через `route_event/route_effect`; для `Effect` разрешен wildcard no-op fallback.
15. В каждом widget-е существует `view/mod.rs`; если `view/` разбит на несколько файлов, `view/mod.rs` используется как обязательный aggregator/root `view(...)`.
16. Wildcard fallback в `map_*_effect_event_to_app_task` (`_ => Task::none()`) считается корректным поведением и не является нарушением детерминизма.
17. Для каждого widget-а с async flow в state хранится `in_flight` по `resource_key` (например `server_id`) с актуальным `request_id`.
18. Для каждого widget-а с async flow соблюдается single-flight per `resource_key`: повторный запуск для того же ключа блокируется до `Done/Failed/Kill`, и это покрыто тестом.
19. Для каждого widget-а с async flow операции на разных `resource_key` запускаются параллельно и покрыты тестом конкурентного запуска.
20. Для каждого widget-а с async flow реализованы и протестированы cancel/retry semantics (после cancel/retry всегда новый `request_id`).
21. Для каждого widget-а с async flow есть тесты stale completion (`request_id` mismatch) и корректного освобождения `resource_key` после `Done/Failed/Cancelled`.
22. `CONVENTIONS.md` обновлен и синхронизирован с `widgets-first`; двойного стандарта (`features-first` vs `widgets-first`) не осталось.
23. Роутеры не импортируют друг друга; кросс-widget координация выполняется через `AppEvent` и публичный API widget-ов.
24. Использование `routers/flow/*` не является обязательным для каждого сценария: для one-hop кейсов допускается прямой dispatch, а если `flow` используется — `flow/mod.rs` остается thin-dispatch без доменной бизнес-логики.

## 11. Риски и смягчение

1. Риск: временное параллельное существование legacy и rewrite модулей до финального cutover.
- Смягчение: phase gates с явным запретом runtime fallback на legacy для завершенных доменов и ранний cleanup.

2. Риск: регрессии event routing.
- Смягчение: тесты на routing и сценарии tab/quick launch/sidebar.

3. Риск: неконтролируемый рост root-модулей orchestration (`app.rs`, `update.rs`) и превращение `flow` (если используется) в god-router.
- Смягчение: `src/update.rs` фиксируется как thin dispatch, а доменные `Event -> Command` и `Event -> AppEvent` преобразования выносятся в роутеры с `route_event/route_effect`; one-hop сценарии остаются в owning router-ах, а многошаговые общие flow декомпозируются в `routers/flow/*`, где `flow/mod.rs` только делегирует.

4. Риск: гонки async-операций (stale completion, double-run на одном ресурсе, некорректный retry/cancel).
- Смягчение: lifecycle-контракт с `request_id` + `in_flight` по `resource_key` (single-flight per key, parallel across keys) + обязательные тесты stale/cancel/retry/race.

## 12. Метрики успеха

1. Изменение любой фичи затрагивает в основном один `widgets/<name>` каталог.
2. Количество импортов `ui/widgets` из бизнес-логики стремится к нулю (после cutover отсутствует).
3. Время на внедрение новой фичи сокращается за счет шаблонного widget-контракта.

## 13. Функциональный Baseline (As-Is)

Этот раздел фиксирует текущее поведение приложения до rewrite и используется как регрессионный чеклист.

Снимок baseline: **26 February 2026**.

Тестовый baseline (факт прогона):

1. Выполнен `cargo test -p otty --all-features`.
2. Результат: `107 passed; 0 failed` (`106` unit tests + `1` integration test `widget_conventions_ast`).

### 13.1 Подтвержденные функциональные области

1. Старт приложения и инициализация первой вкладки терминала.
- На `IcedReady` создается терминальная вкладка по умолчанию (`src/update.rs`).
- Инициализация shell session использует fallback при ошибке setup (`src/app.rs`, `features/terminal/services.rs` tests).

2. Управление окном и chrome.
- Action bar поддерживает: fullscreen toggle, minimize, close, drag window, toggle sidebar (`src/ui/widgets/action_bar.rs`, `src/update.rs`).
- Поддерживается resize окна (resize grips + `Event::ResizeWindow`) и пересчет terminal grid (`src/view.rs`, `src/update.rs`).

3. Tabs и типы контента вкладок.
- Поддерживаются content-типы: `Terminal`, `Settings`, `QuickLaunchWizard`, `QuickLaunchError` (`features/tab/model.rs`).
- Поддерживаются сценарии: activate, close, set title, open settings tab, open terminal/command/wizard/error tabs (`features/tab/event.rs`, `features/tab/feature.rs` tests).

4. Терминальный workspace.
- Поддерживаются pane-grid действия: click/focus, resize, split, close pane, context menu open/close, close all menus (`features/terminal/event.rs`, `features/terminal/state.rs` tests).
- Поддерживаются copy/paste и block-copy действия из pane context menu (`features/terminal/event.rs`).
- Поддерживаются apply theme и selection sync (`features/terminal/event.rs`, `src/update.rs`).

5. Explorer.
- Поддерживается lazy loading root/folder, hover/select, load-failed path, сортировка узлов, sync root из active terminal cwd (`features/explorer/event.rs`, `feature/state/model/services` tests).

6. Quick Launch tree и контекстное меню.
- Поддерживаются tree interactions: hover/press/release/right click, drag/drop target updates, background interactions (`features/quick_launch/event.rs`, `features/quick_launch/feature.rs`).
- Поддерживаются context menu actions: create folder/command, rename, duplicate, remove/delete, edit, kill (`features/quick_launch/model.rs`, `features/quick_launch/feature.rs`, `ui/widgets/quick_launches_context_menu.rs`).
- Поддерживаются inline edit create/rename и валидация title/конфликтов (`features/quick_launch/feature.rs`, `features/quick_launch/model.rs` tests).

7. Quick Launch запуск, persistence и async lifecycle.
- Поддерживается launch preflight/setup с исходами prepared/failed/canceled и открытие terminal/error tab по результату (`features/quick_launch/model.rs`, `features/quick_launch/feature.rs`).
- Поддерживаются stale/canceled completion guards и kill active launch (`features/quick_launch/feature.rs` tests: stale/canceled/failed setup).
- Поддерживается tick-driven launch indicator + persistence flush и обработка persist completed/failed (`src/subscription.rs`, `features/quick_launch/feature.rs` tests, `features/quick_launch/storage.rs` tests).

8. Quick Launch Wizard.
- Поддерживаются create/edit init, command type switch (Custom/SSH), field updates (args/env/ssh), save/cancel/error flows (`features/quick_launch_wizard/event.rs`, `feature/state/model` tests).
- Save из wizard не мутирует quick launch state напрямую, а идет через typed request (`features/quick_launch_wizard/feature.rs` tests).

9. Settings.
- Поддерживаются reload/save/reset, tree navigation (node hover/press), shell/editor inputs, palette edits, presets (`features/settings/event.rs`, `features/settings/feature.rs`, `features/settings/state.rs` tests).
- Поддерживаются storage round-trip и corrupted json fallback (`features/settings/storage.rs` tests).

10. Глобальные guards и interaction safety.
- Контекстные меню блокируют/разрешают события по guard policy (`src/guards.rs` tests).
- Inline edit guard корректно предотвращает нежелательные отмены/перекрытия (`src/guards.rs` tests).

11. Subscriptions/runtime wiring.
- Активные terminal widgets подписаны на terminal events (`src/subscription.rs`).
- Window + keyboard subscriptions всегда активны; quick launch tick включается только при активных launch/dirty/persist_in_flight (`src/subscription.rs`).

### 13.2 Регрессионный чеклист после rewrite

1. Прогнать `cargo test -p otty --all-features` и сохранить паритет: не меньше `107` passed на baseline-сценариях.
2. Проверить ручные smoke-сценарии: open/close tabs, split/close panes, quick launch create/edit/launch/kill, settings save/reload, explorer sync from terminal.
3. Проверить отсутствие функциональных регрессий по областям 13.1 до финального удаления legacy-слоя.
