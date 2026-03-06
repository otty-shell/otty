# PRD: Декомпозиция `otty/src/view.rs` по виджетам

## 1. Цель документа

Зафиксировать детальный план рефакторинга [`otty/src/view.rs`](/home/kemokempo/projects/main/otty/otty/src/view.rs), чтобы:

1. Уменьшить ответственность root-view и оставить в нём только orchestration.
2. Перенести локальную UI-логику в уже существующие виджеты там, где это архитектурно корректно.
3. Выделить новые виджеты там, где текущие границы модулей не подходят.
4. Явно зафиксировать модули, которые нужно оставить без изменений.

---

## 2. Проблема текущего состояния

Файл [`otty/src/view.rs`](/home/kemokempo/projects/main/otty/otty/src/view.rs) (около 500 строк) содержит одновременно:

1. Root-level orchestration (stack слоёв, глобальные overlays, resize grips).
2. Логику конкретных UI-областей (header, sidebar split layout, tab area).
3. Доменно-специфичные overlays (sidebar add menu, terminal pane context menu).
4. Маршрутизацию активного tab content по разным подсистемам.

Это создаёт:

1. Сильную связанность root с внутренними деталями нескольких виджетов.
2. Усложнение поддержки и тестирования.
3. Рост числа регрессий при изменениях любого локального view-компонента.

---

## 3. Принципы декомпозиции

1. `view.rs` на root уровне должен собирать экран из крупных блоков, но не рисовать локальные компоненты.
2. Локальный UI переносится в виджет, который владеет соответствующим `Intent`/state.
3. Если view-композиция требует данных сразу из нескольких виджетов и не принадлежит ни одному из них, создаётся отдельный widget-aggregator.
4. Event mapping в `AppEvent` остаётся в root, чтобы не ломать текущий event-bus.
5. Никакой бизнес-логики редьюсеров в рамках этого PRD не переносится.

---

## 4. Матрица миграции: что куда переносить

| Текущее место в `view.rs` | Назначение | Решение | Целевой модуль |
|---|---|---|---|
| `view_header` | Header-композиция (`action_bar` + separator) | Перенести в существующий `chrome` widget view | `otty/src/widgets/chrome/view/header.rs` (новый файл в существующем виджете) |
| `view_add_menu_overlay`, `add_menu_item` | Sidebar add-menu overlay | Перенести в существующий `sidebar` widget view | `otty/src/widgets/sidebar/view/add_menu_overlay.rs` (новый файл в существующем виджете) |
| `view_terminal_context_menu_overlay` | Поиск открытого context menu среди terminal tabs + render overlay | Перенести в существующий `terminal_workspace` widget view | `otty/src/widgets/terminal_workspace/view/context_menu_overlay.rs` (новый файл в существующем виджете) |
| `view_sidebar_layout`, `view_workspace_content` | Кросс-виджетный layout: rail + pane split + workspace panel + tab area | Нужен новый виджет-агрегатор | `otty/src/widgets/workspace/*` (новый виджет) |
| `view_tab_area`, `view_tab_content`, `missing_tab_state` | Tab bar + routing активного контента в Terminal/Settings/Wizard/Error | Перенести в новый виджет-агрегатор (не в `tabs`, чтобы не смешивать границы) | `otty/src/widgets/workspace/view/tab_area.rs` |
| Root `view()` | Финальная сборка слоёв, глобальные блокировки, resize grips | Оставить на root уровне | `otty/src/view.rs` |

---

## 5. Что переносим в существующие виджеты

## 5.1 Chrome widget

### Текущее состояние

1. [`otty/src/widgets/chrome/view/action_bar.rs`](/home/kemokempo/projects/main/otty/otty/src/widgets/chrome/view/action_bar.rs) уже рисует action bar.
2. Separator под header рисуется вне виджета в root.

### План

1. Добавить `chrome/view/header.rs`, который композирует:
   1. `action_bar::view(...)`
   2. separator container с `HEADER_SEPARATOR_HEIGHT`.
2. Экспортировать модуль в [`chrome/view/mod.rs`](/home/kemokempo/projects/main/otty/otty/src/widgets/chrome/view/mod.rs).
3. Из root вызывать один метод `chrome::view::header::view(...)`.

### Границы ответственности после миграции

1. `chrome` полностью владеет визуальной реализацией window header.
2. Root знает только, что `chrome` возвращает `Element<ChromeIntent>`.

---

## 5.2 Sidebar widget

### Текущее состояние

1. Sidebar rail уже живёт в [`widgets/sidebar/view/mod.rs`](/home/kemokempo/projects/main/otty/otty/src/widgets/sidebar/view/mod.rs).
2. Add-menu overlay логически относится к sidebar, но живёт в root `view.rs`.

### План

1. Добавить `sidebar/view/add_menu_overlay.rs`:
   1. `view_add_menu_overlay(...)`
   2. внутренний helper `add_menu_item(...)`
2. Убрать дублирование intent-нейминга:
   1. Оставить только один dismiss intent (`DismissAddMenu` или `AddMenuDismiss`, выбрать единый).
   2. Привести `guards.rs` и reducer к единому варианту.
3. Root получает overlay из `sidebar::view`, но по-прежнему мапит его в `AppEvent::Sidebar(...)`.

### Границы ответственности после миграции

1. Все sidebar-specific overlays живут в sidebar widget.
2. Root больше не знает деталей размеров/паддингов sidebar add menu.

---

## 5.3 Terminal workspace widget

### Текущее состояние

1. [`pane_context_menu.rs`](/home/kemokempo/projects/main/otty/otty/src/widgets/terminal_workspace/view/pane_context_menu.rs) уже умеет рисовать меню конкретной панели.
2. Логика "найти первую открытую terminal context menu среди tab'ов" находится в root.

### План

1. Добавить `terminal_workspace/view/context_menu_overlay.rs`:
   1. функция, принимающая iterator/VM терминальных tab states;
   2. возвращает `Option<Element<TerminalWorkspaceIntent>>`.
2. Root вызывает этот API и только мапит события в `AppEvent`.

### Границы ответственности после миграции

1. `terminal_workspace` владеет и компонентом меню, и стратегией выбора активного overlay.
2. Root не итерирует `terminal_workspace.tabs()` напрямую для UI-деталей.

---

## 6. Какие новые виджеты создать

## 6.1 Новый виджет `workspace` (обязательный)

### Почему нужен новый, а не перенос в существующие

`view_sidebar_layout` и `view_tab_content` работают сразу с несколькими подсистемами:

1. `sidebar`
2. `tabs`
3. `terminal_workspace`
4. `quick_launch`
5. `explorer`
6. `settings`

Если переносить это в любой один существующий виджет, он станет знать слишком много о чужих границах.

### Цель нового виджета

Сделать отдельный UI-агрегатор рабочей области (workspace shell), который:

1. Рендерит layout `sidebar rail + split pane + tab area`.
2. Роутит активный tab content в нужный контент-блок.
3. Возвращает события в виде `WorkspaceIntent`, а root делает единую трансляцию в `AppEvent`.

### Предлагаемая структура файлов

1. `otty/src/widgets/workspace/mod.rs`
2. `otty/src/widgets/workspace/event.rs`
3. `otty/src/widgets/workspace/model.rs`
4. `otty/src/widgets/workspace/view/mod.rs`
5. `otty/src/widgets/workspace/view/sidebar_layout.rs`
6. `otty/src/widgets/workspace/view/tab_area.rs`

### Минимальный контракт `WorkspaceViewModel`

1. `sidebar_vm` (из sidebar widget)
2. `tabs_vm` (из tabs widget)
3. `active_tab_content`
4. срезы/refs для terminal/quick_launch/explorer/settings, необходимые только для render

### Минимальный контракт событий

`WorkspaceIntent` должен быть UI-уровнем, без бизнес-логики:

1. `Sidebar(SidebarIntent)`
2. `Tabs(TabsIntent)`
3. `QuickLaunch(QuickLaunchIntent)`
4. `Explorer(ExplorerIntent)`
5. `Settings(SettingsIntent)`
6. `TerminalWorkspace(TerminalWorkspaceIntent)`

Root делает `map` в `AppEvent` и оставляет существующий dispatcher в `events/*`.

---

## 7. Что оставить на root уровне

В [`otty/src/view.rs`](/home/kemokempo/projects/main/otty/otty/src/view.rs) после рефакторинга оставить только app-shell обязанности:

1. Инициализация `theme_props`.
2. Композиция верхнего layout:
   1. header layer (из `chrome`),
   2. workspace layer (из `workspace`),
   3. overlay layer stack.
3. Глобальный `Stack`-порядок слоёв.
4. Window-level resize grips.
5. Глобальная блокировка resize grips, когда открыт любой overlay.

Root не должен:

1. Строить конкретные меню-элементы.
2. Выбирать internal UI-контент по типам tab'ов.
3. Знать точные размеры/стили внутренних панелей виджетов.

---

## 8. Какие существующие виджеты оставить в покое

В рамках этого PRD не менять доменную логику и reducer-потоки следующих виджетов:

1. `quick_launch` (кроме адаптации публичного view API, если понадобится).
2. `explorer` (без изменения внутренней tree-логики).
3. `settings` (без изменения form-редьюсера и storage).
4. `tabs` reducer/state (кроме опционального расширения view-model для workspace render).
5. `terminal_workspace` reducer/state (кроме view-level overlay API).
6. `events/*` маршрутизация эффектов (кроме точечных адаптаций под `WorkspaceIntent` при необходимости).

---

## 9. Детальная последовательность внедрения (итерации)

## Итерация 1: Безопасный перенос в существующие виджеты

1. Перенести header-композицию в `chrome/view/header.rs`.
2. Перенести add-menu overlay в `sidebar/view/add_menu_overlay.rs`.
3. Перенести terminal context-menu overlay selector в `terminal_workspace/view/context_menu_overlay.rs`.
4. Убедиться, что поведение UI не изменилось.

### Критерии готовности итерации 1

1. Root `view.rs` больше не содержит код сборки header/add-menu/terminal overlay деталей.
2. Визуальное и интерактивное поведение совпадает с текущим.

## Итерация 2: Введение нового `workspace` widget

1. Создать модуль `widgets/workspace`.
2. Перенести `view_sidebar_layout` + `view_workspace_content`.
3. Перенести `view_tab_area` + `view_tab_content` + `missing_tab_state`.
4. Подключить `workspace` в root `view()`.

### Критерии готовности итерации 2

1. `view.rs` становится тонким orchestrator.
2. Все кросс-виджетные layout-решения локализованы в `workspace`.

## Итерация 3: Нормализация API и нейминга

1. Убрать дубликат `SidebarIntent::AddMenuDismiss`/`DismissAddMenu`.
2. Вычистить неиспользуемые импорты/константы в root.
3. Упростить mapping слоёв и повысить читаемость.

### Критерии готовности итерации 3

1. Нет дублирующих intent-событий.
2. В каждом виджете только релевантные view-функции.

---

## 10. Нефункциональные требования

1. Поведение UI не должно измениться функционально.
2. Все overlays должны оставаться взаимно-исключающими по resize-grips блокировке.
3. Никаких `unwrap()` в production-коде.
4. Минимизировать clone и лишние аллокации в новых props/model.
5. Сохранить текущую event-модель `AppEvent -> events/* -> widget.reduce`.

---

## 11. Риски и меры

1. Риск: циклические зависимости между `workspace` и другими widget modules.
   1. Мера: `workspace` как pure view-aggregator без собственного reducer/state на первом этапе.
2. Риск: регрессии в context-menu guard логике.
   1. Мера: после унификации dismiss intent обновить `guards.rs` тесты.
3. Риск: нарушение порядка слоёв в `Stack`.
   1. Мера: зафиксировать explicit порядок в root и добавить snapshot/UI smoke tests.

---

## 12. Тестовый план

1. Unit tests:
   1. `workspace/view/tab_area` для каждого `TabContent` маршрута.
   2. `sidebar add menu overlay` для anchor/dismiss событий.
   3. `terminal context menu overlay selector` при нескольких tabs.
2. Integration tests:
   1. При открытом add/context menu resize grips заблокированы.
   2. При закрытии overlay resize grips возвращаются.
3. Regression checks:
   1. `cargo +nightly fmt`
   2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   3. `cargo deny check`
   4. `cargo test --workspace --all-features`
   5. `cargo llvm-cov --workspace --all-features --fail-under-lines 80`

---

## 13. Definition of Done

1. Root `view.rs` не содержит локального UI-рендера конкретных виджетов.
2. Перенос выполнен согласно матрице раздела 4.
3. Новый `workspace` widget создан и используется для кросс-виджетной композиции.
4. Дублирующие sidebar dismiss intents устранены.
5. Все проверки из тестового плана проходят.
6. Документация по новым модулям и публичным API добавлена.

