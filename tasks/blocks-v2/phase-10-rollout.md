# Фаза 10: финальная активация v2 и удаление legacy

Статус: **частично выполнена: зафиксировано решение по v1 и выполнена проверочная
подготовка**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-100–B2-105.

## Цель

После прохождения полной acceptance matrix сделать Blocks v2 единственным production
path и до релиза удалить v1 emission/parser/actions/lifecycle, а также старую
stitched-history architecture.

## Фиксированное решение по v1

- V1 не является публичным compatibility mode и не остаётся в production бинарнике
  вместе с v2.
- Runtime switch `old/v2`, compatibility adapter и migration window не реализуются.
- Shell integration и parser поставляются одним application artifact, поэтом user-data
  migration между protocol versions не требуется.
- Rollback выполняется установкой предыдущего application artifact. Предыдущий path не
  хранится в новом бинарнике ради rollback.
- После удаления v1 старое `otty-dcs;block` сообщение безопасно игнорируется
  общим DCS parser-ом, не создаёт block/event и не вызывает panic.

## Текущее состояние

В рамках первого среза запускались format, Clippy, deny, workspace tests и coverage. Все
команды проходят. Зафиксированное глобальное line coverage составляет 51.16%;
фиксированного минимального порога нет, а последующие изменения не должны снижать общий
line coverage относительно baseline до изменений. `cargo deny` сообщает существующие
warnings о yanked transitive `core2 0.4.0` и `spin 0.9.8`, но завершается успешно.

Архитектурное решение B2-100 зафиксировано. V1 script emission, parser/schema,
actions/handlers и lifecycle удалены из production; framing сохранён только в ignore-тестах и
исторической документации. Baseline фазы 0 зафиксирован. Полная acceptance matrix, сравнение с
финальной архитектурой и удаление old stitched-history ещё не выполнены.

## Объём работ

- [x] **B2-100** Зафиксировать single-path release policy: не добавлять runtime switch,
  compatibility adapter и migration window; rollback выполняется предыдущим artifact.
- [x] **B2-101** Добиться успешного прохождения полного обязательного набора проверок,
  включая `cargo llvm-cov`, подтвердить, что общий line coverage не снизился относительно
  baseline до изменений, и не допустить новых deny errors/warnings.
- [ ] **B2-102** После завершения целевой архитектуры повторить B2-004/B2-005 и сравнить с ранее
  зафиксированным `baseline-results.md`; не собирать два engine path в одном бинарнике
  ради сравнения. Зафиксировать memory, frame, latency и scroll correctness.
- [ ] **B2-103** Подтвердить, что каждая новая supported terminal session использует только
  v2 после Bash/Zsh, viewport, export и burst-output acceptance matrix; bootstrap failure оставляет
  рабочий ordinary terminal с `Degraded`, а не переключает его на v1.
- [ ] **B2-104** До релиза завершить и подтвердить removal из B2-014/B2-021: удалить
  оставшиеся v1 script emission, parser/schema/actions/handlers/tests кроме legacy-ignore fixture,
  а также old stitched-history snapshot,
  временный `integration_status_badge` и относящийся к нему `is_shell` plumbing из pane-grid;
  финальные integration diagnostics оставить в status/debug UI. Подтвердить отсутствие
  production search matches для v1.
- [ ] **B2-105** Обновить `otty-surface`, `otty-escape` и `otty-ui/terminal` README финальными
  контрактами, limits, examples и troubleshooting без инструкций по миграции v1.

## Условия финализации

- Фазы 0–8 закрыты, а применимые пункты фазы 9 имеют честные capability statuses.
- Finished history не хранит mutable Surface на block.
- Off-screen copy/save и stable viewport/selection проверены вручную.
- Replaceable backlog ограничен одним frame при burst output.
- Protocol spoof/malformed/stale matrix не corrupt-ит model.
- Final v2 и сохранённый baseline измерены одинаковыми сценариями на сопоставимой
  машине.

## Автоматическая проверка

```bash
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features
```

Coverage запускать одной и той же командой до и после изменений; итоговый overall line
coverage не должен быть ниже исходного. Фиксированный минимальный процент не применяется.

Дополнительно выполнить benchmark/ignored scenarios фазы 0 для финального v2 и сравнить с
ранее сохранённым baseline report. Результат должен содержать не только среднее время,
но и параметры history/viewport, peak memory, replaced frame count и факт прохождения scroll assertions.

## Ручная проверка

1. Запустить финальную сборку и убедиться, что каждая новая Bash/Zsh terminal session
   получает v2 без user-visible protocol selector.
2. Пройти Bash и Zsh root/nested lifecycle, manual-scroll/resize/selection matrix, миллион строк
   burst output и off-screen copy/save.
3. Сравнить результаты с `baseline-results.md`: v2 должен удовлетворять limits раздела
   13.5, а любое ухудшение документируется и блокирует релиз.
4. Принудительно вызвать bootstrap/protocol failure: terminal остаётся рабочим, UI показывает
   причину `Degraded`, v1 fallback не запускается.
5. Отправить старое v1 DCS событие. Оно должно безопасно игнорироваться, не создавать
   block/event и не вызывать panic.
6. Выполнить repository search по `otty-dcs;block`, `BlockEvent`, `BlockPhase`, old stitched snapshot и
   runtime switch. Production references должны отсутствовать; v1 framing допустим только как
   legacy-ignore test fixture и в historical notes.
7. По финальным README с чистого окружения настроить Bash/Zsh и воспроизвести active/degraded/
   unsupported cases без знания внутренней реализации.

Фаза и весь Blocks v2 готовы только когда v2 является единственным production path,
старый код удалён до релиза, все команды выше проходят, а Definition of Done общего roadmap
подтверждён ручной acceptance matrix.
