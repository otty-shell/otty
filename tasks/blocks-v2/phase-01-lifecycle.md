# Фаза 1: typed identity и lifecycle reducer

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-010–B2-016.

## Цель

Сделать lifecycle блока детерминированным и независимым от эвристик UI. Каждое событие должно
адресовать terminal session, shell instance и block через отдельные типы. Duplicate, stale,
missing и out-of-order events обрабатываются reducer-ом и не могут завершить соседний блок.

## Текущее состояние

В первом вертикальном срезе добавлены `BlockId`, `TerminalSessionId`, `ShellInstanceId`,
`ProtocolSequence`, lifecycle states/outcomes, sparse metadata merge, diagnostics,
`block_id_to_index` и tests основных recovery cases. Старая duplicate-ID логика больше не
завершает предыдущий блок.

Старый `BlockEvent` lifecycle path удалён: production lifecycle принимает только
protocol-v2 semantic events. Фаза не завершена, потому что `block.rs` ещё не разбит на
окончательные focused `model`/`list` modules, transition matrix неполна, а reorder invariants
нужно проверить вместе с будущей presentation model.

## Объём работ

- [ ] **B2-010** Дополнить tests до всех transitions/recovery cases; normal, duplicate/stale,
  gap, missing prepare/end, orphan end и shell exit уже покрыты.
- [x] **B2-011** Ввести typed IDs и минимальные публичные accessors.
- [ ] **B2-012** Формализовать отдельный `MetadataPatch`; lifecycle state/outcome и sparse
  metadata update без полной замены известных полей уже реализованы.
- [ ] **B2-013** Довести index tests до append/remove/reorder и randomized sequences; append и
  remove уже покрыты.
- [x] **B2-014** Удалить v1 `BlockEvent` lifecycle path; `LifecycleInput` и reducer принимают
  только protocol-v2 semantic events. Compatibility adapter не добавлять.
- [x] **B2-015** Duplicate/stale ID больше не должен вызывать completion другого блока.
- [ ] **B2-016** Разделить оставшийся монолит на cohesive `model`, `lifecycle`, `id` и `list`
  без pass-through модулей и расширения public API.

## Обязательные lifecycle cases

- normal prompt → command start → command end → next prompt;
- empty Enter и Ctrl-C до/после command start;
- duplicate/stale sequences и sequence gap;
- missing `prompt_prepare` и missing `command_end` с документированным recovery;
- sparse completion не стирает command, cwd-before или start time;
- unknown block ID не меняет соседний block;
- nested/root shell exit завершает только принадлежащие ему active blocks;
- invalid transition создаёт безопасную diagnostic без command/output payload.

## Автоматическая проверка

```bash
cargo test -p otty-surface block::lifecycle
cargo test -p otty-surface block
cargo clippy -p otty-surface --all-targets --all-features -- -D warnings
```

Перед закрытием добавить table-driven test для каждого пункта выше и invariant test, который
после произвольной последовательности append/remove/reorder сравнивает `block_id_to_index` с
простым линейным поиском.

## Ручная проверка

1. Запустить `cargo run -p otty` с Bash.
2. Выполнить последовательно `pwd`, `false`, `cd /tmp`, `true` и пустой Enter.
3. Убедиться, что каждой реально запущенной команде соответствует ровно один блок, а пустой
   Enter не завершает предыдущую команду повторно.
4. Из корня репозитория дважды выполнить `source assets/shell-integrations/otty.bash`, затем
   запустить `printf 'duplicate-check\n'`. Должен появиться один command block.
5. Запустить nested `bash --noprofile --norc -i`, выполнить в нём `false`, выйти и выполнить
   `true` в родительском shell. Child exit не должен завершить или изменить parent block.
6. Повторить сценарий в Zsh с `assets/shell-integrations/otty.zsh`.
7. Прокрутить назад и визуально проверить, что команды, cwd и завершённые блоки не изменяются
   после следующих prompt events.

Фаза готова, когда production lifecycle имеет один protocol-v2 reducer, старый
`BlockEvent` path отсутствует, все recovery rules выражены тестами, а ручной сценарий не
создаёт duplicate/missing/cross-shell blocks.
