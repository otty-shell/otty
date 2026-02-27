# OTTY v3 Functional Parity Plan - P1 (Feature and UX Parity)

## Scope
P1 выполняется после P0 и закрывает функциональные и UX-расхождения, которые не блокируют базовую работоспособность, но нарушают паритет с эталонным `otty`.

## Goals
1. Довести сценарии PRD 13.1 до фактического parity-поведения.
2. Восстановить недостающие UI-секции и interaction details.
3. Укрепить тестовый слой интеграционными и smoke-проверками.

## Work Items
1. Explorer open-file parity (editor command session).
Paths:
- `otty-v3/src/routers/flow/tabs.rs`
- `otty-v3/src/routers/explorer.rs`
- `otty-v3/src/routers/tabs.rs`
- `otty-v3/src/widgets/tabs/command.rs`
- `otty-v3/src/widgets/tabs/reducer.rs`
Definition:
- Открытие файла из Explorer должно создавать command terminal tab с editor command/session semantics, а не обычный shell tab.

2. Command terminal flows как first-class сценарий.
Paths:
- `otty-v3/src/routers/flow/quick_launch.rs`
- `otty-v3/src/routers/tabs.rs`
- `otty-v3/src/widgets/terminal_workspace/model.rs`
- `otty-v3/src/widgets/terminal_workspace/reducer.rs`
Definition:
- Явно разделить shell и command tab open flows.
- Обеспечить корректный `TerminalKind::Command` там, где ожидается command launch behavior.

3. Quick Launch sidebar drag-and-drop UI parity.
Paths:
- `otty-v3/src/widgets/quick_launch/view/sidebar_tree.rs`
- `otty-v3/src/widgets/quick_launch/reducer.rs`
Definition:
- Добавить курсорные события для корректного drag/drop targeting.
- Визуально отразить drop target, включая root/folder state.

4. Quick Launch Wizard full form parity.
Paths:
- `otty-v3/src/widgets/quick_launch/view/wizard_form.rs`
- `otty-v3/src/widgets/quick_launch/state.rs`
- `otty-v3/src/widgets/quick_launch/reducer.rs`
Definition:
- Добавить недостающие редакторы:
- command type switch (create mode);
- args list editor;
- env list editor;
- ssh extra args editor.
- Сохранить typed save request contract без прямой мутации tree из view.

5. Tab activation/closure parity для selection sync/focus.
Paths:
- `otty-v3/src/routers/tabs.rs`
- `otty-v3/src/routers/terminal_workspace.rs`
- `otty-v3/src/widgets/terminal_workspace/reducer.rs`
Definition:
- При activate/close tab обеспечить parity по focus + selection sync behavior для terminal panes.

6. Root title and app chrome polish parity.
Paths:
- `otty-v3/src/app.rs`
- `otty-v3/src/widgets/chrome/view/action_bar.rs`
Definition:
- Поведение заголовка и chrome должно соответствовать эталонным сценариям использования активного tab title.

7. Test parity layer (integration/smoke).
Paths:
- `otty-v3/tests/*`
- `otty-v3/src/**/tests` (точечно)
Definition:
- Добавить интеграционные тесты на cross-widget routing:
- wizard open/init/save/close;
- error tab open/init/close;
- settings reload/save failures;
- quick launch stale/cancel/persist completion;
- terminal context menu overlay visibility path.

## Acceptance Criteria
1. Все области PRD 13.1 покрыты либо unit + integration tests, либо стабильным smoke.
2. Поведение ключевых сценариев визуально и логически совпадает с `otty`:
- quick launch create/edit/launch/kill;
- explorer sync/open file;
- settings save/reload/apply;
- tabs and terminal workspace behavior.
3. Новые тесты ловят регрессии межмодульного wiring.

## Verification
1. `cargo test -p otty-v3 --all-features`
2. `cargo test --workspace --all-features`
3. Ручной smoke checklist из PRD 13.2:
- open/close tabs;
- split/close panes;
- quick launch create/edit/launch/kill;
- settings save/reload;
- explorer sync from terminal.
