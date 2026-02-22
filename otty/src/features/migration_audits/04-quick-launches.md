# 04. Аудит `quick_launches`

## Зависимости
- Выполнить `02-terminal.md`.
- Выполнить `03-quick-launches-editor.md`.

## Текущее состояние (Legacy)
- `otty/src/features/quick_launches/mod.rs:1` объявляет все подмодули как `pub(crate) mod ...` и почти не предоставляет curated API.
- `otty/src/features/quick_launches/mod.rs:8` реэкспортирует только `QuickLaunchErrorState`, но не primary contracts.
- `otty/src/features/quick_launches/event.rs:243` есть дополнительный внешний mutating API `handle_quick_launch_setup_completed` (вторая точка записи).
- `event.rs` перегружен: UI routing + storage/persistence + launch setup + валидация + session assembly.
- `event.rs` импортирует sibling internals terminal: `otty/src/features/quick_launches/event.rs:18` (`terminal::event::settings_for_session`).
- UI слои массово импортируют внутренние модули `quick_launches::{event,model,state}`.
- Тесты strict-матрицы отсутствуют.

## Нарушения CONVENTIONS.md
- Раздел 4: нарушение разделения ответственностей (`event.rs` содержит много сервисной и доменной логики).
- Раздел 5.2: запись в feature state идет не только через reducer.
- Раздел 6/7: нет стабильной экспортной поверхности, есть внешние зависимости на internals.
- Раздел 10: отсутствуют required tests.

## Детальный план миграции

### 04.1 Привести `mod.rs` к strict API
- Порядок модулей: `errors`, `event`, `model`, `state`, `storage`, `services`, `editor`.
- Сделать module declarations приватными.
- Реэкспортировать стабильный контракт:
  - `QuickLaunchEvent`
  - `quick_launches_reducer`
  - `QuickLaunchState`
  - `QuickLaunchError`
  - минимально необходимые UI/domain типы (`ContextMenuAction`, `QuickLaunchSetupOutcome` и т.д.) только если они нужны внешним слоям.

Критерий готовности:
- Внешние импорты идут только через `crate::features::quick_launches`.

### 04.2 Оставить единственный внешний reducer
- Включить обработку setup completion внутрь `QuickLaunchEvent` (например, `SetupCompleted(QuickLaunchSetupOutcome)`).
- Убрать внешний `handle_quick_launch_setup_completed`.
- Все записи в `state.quick_launches` проводить только через `quick_launches_reducer`.

Критерий готовности:
- Один внешний write-entrypoint: `quick_launches_reducer`.

### 04.3 Разделить event/model/services
- В `model.rs` оставить чистую валидацию/нормализацию (`validate_custom_command`, `validate_ssh_command`, `quick_launch_error_message`).
- Вынести launch setup/SSH session builder/resolve PATH в `services.rs`.
- В `event.rs` оставить routing и orchestration.

Критерий готовности:
- `event.rs` не содержит прямого env/path/fs-специфичного кода.

### 04.4 Нормализовать зависимости от terminal
- Убрать `terminal::event::settings_for_session` из импортов internals.
- Использовать только re-export из `crate::features::terminal` (например, `terminal_settings_for_session`).

Критерий готовности:
- Нет `crate::features::terminal::event::...` в quick_launches.

### 04.5 Уточнить границы storage/state
- В `state.rs` оставить state lifecycle (`load`, `persist`, dirty flags).
- В `storage.rs` оставить только JSON I/O и atomic write.
- В event reducer не делать storage I/O напрямую, только через state boundary.

Критерий готовности:
- Нету fs-операций в reducer path.

### 04.6 Обновить внешние слои
- `app.rs` заменить alias `quick_launches::event` на реэкспортированный API.
- `ui/widgets/quick_launches/*` и `ui/widgets/sidebar_workspace/*` перейти на re-exported type surface.
- `tab` и `explorer` использовать только `crate::features::quick_launches`.

Критерий готовности:
- `rg "features::quick_launches::(event|model|state|storage|errors)" otty/src` возвращает только внутренние файлы самой фичи.

### 04.7 Добавить strict-тесты
- model validation/normalization.
- state transitions для mutating событий.
- reducer success/ignored/failure.
- storage round-trip/corruption fallback.

Критерий готовности:
- Тесты deterministic, без сети.

### 04.8 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`

## Пример кода после рефакторинга

```rust
// otty/src/features/quick_launches/mod.rs
mod editor;
mod errors;
mod event;
mod model;
mod services;
mod state;
mod storage;

pub(crate) use errors::QuickLaunchError;
pub(crate) use event::{
    ContextMenuAction,
    QuickLaunchEvent,
    quick_launches_reducer,
};
pub(crate) use state::{QuickLaunchErrorState, QuickLaunchState};
```

```rust
// otty/src/features/quick_launches/event.rs
#[derive(Debug, Clone)]
pub(crate) enum QuickLaunchEvent {
    Ui(QuickLaunchUiEvent),
    SetupCompleted(QuickLaunchSetupOutcome),
    Tick,
}

pub(crate) fn quick_launches_reducer(
    state: &mut State,
    deps: QuickLaunchDeps<'_>,
    event: QuickLaunchEvent,
) -> Task<AppEvent> {
    match event {
        QuickLaunchEvent::Ui(ui_event) => reduce_ui(state, deps, ui_event),
        QuickLaunchEvent::SetupCompleted(outcome) => {
            reduce_setup_completed(state, outcome)
        },
        QuickLaunchEvent::Tick => reduce_tick(state),
    }
}
```
