# 03. Аудит `quick_launches/editor` (subfeature)

## Зависимости
- Выполнить `02-terminal.md` (стабилизировать общие tab/terminal контракты, используемые при открытии вкладок редактора).

## Текущее состояние (Legacy)
- Структура subfeature не каноническая: нет `state.rs` и `errors.rs`.
- `otty/src/features/quick_launches/editor/model.rs:62` хранит runtime state (`QuickLaunchEditorState`) в `model.rs`.
- `otty/src/features/quick_launches/editor/event.rs:173` и `:195` экспортируют внешние mutating API (`open_create_editor_tab`, `open_edit_editor_tab`) в обход reducer-границы.
- Ошибки сохранения представлены строками (`Result<(), String>`), нет typed error boundary.
- Тесты strict-матрицы отсутствуют.

## Нарушения CONVENTIONS.md
- Раздел 3/4: subfeature не зеркалит canonical contract (`event/state/model/errors`).
- Раздел 5.2: запись в состояние происходит не только через reducer.
- Раздел 7: внешний слой зависит от editor internals (`crate::features::quick_launches::editor::...` пакетно, без тонкого API subfeature).
- Раздел 10: нет детерминированных тестов.

## Детальный план миграции

### 03.1 Нормализовать структуру subfeature
- Добавить `otty/src/features/quick_launches/editor/state.rs`.
- Добавить `otty/src/features/quick_launches/editor/errors.rs`.
- Перенести `QuickLaunchEditorState`, `QuickLaunchEditorMode` и опции из `model.rs` в `state.rs`.
- В `model.rs` оставить только чистые преобразования/валидацию draft -> domain model.

Критерий готовности:
- В `model.rs` нет runtime-состояния UI.

### 03.2 Ввести typed ошибки
- Заменить `Result<_, String>` на `Result<_, QuickLaunchEditorError>`.
- Вынести все сообщения в `errors.rs`.
- В reducer на границе UI преобразовывать ошибку в отображаемое сообщение.

Критерий готовности:
- В editor reducer нет строковых ad-hoc ошибок в API.

### 03.3 Оставить один внешний write-entrypoint
- Оставить публично только `quick_launch_editor_reducer`.
- `open_create_editor_tab` и `open_edit_editor_tab` убрать из editor API:
  - либо перенести в `tab` feature,
  - либо заменить на `TabEvent::NewTab { request: ... }` фабрики в `tab`.

Критерий готовности:
- Внешние модули не вызывают editor-функции, которые напрямую мутируют `State`.

### 03.4 Привести `editor/mod.rs` к strict-профилю
- private `mod errors; mod event; mod model; mod state;`
- Явные re-exports:
  - `QuickLaunchEditorEvent`
  - `quick_launch_editor_reducer`
  - `QuickLaunchEditorState`
  - `QuickLaunchEditorError`

Критерий готовности:
- Нет wildcard/public module exports.

### 03.5 Обновить внешние импорты
- `app.rs`, `tab`, `ui/widgets/quick_launches/editor.rs` переключить на API из `crate::features::quick_launches::editor` re-exports.
- Исключить доступ к `editor::model`/`editor::event` internals снаружи.

Критерий готовности:
- `rg "features::quick_launches::editor::(event|model|state|errors)" otty/src` не возвращает внешних импортов.

### 03.6 Добавить strict-тесты subfeature
- model: валидация draft/конвертация в `QuickLaunch`.
- state: mutating transitions для каждого `QuickLaunchEditorEvent`.
- reducer: success/ignored/failure.

Критерий готовности:
- Тесты deterministic, naming по `given_when_then`.

### 03.7 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`

## Пример кода после рефакторинга

```rust
// otty/src/features/quick_launches/editor/mod.rs
mod errors;
mod event;
mod model;
mod state;

pub(crate) use errors::QuickLaunchEditorError;
pub(crate) use event::{QuickLaunchEditorEvent, quick_launch_editor_reducer};
pub(crate) use state::{QuickLaunchEditorMode, QuickLaunchEditorState};
```

```rust
// otty/src/features/quick_launches/editor/event.rs
pub(crate) fn quick_launch_editor_reducer(
    state: &mut State,
    tab_id: u64,
    event: QuickLaunchEditorEvent,
) -> Task<AppEvent> {
    let Some(editor) = editor_mut(state, tab_id) else {
        return Task::none();
    };

    match event {
        QuickLaunchEditorEvent::Save => {
            let draft = editor.clone();
            match apply_save(state, draft) {
                Ok(()) => Task::done(AppEvent::Tab(TabEvent::CloseTab { tab_id })),
                Err(err) => {
                    if let Some(editor) = editor_mut(state, tab_id) {
                        editor.error = Some(format!("{err}"));
                    }
                    Task::none()
                },
            }
        },
        other => reduce_editor_fields(editor, other),
    }
}
```
