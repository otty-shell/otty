# Фаза 4: раздельный block content и freeze

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-040–B2-047.

## Цель

Хранить prompt, canonical command и output как самостоятельные семантические секции. Пока
команда активна, mutable terminal state разрешён только владельцу output. После completion
блок замораживается в компактные read-only logical lines и больше не хранит полный `Surface`.

## Текущее состояние

Добавлены `BlockContent` и `CommandRecord`, базовый capture prompt range, command header и
output, а snapshot/query может возвращать semantic prompt/output. Это минимальный первый срез,
а не завершённая sectioned-content model.

Фаза не завершена: нет самостоятельных `HeaderGrid`/`OutputGrid`, output routing, immutable
freeze, budgets/truncation, alt-screen ownership и отказа от `cached_text`/mutable historical
surfaces.

## Объём работ

- [ ] **B2-040** Сначала покрыть section boundaries для single/multiline/right prompt,
  отредактированной команды, empty command, output и background output.
- [ ] **B2-041** Ввести явные `HeaderGrid`, prompt/command ranges и `OutputGrid`; текущий
  `BlockContent` считать подготовкой.
- [ ] **B2-042** Реализовать `OutputRouter` для root/child active block, header, output,
  background stream и alt screen.
- [ ] **B2-043** Дополнить существующий `CommandRecord` явным confidence и всеми source cases;
  shell command уже используется как основной source.
- [ ] **B2-044** При completion конвертировать block в read-only logical lines и освобождать
  runtime-only `Surface` state.
- [ ] **B2-045** Добавить per-block/global budgets, явную truncation metadata и безопасное
  обновление anchors/selections.
- [ ] **B2-046** Связать alt screen с owning command block и восстановлением normal viewport.
- [ ] **B2-047** Удалить per-snapshot `cached_text`; все off-screen queries должны читать
  frozen content.

## Инварианты данных

- Prompt не входит в `Output`, а command не извлекается из первой визуальной строки как
  основной путь.
- Soft wrap меняет visual rows, но не logical content или section ranges.
- Finished content неизменяем; дальнейший PTY output маршрутизируется только в active owner
  либо документированный background destination.
- Truncation всегда видима в metadata/export и не оставляет anchor внутри удалённого range.
- Alt-screen content принадлежит запустившему его command block и не загрязняет normal output.

## Автоматическая проверка

```bash
cargo test -p otty-surface block::content
cargo test -p otty-surface block
cargo test -p otty-ui-term block
```

Добавить memory assertion или metric-based ignored test, доказывающий, что после freeze число
полных mutable `Surface` не растёт линейно с числом finished blocks.

## Ручная проверка

1. Запустить `cargo run -p otty` и выполнить команды с обычным prompt, пустым output, несколькими
   строками output и multiline heredoc.
2. Набрать команду, отредактировать её стрелками до Enter и проверить, что Copy Command
   возвращает исполненный текст, а не исходные keystrokes или первую визуальную строку.
3. В Zsh включить правый prompt и повторить копирование prompt/command/output по отдельности.
4. Запустить background command, который печатает между двумя prompts. Его output не должен
   попадать в случайный завершённый блок.
5. Для одного блока выполнить Copy Prompt, Copy Command, Copy Output и Copy Whole. Каждая
   операция должна содержать только заявленные секции в стабильном порядке.
6. Запустить `less` или полноэкранное TUI, выйти из него и проверить восстановление normal
   viewport и принадлежность alt-screen правильному block.
7. Завершить 1 000 небольших команд, изменить ширину окна и продолжить печать. Содержимое
   старых блоков не должно меняться, а metric mutable surfaces должна оставаться ограниченной.
8. Превысить per-block и global budgets. UI/export должны показать truncation marker и число
   потерянных строк.

Фаза готова только после удаления mutable `Surface` из finished history и прохождения всех
section-boundary сценариев без viewport-based эвристик.
