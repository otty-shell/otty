# 03. Аудит `quick_launches/editor` (subfeature)

## Зависимости
- Выполнить `04-quick-launches.md` (единая write-boundary quick launches и async persistence).
- Выполнить `05-tab.md` (стабильный tab boundary для editor-tab orchestration).

## Текущее состояние
- Subfeature уже приведен к canonical структуре: `mod.rs`, `event.rs`, `state.rs`, `model.rs`, `errors.rs`.
- Typed errors введены (`QuickLaunchEditorError`), строковые `Result<_, String>` убраны.
- Primary contracts присутствуют: `QuickLaunchEditorState`, `QuickLaunchEditorEvent`, `quick_launch_editor_reducer`.

## Остаточные нарушения CONVENTIONS.md
- Раздел 7.3 (blocking I/O в reducer path):
  - `otty/src/features/quick_launches/editor/event.rs:217`
  В save-path вызывается sync `state.quick_launches.persist()`.
- Раздел 8 (write-boundary quick_launches размыта):
  - `otty/src/features/quick_launches/editor/event.rs:145`
  - `otty/src/features/quick_launches/editor/event.rs:170`
  Editor reducer напрямую мутирует `state.quick_launches.data`.
- Раздел 8 (обход reducer boundary из `app.rs` для связанного UI state):
  - `otty/src/app.rs:163`
  - `otty/src/app.rs:166`
  - `otty/src/app.rs:263`
  Inline-edit state закрывается прямой мутацией, не через feature events.

## Детальный план до 100% strict

### 03.1 Убрать sync persistence из editor reducer
- Перевести сохранение в async flow через события quick_launches feature.
- Editor reducer должен только валидировать draft и формировать команду/intent.

Критерий готовности:
- В `quick_launches/editor/event.rs` нет прямых вызовов `persist()`.

### 03.2 Ограничить editor reducer ответственностью subfeature
- Прекратить прямые записи в `state.quick_launches.data`.
- Передавать результат через typed событие в `quick_launches_reducer`.

Критерий готовности:
- Мутации quick launches дерева выполняются только в `quick_launches_reducer`.

### 03.3 Закрыть внешние обходы UI state
- Добавить explicit события `QuickLaunchEvent` для cancel/close inline edit.
- Перевести текущие мутации из `app.rs` в событийный маршрут.

Критерий готовности:
- `app.rs` не пишет напрямую в editor/quick_launches UI state.

### 03.4 Расширить тесты на новый flow
- reducer: success/ignored/failure path при save-intent.
- интеграция editor -> quick_launches: корректное обновление дерева и ошибок.

### 03.5 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`
