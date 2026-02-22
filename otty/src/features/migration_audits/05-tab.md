# 05. Аудит `tab`

## Зависимости
- Выполнить `02-terminal.md`.
- Выполнить `04-quick-launches.md`.

## Текущее состояние (Legacy)
- Структура неполная: отсутствует `state.rs` (`otty/src/features/tab/` содержит только `event.rs`, `model.rs`, `mod.rs`).
- `otty/src/features/tab/mod.rs:1` использует `pub(crate) mod ...` вместо private declarations.
- `otty/src/features/tab/event.rs` зависит от sibling internals:
  - `quick_launches::editor` (`:6`)
  - `quick_launches::model` (`:9`, `:214`)
  - `terminal::event|model|state` (`:10-12`)
  - `explorer::event::sync_explorer_from_*` (`:92`, `:164`, `:177`)
- В reducer происходят записи в другие feature slices (`settings.reload`, `explorer sync`) напрямую.
- Нет strict-тестов.

## Нарушения CONVENTIONS.md
- Раздел 3: отсутствует обязательный `state.rs`.
- Раздел 6/7: direct imports sibling internals.
- Раздел 8: нет явного владельца tab-состояния как отдельного feature state.
- Раздел 10: отсутствуют reducer/state/model tests.

## Детальный план миграции

### 05.1 Ввести `TabState` и ownership таб-домена
- Создать `otty/src/features/tab/state.rs`.
- Перенести из `otty/src/state.rs` tab-данные в `TabState`:
  - `active_tab_id`
  - `tab_items`
  - tab-счетчики (`next_tab_id`)
- Явно определить границы владения (ID терминалов может остаться у terminal feature, но tab ownership должен быть формализован).

Критерий готовности:
- Табовые данные доступны через `state.tab` (или эквивалентный feature slice), а не плоско в `State`.

### 05.2 Привести `mod.rs` к strict API
- Private module declarations (`event`, `model`, `state`, `errors` при необходимости).
- Реэкспорт только стабильных контрактов:
  - `TabEvent`
  - `tab_reducer`
  - `TabState`
  - минимально необходимые доменные типы (`TabContent`, `TabOpenRequest`, `TabItem`) при обосновании.

Критерий готовности:
- Нет `pub(crate) mod ...` в `tab/mod.rs`.

### 05.3 Убрать sibling-internal зависимости
- Заменить импорты в `tab/event.rs` на API через `crate::features::{quick_launches, terminal, explorer}` re-exports.
- Запретить `crate::features::<other>::event|state|model` в tab feature.

Критерий готовности:
- `rg "crate::features::[a-z_]+::(event|state|model|storage|errors)" otty/src/features/tab` возвращает пусто.

### 05.4 Формализовать runtime-зависимости reducer
- Ввести `TabDeps<'a>` вместо длинного списка параметров.
- Явно передавать внешние зависимости (`terminal_settings`, `shell_session`, возможно feature adapters).

Критерий готовности:
- Сигнатура reducer соответствует strict-стилю с `deps`.

### 05.5 Убрать прямую мутацию sibling state
- Вместо прямых `explorer::event::sync_explorer_from_*` возвращать `Task<AppEvent>` для событий explorer.
- Аналогично для settings reload/open flows: только через события соответствующей фичи.

Критерий готовности:
- tab reducer не вызывает напрямую функции, мутирующие state другой фичи.

### 05.6 Очистить `TabOpenRequest` от внутренних типов sibling features
- По возможности заменить payload-и из внутренних типов (`QuickLaunch`, `TerminalState`) на boundary DTO или re-exported stable types.
- Для тяжелых payload использовать feature-prefixed request structs.

Критерий готовности:
- `tab/model.rs` не импортирует sibling internals напрямую.

### 05.7 Добавить strict-тесты
- model: корректность `TabOpenRequest`/`Debug`.
- state: activate/close/open transitions.
- reducer: success/ignored/failure path.

Критерий готовности:
- Все mutating события покрыты deterministic тестами.

### 05.8 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`

## Пример кода после рефакторинга

```rust
// otty/src/features/tab/mod.rs
mod event;
mod model;
mod state;

pub(crate) use event::{TabEvent, tab_reducer};
pub(crate) use model::{TabContent, TabItem, TabOpenRequest};
pub(crate) use state::TabState;
```

```rust
// otty/src/features/tab/event.rs
pub(crate) struct TabDeps<'a> {
    pub(crate) terminal_settings: &'a Settings,
    pub(crate) shell_session: &'a ShellSession,
}

pub(crate) fn tab_reducer(
    state: &mut State,
    deps: TabDeps<'_>,
    event: TabEvent,
) -> Task<AppEvent> {
    match event {
        TabEvent::NewTab { request } => reduce_open(state, deps, request),
        TabEvent::ActivateTab { tab_id } => reduce_activate(state, tab_id),
        TabEvent::CloseTab { tab_id } => reduce_close(state, tab_id),
    }
}
```
