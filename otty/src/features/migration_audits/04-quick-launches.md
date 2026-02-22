# 04. Аудит `quick_launches`

## Зависимости
- Выполнить `05-tab.md`.
- Выполнить `02-terminal.md`.

## Текущее состояние
- Feature приведена к canonical layout (`errors/event/model/state/storage/services/editor`).
- Введен единый внешний reducer `quick_launches_reducer` и `QuickLaunchSetupOutcome`.
- Прямые импорты terminal internals устранены (используются re-exports).

## Остаточные нарушения CONVENTIONS.md
- Раздел 7.3 (blocking I/O в reducer path):
  - `otty/src/features/quick_launches/event.rs:849`
  - `otty/src/features/quick_launches/event.rs:856`
  Persist выполняется синхронно внутри reducer flow.
- Раздел 6 (API surface слишком широкая):
  - `otty/src/features/quick_launches/mod.rs:16-33`
  Экспортируются внутренние state/details (`LaunchInfo`, `ContextMenuState`, `InlineEditState` и др.).
- Раздел 8 (обход reducer boundary из `app.rs`):
  - `otty/src/app.rs:632-635`
  - `otty/src/app.rs:717`
  - `otty/src/app.rs:166`
  Прямые мутации quick launches internals (`hovered`, `pressed`, `drag`, `drop_target`, `context_menu`, `inline_edit`).
- Раздел 7.3 (циклические зависимости):
  - `otty/src/features/quick_launches/event.rs:12` -> зависимость от `tab`.
  - `otty/src/features/tab/model.rs:3-7` -> зависимость от quick_launches/terminal типов.

## Детальный план до 100% strict

### 04.1 Перевести persistence в async эффекты
- Заменить sync `persist_quick_launches` на `Task::perform` с событиями результата.
- Reducer должен оставаться без blocking filesystem операций.

Критерий готовности:
- В `quick_launches/event.rs` нет sync save I/O в hot path.

### 04.2 Закрыть app-level прямые мутации
- Добавить explicit события quick_launches для:
  - сброса drag/hover/pressed;
  - закрытия context menu;
  - отмены inline edit.
- Использовать их в `app.rs` вместо прямых присваиваний.

Критерий готовности:
- `app.rs` не пишет напрямую в `state.quick_launches.*`.

### 04.3 Сузить `quick_launches/mod.rs` surface
- Оставить primary boundary (`QuickLaunchEvent`, `quick_launches_reducer`, `QuickLaunchState`, `QuickLaunchError`) и минимально необходимые view/domain DTO.
- Убрать реэкспорт внутренних transient-типов состояния.

Критерий готовности:
- Внешний код не зависит от внутренних state-деталей quick launches.

### 04.4 Разорвать цикл `quick_launches <-> tab`
- Убрать взаимозависимость от тяжелых типов в `TabOpenRequest`.
- Передавать команды через boundary DTO/ID + события.

Критерий готовности:
- Нет двунаправленных импортов между `tab` и `quick_launches`.

### 04.5 Дотестировать async/failure сценарии
- reducer: save fail / save success / stale setup completion.
- storage: round-trip/corruption (сохранить существующее покрытие).

### 04.6 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`
