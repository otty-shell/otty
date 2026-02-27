# OTTY v3 Functional Parity Plan - P0 (Critical Wiring and Lifecycle)

## Scope
P0 закрывает критичные расхождения с baseline из PRD 13.1, из-за которых UI и сценарии работают некорректно даже при зеленых unit-тестах.

## Goals
1. Восстановить корректный межвиджетный event/effect wiring.
2. Восстановить корректный async lifecycle для Quick Launch.
3. Убрать блокирующие регрессии в runtime overlays, guards и layout math.
4. Вернуть корректный startup/apply settings flow (shell/theme/runtime settings).

## Work Items
1. Quick Launch wizard/error tab wiring без `tab_id: 0`.
Paths:
- `otty-v3/src/routers/flow/quick_launch.rs`
- `otty-v3/src/widgets/tabs/command.rs`
- `otty-v3/src/widgets/tabs/event.rs`
- `otty-v3/src/widgets/tabs/reducer.rs`
- `otty-v3/src/routers/tabs.rs`
Definition:
- Flow должен получать реальный `tab_id` при `OpenWizardTab`/`OpenErrorTab` и передавать его в `QuickLaunchCommand::WizardInitialize*` и `QuickLaunchCommand::OpenErrorTab`.

2. Quick Launch async lifecycle через completion events вместо dummy close.
Paths:
- `otty-v3/src/widgets/quick_launch/reducer.rs`
- `otty-v3/src/widgets/quick_launch/event.rs`
- `otty-v3/src/routers/quick_launch.rs`
Definition:
- `request_persist_quick_launches` должен возвращать `PersistCompleted`/`PersistFailed`.
- `launch_quick_launch` должен возвращать `SetupCompleted(outcome)`.
- Router не должен использовать `CloseTabRequested { tab_id: 0 }` как no-op транспорт.
- Stale/cancel guards должны исполняться в одном completion path.

3. Включить global guards в root update loop.
Paths:
- `otty-v3/src/update.rs`
- `otty-v3/src/guards.rs`
Definition:
- Добавить pre-dispatch guard обработку по аналогии с baseline:
- inline edit cancel policy;
- context menu allow/ignore/dismiss policy;
- централизованное закрытие всех открытых меню.

4. Вернуть terminal pane context menu overlay в root view.
Paths:
- `otty-v3/src/view.rs`
- `otty-v3/src/widgets/terminal_workspace/view/pane_context_menu.rs`
Definition:
- При открытом terminal context menu рендерить overlay слой и блокировать resize grips.

5. Исправить double subtraction tab bar в вычислениях размеров.
Paths:
- `otty-v3/src/view.rs`
- `otty-v3/src/state.rs`
- `otty-v3/src/routers/window.rs`
Definition:
- `TAB_BAR_HEIGHT` вычитается один раз в общем pipeline расчета размеров.

6. Восстановить startup shell session + fallback.
Paths:
- `otty-v3/src/app.rs`
- `otty-v3/src/widgets/terminal_workspace/services.rs`
- `otty-v3/src/routers/flow/tabs.rs`
- `otty-v3/src/routers/tabs.rs`
Definition:
- Инициализировать shell session при старте.
- На ошибке setup использовать fallback session.
- Заголовок default terminal tab должен соответствовать shell name.
- Shell tab открывается с session-aware terminal settings.

7. Восстановить settings apply pipeline.
Paths:
- `otty-v3/src/widgets/settings/reducer.rs`
- `otty-v3/src/routers/settings.rs`
- `otty-v3/src/app.rs`
Definition:
- `Reload` и `Save` должны проходить через корректные completion события.
- Ошибки save/reload не должны маскироваться как success.
- При apply/save обновлять:
- app theme manager;
- app terminal settings;
- shell session setup/fallback;
- terminal palette (через terminal workspace).

8. Subscription parity для quick launch tick.
Path:
- `otty-v3/src/subscription.rs`
Definition:
- Tick включается при `has_active_launches || dirty || persist_in_flight`.

## Acceptance Criteria
1. Smoke сценарии работают без блокирующих регрессий:
- open/close tabs;
- quick launch create/edit/save/open error;
- quick launch launch/cancel/stale completion;
- terminal context menu copy/paste/split/close.
2. Нет usage `tab_id: 0` в рабочих flow сценариях.
3. Guard policy фактически применяется в `update`.
4. Layout resize корректен без двойного уменьшения grid.

## Verification
1. `cargo test -p otty-v3 --all-features`
2. Ручной smoke:
- context menu interaction safety;
- quick launch lifecycle;
- settings reload/save and apply;
- startup terminal behavior.
