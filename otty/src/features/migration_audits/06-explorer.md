# 06. Аудит `explorer`

## Зависимости
- Выполнить `02-terminal.md`.
- Выполнить `05-tab.md`.

## Текущее состояние (Legacy)
- В `otty/src/features/explorer/` отсутствует обязательный `model.rs`.
- `otty/src/features/explorer/mod.rs:1` использует public module declarations и не предоставляет curated re-exports.
- В `event.rs` кроме reducer экспортируются внешние mutating функции:
  - `sync_explorer_from_active_terminal` (`:51`)
  - `sync_explorer_from_terminal_event` (`:64`)
- В reducer/hot path есть blocking FS I/O:
  - `ExplorerState::refresh_tree` вызывает `read_dir_nodes` (`state.rs:89`, `:152`)
  - `toggle_folder` читает директорию синхронно (`state.rs:119`)
- Есть прямые зависимости от terminal internals:
  - `otty/src/features/explorer/event.rs:9` (`terminal::event::settings_for_session`)
  - `otty/src/features/explorer/event.rs:10` (`terminal::state::TerminalState`)
- Тесты strict-матрицы отсутствуют.

## Нарушения CONVENTIONS.md
- Раздел 3: отсутствует `model.rs`.
- Раздел 5.2: запись в feature state выполняется не только через reducer.
- Раздел 7.2: direct sibling internal imports.
- Раздел 7.3: blocking I/O в reducer/hot state path.
- Раздел 10: нет required tests.

## Детальный план миграции

### 06.1 Нормализовать структуру
- Добавить `otty/src/features/explorer/model.rs`.
- Перенести в `model.rs` доменные типы и чистые утилиты (`FileNode`, сортировка/compare logic).
- Оставить `state.rs` только для runtime state и deterministic мутаций.

Критерий готовности:
- Доменные структуры не живут в `state.rs`.

### 06.2 Ввести единый reducer write-boundary
- Убрать внешние `sync_explorer_from_*` функции из публичного API.
- Добавить явные события sync в `ExplorerEvent` (например, `SyncFromActiveTerminal`, `SyncFromTerminalEvent`).
- Синхронизацию делать только через `explorer_reducer`.

Критерий готовности:
- Внешняя запись explorer state происходит только через reducer.

### 06.3 Убрать blocking I/O из reducer path
- Вынести чтение файловой системы в async `services.rs` (optional strict extension).
- В reducer запускать `Task::perform(...)` и обрабатывать результат отдельным событием (`TreeLoaded`/`FolderLoaded`).
- `state.rs` оставляет только применение уже загруженных данных.

Критерий готовности:
- В `explorer_reducer` и `ExplorerState` нет прямых `std::fs::read_dir`.

### 06.4 Нормализовать cross-feature зависимости
- Использовать только re-exports terminal/tab features.
- Убрать импорты `terminal::event` и `terminal::state` internals.
- Если нужен доступ к cwd текущего терминала, добавить explicit API в terminal feature (через `mod.rs` re-export).

Критерий готовности:
- Нет `crate::features::terminal::event|state` импортов в explorer.

### 06.5 Привести `mod.rs` к strict
- Private module declarations: `errors`, `event`, `model`, `state`, `services` (если добавлен).
- Реэкспорт только `ExplorerEvent`, `explorer_reducer`, `ExplorerState`, `ExplorerError`.

Критерий готовности:
- Внешние импорты explorer идут только через `crate::features::explorer`.

### 06.6 Обновить callsites
- `tab`/`terminal` вместо прямых sync-функций explorer отправляют `AppEvent::Explorer(...)`.
- `ui/widgets/sidebar_workspace/explorer.rs` использует только re-exported типы.

Критерий готовности:
- Нет прямых вызовов helper-функций explorer извне.

### 06.7 Добавить strict-тесты
- model: сортировка/сравнение и path resolution.
- state: transitions для selected/hovered/tree apply.
- reducer: success/ignored/failure.
- services: загрузка директории, ошибки, fallback.

Критерий готовности:
- Полная deterministic матрица для explorer.

### 06.8 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`

## Пример кода после рефакторинга

```rust
// otty/src/features/explorer/mod.rs
mod errors;
mod event;
mod model;
mod services;
mod state;

pub(crate) use errors::ExplorerError;
pub(crate) use event::{ExplorerEvent, explorer_reducer};
pub(crate) use state::ExplorerState;
```

```rust
// otty/src/features/explorer/event.rs
#[derive(Debug, Clone)]
pub(crate) enum ExplorerEvent {
    NodePressed { path: TreePath },
    NodeHovered { path: Option<TreePath> },
    SyncFromActiveTerminal,
    SyncFromTerminal { tab_id: u64, terminal_id: u64 },
    TreeLoaded { root: PathBuf, nodes: Vec<FileNode> },
}

pub(crate) fn explorer_reducer(
    state: &mut State,
    deps: ExplorerDeps<'_>,
    event: ExplorerEvent,
) -> Task<AppEvent> {
    match event {
        ExplorerEvent::NodePressed { path } => reduce_node_pressed(state, deps, path),
        ExplorerEvent::NodeHovered { path } => {
            state.explorer.hovered = path;
            Task::none()
        },
        ExplorerEvent::SyncFromActiveTerminal => reduce_sync_from_active_terminal(state, deps),
        ExplorerEvent::SyncFromTerminal { tab_id, terminal_id } => {
            reduce_sync_from_terminal(state, deps, tab_id, terminal_id)
        },
        ExplorerEvent::TreeLoaded { root, nodes } => {
            state.explorer.apply_loaded_tree(root, nodes);
            Task::none()
        },
    }
}
```
