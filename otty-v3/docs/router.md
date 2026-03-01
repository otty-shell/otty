# Router Architecture (`otty`)

Этот документ фиксирует правила и контракты роутинга в `otty`.

## 1. Цель

Роутеры являются app-orchestration слоем между `AppEvent` и widget-редьюсерами.
Они обеспечивают:

1. Детерминированный путь обработки событий.
2. Разделение `Ui` и `Effect` каналов.
3. Кросс-widget координацию только через app-level события.
4. Thin-dispatch в `src/update.rs`.

## 2. Термины

1. `UiEvent` — событие из `view`, описывает пользовательское действие.
2. `EffectEvent` — событие из `reduce`, описывает результат/запрос side-effect.
3. `WidgetCommand` — внутренняя команда редьюсера виджета.
4. `Owning router` — модуль `src/routers/<domain>.rs`, владеющий mapping для одного widget-домена.
5. `Flow router` — модуль `src/routers/flow/*`, координирующий многошаговый или переиспользуемый cross-widget сценарий.

## 3. Слои и границы

Архитектурный поток:

```text
UI:     view -> AppEvent::<Widget>Ui -> route_event -> WidgetCommand -> reduce
EFFECT: reduce -> WidgetEffectEvent -> AppEvent::<Widget>Effect -> route_effect -> app task/action
```

Инварианты:

1. `Ui`-события маппятся только в `WidgetCommand`.
2. `Effect`-события маппятся только в app-level `Task<AppEvent>`/action.
3. `reduce` не эмитит `Ui`-события.
4. Для одного события существует одна стабильная ветка обработки.
5. Роутеры не импортируют друг друга; cross-widget переходы выполняются через `AppEvent`/`AppFlowEvent`.

## 4. `src/update.rs` контракт

`update.rs` выполняет top-level диспетчеризацию `AppEvent` по роутерам.

Разрешено исключение (зафиксировано в PRD): app-wide pre-dispatch guard-блок перед основным dispatch, если одновременно выполняются условия:

1. Guard кросс-доменный.
2. Guard не делает widget-specific `Ui -> Command` mapping.
3. Guard не делает widget-specific `Effect -> AppEvent` mapping.
4. Guard не содержит доменной бизнес-логики конкретного widget-а.

Пример допустимого guard-кейса: interaction/menu gating перед dispatch.

## 5. Контракт owning router

Для каждого widget-домена:

1. `route_event(app, WidgetUiEvent) -> Task<AppEvent>`
2. `route_effect(..., WidgetEffectEvent) -> Task<AppEvent>`
3. `map_*_ui_event_to_command(WidgetUiEvent) -> WidgetCommand`
4. `map_*_effect_event_to_app_task(WidgetEffectEvent) -> Task<AppEvent>`

Допустимо:

1. Локальный helper `route_command(app, WidgetCommand)` для повторного использования из `update.rs`/flow/app orchestration.
2. `Task::batch` для нескольких независимых реакций.
3. Wildcard fallback в effect mapping: `_ => Task::none()`.

Недопустимо:

1. Дублировать одну и ту же реакцию одновременно в `route_event` и `route_effect`.
2. Возвращать `WidgetUiEvent` из `reduce`.
3. Мутировать `WidgetState` вне его `reduce`.

## 6. Контракт flow router

`src/routers/flow/mod.rs` является thin-dispatch по `AppFlowEvent`.

Назначение flow:

1. Многошаговые сценарии.
2. Переиспользуемая cross-widget orchestration.
3. Последовательности вида `Open* -> *Opened -> Initialize*`.

Когда flow не нужен:

1. One-hop сценарий можно завершить прямым `Task::done(AppEvent::<Target>...)` из owning router.

## 7. Карта роутеров в `otty-v3`

Текущие доменные роутеры:

1. `src/routers/sidebar.rs`
2. `src/routers/chrome.rs`
3. `src/routers/tabs.rs`
4. `src/routers/quick_launch.rs`
5. `src/routers/terminal_workspace.rs`
6. `src/routers/explorer.rs`
7. `src/routers/settings.rs`
8. `src/routers/window.rs`
9. `src/routers/flow/mod.rs` + `flow/tabs.rs` + `flow/quick_launch.rs`

Роли:

1. `window.rs` — app-global runtime/window операции (resize, drag_resize, sync grid).
2. `flow/*` — cross-widget orchestration.
3. Остальные — owning routers соответствующих widget-доменов.

## 8. Паттерны маршрутизации

### 8.1 UI path

1. `view` эмитит `WidgetUiEvent`.
2. `src/view.rs` маппит его в `AppEvent::<Widget>Ui`.
3. `update.rs` делегирует в `routers::<widget>::route_event`.
4. Роутер превращает `UiEvent` в `WidgetCommand`.
5. `widget.reduce(command, ctx)` возвращает `Task<WidgetEffectEvent>`.
6. Роутер маппит его в `AppEvent::<Widget>Effect`.

### 8.2 Effect path

1. `update.rs` получает `AppEvent::<Widget>Effect`.
2. Делегирует в `routers::<widget>::route_effect`.
3. Роутер превращает `EffectEvent` в app-level task/action:
   `Task::done(AppEvent::...)`, `Task::perform(...)`, `Task::batch(...)`, `Task::none()`.

### 8.3 Multi-step flow path

1. Owning router получает `Effect` и эмитит `AppEvent::Flow(AppFlowEvent::...)`.
2. `update.rs` делегирует в `routers::flow::route`.
3. Flow use-case модуль выполняет следующий шаг и при необходимости эмитит следующий `Flow` event.
4. Финальный шаг завершает сценарий через целевой owning router/`AppEvent`.

## 9. Правила детерминизма

1. Одно событие обрабатывается одной предсказуемой веткой.
2. Один `UiEvent` не должен иметь разные маппинги в разных местах.
3. Один `EffectEvent` не должен «теряться» неявно.
4. Явный no-op должен быть выражен как `Task::none()`.
5. Если используется `Task::batch`, каждый элемент batch должен быть сам по себе детерминирован.

## 10. Async-flow lifecycle

Для widget-ов с асинхронными эффектами:

1. Использовать `request_id`.
2. Поддерживать `in_flight` по `resource_key`.
3. Обеспечивать single-flight per key.
4. Игнорировать stale completion (mismatch `request_id`).
5. Освобождать key на `Done/Failed/Cancelled`.
6. Покрывать тестами: stale, cancel, retry, double-start, parallel keys.

## 11. Что проверять в code review для роутеров

1. `Ui -> Command` находится только в owning router.
2. `Effect -> AppEvent/Task` находится только в owning router.
3. `update.rs` не содержит widget-specific mapping.
4. Pre-dispatch guard в `update.rs` (если есть) кросс-доменный и без доменной логики widget-ов.
5. `flow/mod.rs` остаётся thin-dispatch.
6. Нет прямых зависимостей `router -> router`.
7. Нет циклов вида `UiEvent::A -> Command::A -> UiEvent::A`.
8. Для новых async flow есть lifecycle тесты.

## 12. Чеклист добавления нового события

1. Добавить новый вариант в `widgets/<name>/event.rs` (`Ui` или `Effect`).
2. Если это `Ui`, добавить mapping в `map_*_ui_event_to_command`.
3. Добавить обработку в `widgets/<name>/reducer.rs` через `WidgetCommand`.
4. Если reducer эмитит новый `Effect`, добавить mapping в `map_*_effect_event_to_app_task`.
5. При необходимости добавить/обновить `AppFlowEvent` и `routers/flow/*`.
6. Добавить тест маршрутизации (минимум: событие приводит к ожидаемой ветке).
7. Для async-сценария добавить lifecycle-тесты.

## 13. Источник истины

Нормативный источник: `../prd.md`.

При конфликте локальных комментариев в коде и этого документа приоритет у `../prd.md`, если для `otty-v3` не зафиксировано отдельное явное исключение (как для app-wide pre-dispatch guard в `update.rs`).
