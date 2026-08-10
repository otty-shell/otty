# Фаза 7: UI actions и presentation model

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-070–B2-078.

## Цель

Дать внутреннему BlockUI, overlay, keyboard и context menu единый action/query contract.
Пользовательские reorder/group/split/collapse операции меняют только presentation references,
не canonical PTY transcript и не immutable finished content.

## Текущее состояние

Подключены существующая geometry action button, hover/click и базовое копирование semantic
output/whole. Off-screen `ScrollToBlock` идёт через model request, а не только через текущий
frame.

Фаза не завершена: отсутствуют exhaustive `BlockAction`, `available_actions`, backend export
queries, collapse/pin/hide/rerun, presentation order/groups/slices, единый summary API и
полная keyboard accessibility.

## Объём работ

- [ ] **B2-070** Сначала протестировать один action path для internal controls, overlay,
  keyboard и context menu.
- [ ] **B2-071** Ввести exhaustive `BlockAction` и state-aware `available_actions()`.
- [x] **B2-072** Подключить action-button hover/click; дополнить focus и overlap tests.
- [ ] **B2-073** Перевести copy prompt/command/output/whole на backend `ExportBlock` query;
  текущие snapshot helpers являются временными.
- [ ] **B2-074** Collapse/expand, pin/hide и rerun через stable `BlockId`.
- [ ] **B2-075** Presentation order/groups/slices отдельно от canonical transcript.
- [ ] **B2-076** Move/group/split меняют references; physical mutation active content
  запрещена.
- [ ] **B2-077** Один `BlockSummary`/`BlockAction` API для внутреннего и внешнего UI.
- [ ] **B2-078** Keyboard focus/accessibility без перехвата обычного terminal input.

## Правила presentation model

- Canonical transcript остаётся append-only, кроме документированной retention/truncation.
- Active block нельзя физически split/move; UI может создавать только presentation slice.
- Action availability зависит от lifecycle/content capability, а не от видимости block.
- Rerun создаёт новый command execution/block и не мутирует старый outcome.
- Hidden/collapsed/pinned state не меняет export content.
- Internal и external controls публикуют одну semantic action, без дублирования business logic.

## Автоматическая проверка

```bash
cargo test -p otty-ui-term --all-features
cargo test -p otty-surface block
cargo test -p otty terminal_workspace
```

Добавить table-driven tests `available_actions` для active/finished/static/background/
truncated states и contract test, который отправляет одинаковое действие из четырёх UI paths.

## Ручная проверка

1. Запустить `cargo run -p otty`, создать несколько success/failure blocks и навести курсор на
   action area. Hover target и click target должны совпадать с нарисованной кнопкой.
2. Выполнить Copy Prompt/Command/Output/Whole из внутренней кнопки, overlay, context menu и
   keyboard. Для одного action все четыре пути должны возвращать идентичный результат.
3. Прокрутить block полностью за viewport и вызвать действие из внешнего списка. Оно должно
   адресовать тот же `BlockId` без предварительного render block.
4. Collapse/expand старый block; viewport anchor не должен прыгать. Pin/hide не меняют
   canonical transcript или export.
5. Rerun finished command. Должен появиться новый block с новым ID, старый outcome остаётся
   неизменным.
6. Создать group, изменить presentation order и split reference. Вернуть исходное
   представление и убедиться, что canonical order/content не изменились.
7. Попытаться split/move active block. Недоступное действие должно отсутствовать или вернуть
   явный отказ без частичной mutation.
8. Пройти controls только клавиатурой, проверить видимый focus и screen-reader labels. В
   обычном terminal mode печатные клавиши должны по-прежнему идти в PTY.

Фаза готова, когда все UI entry points используют один action API, off-screen operations не
зависят от snapshot geometry, а presentation changes не мутируют transcript.

