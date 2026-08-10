# Фаза 8: export и сохранение

Статус: **не начато**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-080–B2-085.

## Цель

Создать единый backend pipeline, который экспортирует любую секцию on-screen или off-screen
блока в Plain, ANSI, Markdown или versioned JSON. Clipboard и Save должны использовать один
результат; filesystem I/O выполняется вне render thread и возвращает явный success/error.

## Текущее состояние

Первый срез умеет копировать часть semantic content из snapshot, но это временный UI helper.
Единого `ExportBlock` query, formatters, golden tests, atomic save и `otty.block` schema нет.
Поэтому ни один B2-080–B2-085 пока не считается выполненным.

## Объём работ

- [ ] **B2-080** Сначала добавить golden tests для Plain/ANSI/Markdown/JSON, soft wrap, wide
  chars, hyperlinks, trailing spaces и truncation markers.
- [ ] **B2-081** Реализовать единый `ExportBlock` pipeline в
  `otty-surface/src/block/export.rs` с выбором Prompt/Command/Output/Whole.
- [ ] **B2-082** Перевести все clipboard entry points на backend export result, удалив
  viewport/cached-text fallback.
- [ ] **B2-083** Реализовать atomic save в application layer с explicit result/error event,
  временным файлом в той же директории, flush/sync и rename.
- [ ] **B2-084** Определить versioned `otty.block` JSON schema и round-trip tests без
  сериализации внутреннего `Surface`.
- [ ] **B2-085** До session persistence согласовать secret/redaction policy и отдельно
  запросить разрешение на любые storage/compression dependencies.

## Контракт форматов

- Plain удаляет terminal fill spaces, сохраняет значимые внутренние пробелы и не превращает
  soft wrap в hard newline.
- Wide-char spacer cells не экспортируются второй раз.
- ANSI восстанавливает только поддерживаемые SGR/hyperlink semantics и всегда имеет валидный
  reset/termination.
- Markdown содержит отдельные command/output fences и стабильную metadata section.
- JSON имеет собственные schema name/version и не зависит от live protocol version.
- `Whole` всегда имеет порядок prompt → command → output; `Output` не содержит prompt.
- Truncated content включает marker и доступное количество потерянных строк.

## Автоматическая проверка

```bash
cargo test -p otty-surface block::export
cargo test -p otty-ui-term --all-features
cargo test -p otty terminal_workspace
```

Golden fixtures должны быть небольшими, человекочитаемыми и проверять exact bytes. JSON
round-trip test должен отвергать unsupported major schema и сохранять неизвестные безопасные
optional fields согласно выбранной migration policy.

## Ручная проверка

1. Запустить `cargo run -p otty` и создать block с Unicode/wide chars, цветным ANSI output,
   OSC 8 hyperlink, длинной soft-wrapped строкой и значимыми внутренними пробелами.
2. Изменить ширину окна, чтобы wrapping стал другим, и выполнить Copy Output. Plain text не
   должен получить новые hard newlines из-за reflow.
3. Скопировать Prompt, Command, Output и Whole в каждом формате и сравнить их с contract выше.
4. Прокрутить block за viewport, затем collapse/hide его и повторить export. Результат должен
   быть byte-identical результату видимого expanded block.
5. Сохранить тот же export в файл и сравнить bytes с clipboard/backend result. Copy и Save не
   должны иметь разные formatters.
6. Открыть ANSI result в поддерживающем terminal, Markdown — в preview, JSON — parser-ом;
   control sequences/fences/schema должны быть валидны.
7. Превысить output budget и проверить явный truncation marker во всех форматах.
8. Сохранить поверх существующего файла, затем смоделировать permission denied и прерывание до
   rename. При успехе файл заменяется атомарно, при ошибке исходный файл не повреждается и UI
   показывает причину.
9. Осмотреть export diagnostics/logs: command/output contents не должны туда попадать.

Фаза готова, когда все UI actions используют один backend pipeline, off-screen export не
требует scroll/render, а clipboard и atomic save дают одинаковые bytes.

