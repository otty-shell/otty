# 02. Аудит `terminal`

## Зависимости
- Выполнить `01-settings.md` (стабилизировать контракт настроек, которые участвуют в terminal deps).

## Текущее состояние (Legacy)
- В `otty/src/features/terminal/mod.rs:1` используется `error.rs` (singular) и `shell.rs`, что нарушает strict-layout.
- `otty/src/features/terminal/event.rs:55` экспортирует reducer под именем `terminal_tab_reducer`, а также несколько дополнительных внешних mutating API (`internal_widget_event_reducer`, `insert_terminal_tab`, `focus_active_terminal`, `sync_tab_block_selection`, `settings_for_session`).
- Внешние модули импортируют internals terminal напрямую:
  - `otty/src/app.rs:21` (`terminal::event::{...}`)
  - `otty/src/app.rs:22` (`terminal::model::ShellSession`)
  - `otty/src/app.rs:23` (`terminal::shell::...`)
  - `otty/src/features/explorer/event.rs:9`
  - `otty/src/features/quick_launches/event.rs:18`
  - `otty/src/features/tab/event.rs:10`
- Тестовая матрица strict отсутствует.

## Нарушения CONVENTIONS.md
- Раздел 3: `error.rs` вместо `errors.rs`; `shell.rs` вне canonical/optional (`services.rs`).
- Раздел 5.1/5.2: нет единственной внешней точки записи `terminal_reducer`.
- Раздел 6/7: межфичевые зависимости идут через внутренние подмодули terminal.
- Раздел 10: нет feature-level тестов reducer/state/model.

## Детальный план миграции

### 02.1 Привести layout к strict
- Переименовать `error.rs` -> `errors.rs`.
- Перенести `shell.rs` -> `services.rs` (интеграции shell остаются там).
- Обновить импорты внутри feature.

Критерий готовности:
- В директории terminal остаются только канонические/optional файлы.

### 02.2 Ввести единый reducer API
- Переименовать `terminal_tab_reducer` -> `terminal_reducer`.
- Свернуть внешние mutating функции в событийный контракт:
  - `internal_widget_event_reducer` -> `TerminalEvent::Widget(...)`
  - `insert_terminal_tab` -> `TerminalEvent::InsertTab(...)`
  - `sync_tab_block_selection` -> `TerminalEvent::SyncSelection { tab_id }`
- Внешне оставить только `TerminalEvent` + `terminal_reducer`.

Критерий готовности:
- У terminal один внешний reducer-entrpoint.

### 02.3 Стабилизировать публичную поверхность `mod.rs`
- Использовать только private module declarations.
- Реэкспортировать только стабильные типы:
  - `TerminalEvent`
  - `terminal_reducer`
  - `TerminalState`
  - `TerminalError`
  - необходимые публичные domain-типы из `model.rs` (`ShellSession`, `TerminalKind`, `TerminalEntry` при необходимости UI).

Критерий готовности:
- Никакой внешний код не импортирует `crate::features::terminal::event|state|model|services` напрямую.

### 02.4 Вынести shell-интеграции в сервисный слой
- В `services.rs` оставить только взаимодействие с FS/env/shell wrapper scripts.
- Описать явный API (`setup_shell_session_with_shell`, `fallback_shell_session_with_shell`) как сервисные функции.

Критерий готовности:
- В `event.rs` нет прямого FS/env I/O.

### 02.5 Обновить callsites
- `app.rs` и sibling features (`tab`, `explorer`, `quick_launches`) переключить на `crate::features::terminal` re-exports.
- `ui/widgets/terminal/*` переключить на re-exported типы и события из `terminal/mod.rs`.

Критерий готовности:
- `rg "features::terminal::(event|state|model|services|errors|error)" otty/src` не находит внешних импортов.

### 02.6 Упорядочить visibility в `state.rs`
- Снизить `pub` до `pub(crate)` там, где методы не должны быть публичным контрактом.
- Добавить doc-комментарии для оставшихся публичных API.

Критерий готовности:
- Публичная поверхность `TerminalState` минимальна и осмысленна.

### 02.7 Добавить strict-тесты
- reducer: success/ignored/failure path.
- state: split/close/focus/selection transitions.
- model: корректность `TerminalKind`, `BlockSelection` и контрактов `ShellSession`.
- services: fallback/setup сценарии (через temp dir).

Критерий готовности:
- Тесты deterministic, без сети.

### 02.8 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`

## Пример кода после рефакторинга

```rust
// otty/src/features/terminal/mod.rs
mod errors;
mod event;
mod model;
mod services;
mod state;

pub(crate) use errors::TerminalError;
pub(crate) use event::{TerminalEvent, terminal_reducer};
pub(crate) use model::{ShellSession, TerminalEntry, TerminalKind};
pub(crate) use services::{
    fallback_shell_session_with_shell,
    setup_shell_session_with_shell,
};
pub(crate) use state::TerminalState;
```

```rust
// otty/src/features/terminal/event.rs
pub(crate) fn terminal_reducer(
    state: &mut State,
    event: TerminalEvent,
) -> Task<AppEvent> {
    match event {
        TerminalEvent::TabEvent { tab_id, action } => {
            reduce_tab_event(state, tab_id, action)
        },
        TerminalEvent::Widget(widget_event) => {
            reduce_widget_event(state, widget_event)
        },
        TerminalEvent::SyncSelection { tab_id } => {
            sync_tab_block_selection(state, tab_id)
        },
    }
}
```
