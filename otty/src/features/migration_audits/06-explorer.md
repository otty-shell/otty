# 06. Аудит `explorer`

## Зависимости
- Выполнить `05-tab.md`.
- Выполнить `02-terminal.md`.

## Текущее состояние
- Добавлен `model.rs`, доменные структуры вынесены из `state.rs`.
- Blocking чтение директории из reducer-path убрано: используется `Task::perform` + `services.rs`.
- Primary contracts присутствуют: `ExplorerState`, `ExplorerEvent`, `explorer_reducer`.

## Остаточные нарушения CONVENTIONS.md
- Раздел 3 (стабильный порядок модулей):
  - `otty/src/features/explorer/mod.rs:4`
  - `otty/src/features/explorer/mod.rs:5`
  Сейчас `services` объявлен раньше `state`.
- Раздел 6 (слишком широкая surface):
  - `otty/src/features/explorer/mod.rs:10`
  - `otty/src/features/explorer/mod.rs:13`
  Экспортируются `ExplorerDeps`, `ExplorerLoadTarget`, `FileNode` как часть внешнего API.
- Раздел 7.3 (циклическая зависимость с terminal):
  - `otty/src/features/explorer/event.rs:9` зависит от `terminal` helper API.
  - `otty/src/features/terminal/event.rs:9` зависит от `ExplorerEvent`.
- Раздел 8 (ownership/tab allocation leakage):
  - `otty/src/features/explorer/event.rs:216`
  - `otty/src/features/explorer/event.rs:217`
  Explorer reducer аллоцирует `tab_id/terminal_id`, что относится к tab/terminal boundaries.
- Раздел 8 (cross-feature read dependency в обход deps boundary):
  - `otty/src/features/explorer/event.rs:199`
  Чтение `state.settings.draft().terminal_editor()` напрямую из explorer reducer.

## Детальный план до 100% strict

### 06.1 Нормализовать `explorer/mod.rs`
- Порядок modules: `errors`, `event`, `model`, `state`, `services`.
- Сократить re-exports до primary boundary.

Критерий готовности:
- `explorer/mod.rs` следует canonical order и минимальному API.

### 06.2 Убрать циклическую связь с terminal
- Перенести доступ к cwd в `ExplorerDeps` (query API/trait), без прямой зависимости на terminal helpers из event.rs.
- События sync оставить в explorer boundary.

Критерий готовности:
- `explorer` не импортирует terminal-specific helper функции.

### 06.3 Вернуть ownership аллокации в tab/terminal
- В `open_file_in_editor` перестать выделять `tab_id/terminal_id` внутри explorer.
- Генерировать `TabOpenRequest`, который аллоцируется/разрешается в `tab_reducer`.

Критерий готовности:
- Explorer reducer не вызывает `state.allocate_tab_id()`/`state.allocate_terminal_id()`.

### 06.4 Передавать editor command через `ExplorerDeps`
- Параметр editor command передавать из app-layer через deps.
- Убрать прямое чтение settings-slice из explorer reducer.

Критерий готовности:
- В `explorer/event.rs` нет прямого доступа к `state.settings`.

### 06.5 Дотестировать новый orchestration flow
- reducer: sync from active/sync from terminal/failure path с deps query.
- integration: open file request строится без внутренней аллокации ids.

### 06.6 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`
