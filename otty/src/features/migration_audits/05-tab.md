# 05. Аудит `tab`

## Зависимости
- Нет. Это базовая задача для развязки feature ownership.

## Текущее состояние
- `TabState` выделен в отдельный файл (`state.rs`) и подключен в `State`.
- Primary contracts присутствуют: `TabState`, `TabEvent`, `tab_reducer`.
- Прямые импорты sibling internals (`::event::...`, `::state::...`) устранены.

## Остаточные нарушения CONVENTIONS.md
- Раздел 8 (ownership нарушен):
  - `otty/src/features/terminal/event.rs:375`
  - `otty/src/features/terminal/event.rs:383`
  Terminal reducer напрямую мутирует `state.tab`, хотя tab — канонический owner.
- Раздел 8/7 (tab reducer напрямую запускает sibling reducer):
  - `otty/src/features/tab/event.rs:155`
  - `otty/src/features/tab/event.rs:196`
  - `otty/src/features/tab/event.rs:264`
  `tab_reducer` вызывает `terminal::terminal_reducer(...)` напрямую вместо событийного обмена.
- Раздел 7.3 (циклы feature dependency):
  - `tab -> terminal` (`otty/src/features/tab/event.rs:10`)
  - `terminal -> tab` (`otty/src/features/terminal/event.rs:10`)
  - `tab -> quick_launches` (`otty/src/features/tab/model.rs:3-4`)
  - `quick_launches -> tab` (`otty/src/features/quick_launches/event.rs:12`)
- Раздел 10 (testing matrix):
  - `otty/src/features/tab/model.rs` не содержит model-level тестов.

## Детальный план до 100% strict

### 05.1 Зафиксировать tab как единственного owner `TabState`
- Убрать любые записи в `state.tab` из sibling features.
- Все операции open/activate/close табов оставить только в `tab_reducer`.

Критерий готовности:
- В других feature нет `state.tab.insert/remove/activate`.

### 05.2 Перевести межфичевую оркестрацию на `Task<AppEvent>`
- Вместо прямого вызова `terminal_reducer` в tab возвращать `Task::done(AppEvent::Terminal(...))`.
- Сохранить детерминированный порядок через `Task::batch`.

Критерий готовности:
- `tab/event.rs` не вызывает sibling reducer-функции напрямую.

### 05.3 Развязать модель tab от heavy sibling payload
- Пересмотреть `TabOpenRequest`:
  - исключить прямую зависимость от `QuickLaunch`/`TerminalState` где возможно;
  - использовать boundary DTO + идентификаторы.

Критерий готовности:
- `tab/model.rs` не тянет детальные типы sibling features без необходимости.

### 05.4 Дополнить tests matrix
- model: тесты `TabOpenRequest`/`TabContent` invariants и `Debug` контракта.
- reducer: отдельный failure path (например, недоступный terminal/open flow).

Критерий готовности:
- Для `tab` есть model/state/reducer success/ignored/failure coverage.

### 05.5 Финальная верификация
- `cargo fmt`
- `cargo clippy --workspace --all-targets`
- `cargo test -p otty`
