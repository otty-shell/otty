# 01. Аудит `settings`

## Зависимости
- Нет. Это стартовая задача миграции.

## Текущее состояние (Legacy)
- `otty/src/features/settings/mod.rs:12` содержит бизнес-логику, события, состояние и редукцию через прямые методы состояния.
- В структуре фичи отсутствуют обязательные `event.rs` и `state.rs`.
- Внешний слой (`otty/src/app.rs:272`) напрямую мутирует `SettingsState`, обходя единый reducer-entrpoint.
- Нет тестов матрицы strict-профиля (model/state/reducer/storage).

## Нарушения CONVENTIONS.md
- Раздел 3 (Canonical layout): нет `event.rs`, `state.rs`.
- Раздел 4 (Responsibilities): `mod.rs` перегружен логикой.
- Раздел 5.1/5.2: нет явного `settings_reducer`, отсутствует единая внешняя точка записи.
- Раздел 6: `mod.rs` не является тонкой экспортной поверхностью.
- Раздел 10: нет обязательных тестов.

## Детальный план миграции

### 01.1 Создать каноническую структуру файлов
- Добавить `otty/src/features/settings/event.rs`.
- Добавить `otty/src/features/settings/state.rs`.
- Перенести `SettingsEvent`, `SettingsPreset`, `SettingsSection`, `SettingsNode`, `SettingsState` из `mod.rs` в `event.rs`/`state.rs` по ответственности.
- Оставить в `mod.rs` только declarations + curated re-exports.

Критерий готовности:
- `mod.rs` не содержит бизнес-логики и методов состояния.

### 01.2 Ввести единый reducer entrypoint
- Реализовать `pub(crate) fn settings_reducer(state: &mut State, event: SettingsEvent) -> Task<AppEvent>` в `event.rs`.
- Все ветки `SettingsEvent` должны обрабатываться только через reducer.
- Сохранение (`Save`) возвращает явный эффект (`Task<AppEvent>`) вместо скрытой логики в `app.rs`.

Критерий готовности:
- `app.rs` не мутирует `state.settings` напрямую в `handle_settings`.

### 01.3 Привести `mod.rs` к strict-экспорту
- Порядок модулей: `errors`, `event`, `model`, `state`, `storage`.
- Все внутренние declarations только приватные `mod ...;`.
- Переэкспорт только стабильных API:
  - `SettingsEvent`
  - `settings_reducer`
  - `SettingsState`
  - `SettingsError`

Критерий готовности:
- Нет `pub(crate) mod ...;` внутри `settings/mod.rs`.

### 01.4 Убрать протекание internals в UI
- Для `otty/src/ui/widgets/settings.rs:11` перейти на импорты только из `crate::features::settings` re-exports.
- Если UI нужны дополнительные типы (`SettingsNode`, `SettingsSection`, `SettingsPreset`) — добавить явные стабильные re-exports из `mod.rs`.

Критерий готовности:
- Нет импорта `crate::features::settings::<internal_module>::...`.

### 01.5 Разделить доменную и runtime-логику
- В `model.rs` оставить только доменные типы/валидацию/нормализацию.
- В `state.rs` оставить state-мутации и deterministic helper-методы.
- В `storage.rs` оставить только I/O и сериализацию.

Критерий готовности:
- Валидация цветов и нормализация настроек не зависят от `State`/UI.

### 01.6 Добавить строгие тесты
- `model.rs`: валидные/невалидные палитры и `normalized`.
- `state.rs`: переходы для каждого mutating-event.
- `event.rs`: success/ignored/failure path reducer.
- `storage.rs`: round-trip и corruption fallback.
- Именование тестов: `given_<context>_when_<action>_then_<outcome>`.

Критерий готовности:
- Все тесты deterministic и не требуют пользовательского окружения.

### 01.7 Финальная верификация
- Выполнить `cargo fmt`.
- Выполнить `cargo clippy --workspace --all-targets`.
- Выполнить `cargo test -p otty`.

## Пример кода после рефакторинга

```rust
// otty/src/features/settings/mod.rs
mod errors;
mod event;
mod model;
mod state;
mod storage;

pub(crate) use errors::SettingsError;
pub(crate) use event::{SettingsEvent, settings_reducer};
pub(crate) use state::SettingsState;
```

```rust
// otty/src/features/settings/event.rs
use iced::Task;

use crate::app::Event as AppEvent;
use crate::state::State;

#[derive(Debug, Clone)]
pub(crate) enum SettingsEvent {
    Save,
    Reset,
    NodePressed { path: Vec<String> },
    NodeHovered { path: Option<Vec<String>> },
    ShellChanged(String),
    EditorChanged(String),
    PaletteChanged { index: usize, value: String },
}

pub(crate) fn settings_reducer(
    state: &mut State,
    event: SettingsEvent,
) -> Task<AppEvent> {
    match event {
        SettingsEvent::Save => state.settings.persist_event(),
        SettingsEvent::Reset => {
            state.settings.reset();
            Task::none()
        },
        SettingsEvent::NodePressed { path } => {
            state.settings.select_path(&path);
            Task::none()
        },
        SettingsEvent::NodeHovered { path } => {
            state.settings.set_hovered_path(path);
            Task::none()
        },
        SettingsEvent::ShellChanged(value) => {
            state.settings.set_shell(value);
            Task::none()
        },
        SettingsEvent::EditorChanged(value) => {
            state.settings.set_editor(value);
            Task::none()
        },
        SettingsEvent::PaletteChanged { index, value } => {
            state.settings.set_palette_input(index, value);
            Task::none()
        },
    }
}
```
