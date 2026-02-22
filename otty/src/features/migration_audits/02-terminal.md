# 02. Аудит `terminal`

## Зависимости
- Выполнить `05-tab.md` (фикс ownership tab-слайса и развязка reducer-границ).

## Текущее состояние
- Канонический набор файлов есть: `errors.rs`, `event.rs`, `model.rs`, `state.rs`, `services.rs`.
- Primary contracts присутствуют: `TerminalState`, `TerminalEvent`, `terminal_reducer`.
- `error.rs`/`shell.rs` legacy больше нет.
- Есть тесты для model/state/event/services.

## Остаточные нарушения CONVENTIONS.md
- Раздел 3 (стабильный порядок модулей):
  - `otty/src/features/terminal/mod.rs:4`
  - `otty/src/features/terminal/mod.rs:5`
  Сейчас `services` объявлен раньше `state`.
- Раздел 6 (минимальная экспортная поверхность):
  - `otty/src/features/terminal/mod.rs:9`
  - `otty/src/features/terminal/mod.rs:13`
  - `otty/src/features/terminal/mod.rs:14`
  Реэкспортируется большой объем вспомогательного API (model/services/helpers), а не только primary boundary.
- Раздел 8 (соседняя feature-state мутируется напрямую):
  - `otty/src/features/terminal/event.rs:375`
  - `otty/src/features/terminal/event.rs:383`
  `terminal_reducer` пишет в `state.tab`.
- Раздел 7.3 (циклические зависимости между feature-модулями):
  - `otty/src/features/terminal/event.rs:9-10` (зависит от `explorer` и `tab`).
  - `otty/src/features/tab/event.rs:10` (зависит от `terminal`).
  - `otty/src/features/explorer/event.rs:9` (зависит от `terminal`).
- Раздел 8 (обход reducer boundary из `app.rs`):
  - `otty/src/app.rs:289` прямой `terminal.apply_theme(...)`.
  - `otty/src/app.rs:722` прямой `terminal.close_context_menu()`.

## Детальный план до 100% strict

### 02.1 Нормализовать `mod.rs`
- Порядок declarations: `errors`, `event`, `model`, `state`, `services`.
- Оставить в re-export только стабилизированный boundary; helper API вынести в отдельные целевые фасады.

Критерий готовности:
- `terminal/mod.rs` соответствует canonical order и узкой surface.

### 02.2 Вернуть ownership табов в `tab` feature
- Убрать записи в `state.tab` из `terminal_reducer` (`InsertTab` flow).
- Терминал должен оперировать только terminal-состоянием уже созданной вкладки.

Критерий готовности:
- В `terminal/event.rs` нет `state.tab.insert(...)`/`state.tab.activate(...)`.

### 02.3 Разорвать циклы `terminal <-> tab` и `terminal <-> explorer`
- Заменить прямые cross-reducer вызовы на событийный обмен через `Task<AppEvent>`.
- Синхронизацию explorer от terminal перевести на API уровня app orchestration/deps, без двунаправленных ссылок feature-to-feature.

Критерий готовности:
- Граф импортов features ацикличен.

### 02.4 Закрыть app-level мутации terminal internals
- Ввести typed terminal events для применения темы и закрытия context menu.
- `app.rs` должен только диспатчить события terminal feature.

Критерий готовности:
- В `app.rs` нет прямых вызовов методов `TerminalState`.

### 02.5 Дотестировать новые границы
- reducer: success/ignored/failure path после переноса tab ownership.
- integration: корректный focus/sync при открытии/закрытии вкладок через tab feature.

### 02.6 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`
