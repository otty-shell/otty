# Фаза 0: baseline и regression-сценарии

Статус: **Готово**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-001–B2-005.

## Цель

Зафиксировать воспроизводимые ошибки старой модели и числовой baseline для дальнейшей замены
архитектуры. Результат фазы — тесты, сценарии и метрики, которые доказывают улучшение или
регрессию следующих фаз.

## Текущее состояние

Model regression tests покрывают active growth, resize, head truncation и off-screen
`ScrollTo`. Bash/Zsh и nested-shell harness работает через настоящий PTY; test-only legacy
fixture воспроизводит потерю nested integration и collision ID. Добавлены оценки snapshot и
block memory, а также current/peak queue depths. Ignored release-сценарий для 10 000 блоков,
длинного output, resize и медленного consumer выполнен дважды; параметры машины и оба набора
результатов сохранены в [baseline-results.md](baseline-results.md).

Все автоматические пункты выполнены. Для строгого статуса «Выполнено» остаётся пройти
описанный ниже визуальный scroll/resize сценарий в реальном окне и зафиксировать результат.

## Объём работ

- [x] **B2-001** Дополнить model-level regression tests сценариями active growth, resize,
  head truncation и off-screen `ScrollTo`; существующие manual-scroll tests считать только
  частью пункта.
- [x] **B2-002** Сохранить v1 framing только как legacy test fixture и зафиксировать
  требуемый финальный результат: после удаления v1 старые, fragmented и malformed
  `otty-dcs;block` sequences не создают semantic event/block и не вызывают panic.
- [x] **B2-003** Перевести Bash/Zsh harness на PTY и воспроизвести nested integration loss и
  collision IDs до применения исправления.
- [x] **B2-004** Измерять snapshot bytes/build time, replaceable frame depth, lossless queue
  depth и приблизительную block memory без записи command/output.
- [x] **B2-005** Добавить воспроизводимые benchmark или ignored integration scenarios для
  10 000 блоков, длинного output, resize и медленного consumer без новой dependency.
- [x] Сохранить результаты и сведения о машине в `tasks/blocks-v2/baseline-results.md`, чтобы
  фаза 10 могла сравнить одинаковые сценарии.

## Требования к измерениям

- Отдельно считать mutable active content и finished history.
- Отчёт должен содержать число блоков, строк, columns, viewport size и длительность прогона.
- Нельзя логировать текст prompt, command или output.
- Повторный запуск с теми же параметрами должен давать сопоставимые числа.
- Benchmark failure должен быть диагностируемым: memory, latency, queue depth и scroll
  correctness фиксируются раздельно.

## Автоматическая проверка

```bash
cargo test -p otty-surface block
cargo test -p otty-escape dcs
cargo test -p otty-libterm terminal::channel
cargo test -p otty --test shell_integration -- --nocapture
cargo test --release -p otty --test blocks_baseline -- --ignored --nocapture
```

После добавления benchmark выполнить документированную в `baseline-results.md` команду дважды
и убедиться, что оба результата содержат одинаковый набор метрик.

## Ручная проверка

1. Запустить приложение из корня workspace: `cargo run -p otty`.
2. Выполнить команду, печатающую 100 000 нумерованных строк:
   `i=1; while [ "$i" -le 100000 ]; do printf '%06d\n' "$i"; i=$((i + 1)); done`.
3. Во время вывода уйти колёсиком/клавишами в середину истории и записать, прыгает ли видимая
   строка при продолжении output.
4. Изменить ширину окна по схеме 80 → 200 → 40 columns и записать, сохраняется ли та же
   логическая строка.
5. Перейти к блоку, который полностью находится вне viewport, и проверить `ScrollTo`.
6. После GUI-сценария запустить документированную release-команду из
   `baseline-results.md` и сравнить snapshot size/build time, queue depth и memory estimate.
7. Повторить сценарий в Bash и Zsh с nested interactive shell.

Ожидаемый результат этой фазы — стабильное воспроизведение исторической ошибки test-only
fixture, корректное поведение production v2 и зафиксированные числа. Фазу можно закрыть только
после подтверждения визуального сценария и когда другой разработчик способен повторить прогон
по этому документу без дополнительных устных инструкций.
