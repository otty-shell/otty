# 01. Аудит `settings`

## Зависимости
- Выполнить `02-terminal.md` для переноса применения настроек терминала в terminal reducer API (без прямых мутаций из `app.rs`).

## Текущее состояние
- Канонический layout `settings` собран (`mod.rs`, `event.rs`, `state.rs`, `model.rs`, `errors.rs`, `storage.rs`).
- Primary contracts присутствуют: `SettingsState`, `SettingsEvent`, `settings_reducer`.
- Есть тесты для model/state/reducer/storage.

## Остаточные нарушения CONVENTIONS.md
- Раздел 7.3 (blocking I/O в reducer path):
  - `otty/src/features/settings/event.rs:34` вызывает sync `load_settings`.
  - `otty/src/features/settings/event.rs:37` вызывает sync `save_settings`.
- Раздел 6 (слишком широкая экспортная поверхность):
  - `otty/src/features/settings/mod.rs:10`
  - `otty/src/features/settings/mod.rs:11`
  Экспортируются дополнительные внутренние типы/утилиты вместо минимального стабильного API.
- Раздел 8 (обход reducer boundary для effects, связанных с settings):
  - `otty/src/app.rs:289` прямой вызов `terminal.apply_theme(...)`.
  - `otty/src/app.rs:291` прямые мутации терминальных вкладок после `SettingsApplied`.

## Детальный план до 100% strict

### 01.1 Перевести `Reload`/`Save` в async reducer flow
- Добавить событийные результаты (`Loaded`, `LoadFailed`, `Saved`, `SaveFailed`) в `SettingsEvent`.
- Использовать `Task::perform(...)` для чтения/записи через `storage.rs`.
- Оставить reducer синхронным и без прямого filesystem I/O.

Критерий готовности:
- В `settings_reducer` нет прямых вызовов sync storage операций.

### 01.2 Сузить публичную поверхность `settings/mod.rs`
- Оставить минимальный стабильный контракт:
  - `SettingsEvent`
  - `settings_reducer`
  - `SettingsState`
  - `SettingsError`
- Дополнительные типы (`SettingsNode`, `SettingsPreset`, `SettingsSection`, утилиты model) экспортировать только при подтвержденной необходимости и через отдельный API-слой.

Критерий готовности:
- `mod.rs` не реэкспортирует служебные детали без явной архитектурной причины.

### 01.3 Закрыть side-effect границу применения настроек
- Убрать прямые терминальные мутации из `app.rs`.
- Передавать применение темы/обновление shell session через typed события terminal feature.

Критерий готовности:
- `app.rs` не мутирует terminal internals напрямую в ветке `SettingsApplied`.

### 01.4 Дотестировать async/failure сценарии
- reducer: `given_save_failed_when_save_then_state_remains_dirty`.
- reducer: `given_load_failed_when_reload_then_state_preserved`.

Критерий готовности:
- Покрыты success/ignored/failure path после перехода на async flow.

### 01.5 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`
