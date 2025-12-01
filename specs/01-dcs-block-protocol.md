# 01. DCS‑протокол блоков и Action::BlockEvent

## Обзор

Первая задача: реализовать протокол блоков на уровне `otty-escape` и провести его до `TerminalSurfaceActor` в `otty-libterm`, не меняя модель поверхности и поведение отрисовки. Результат — ядро понимает DCS‑сообщения о блоках и генерирует `Action::BlockEvent`, но ещё никак не меняет контент терминала.

См. спецификацию: `specs/spec.md`, разделы 5 и 6.3 (частично).

## Что уже есть

- `otty-escape`:
  - Реализует парсинг стандартных ESC/DCS последовательностей и транслирует их в `Action`.
  - Структура `Action` пока не содержит вариантов для блоков.
- `otty-libterm`:
  - `TerminalSurfaceActor<'a, S>` адаптирует `Action` → `SurfaceActor` и уже умеет обрабатывать множество вариантов `Action`.
  - `TerminalEngine` использует `EscapeParser::advance` + `TerminalSurfaceActor` для обработки входящего потока байт.

## Что нужно сделать

1. Ввести типы блоков в `otty-escape`:
   - `BlockKind` (`Command`, `Prompt`, `FullScreen`).
   - `BlockPhase` (`Preexec`, `Exit`, `Precmd`).
   - `BlockMeta` с полями:
     - `id: String`;
     - `kind: BlockKind`;
     - `cmd: Option<String>`;
     - `cwd: Option<String>`;
     - `started_at: Option<i64>`;
     - `finished_at: Option<i64>`;
     - `exit_code: Option<i32>`;
     - `shell: Option<String>`;
     - `is_alt_screen: bool` (пока может оставаться `false`, до фактической интеграции с SurfaceMode).
   - `BlockEvent { phase: BlockPhase, meta: BlockMeta }`.
   - Новый вариант `Action::BlockEvent(BlockEvent)`.

2. Реализовать парсинг DCS‑протокола блоков:
   - В DCS‑обработчике распознавать префикс `otty-block;`.
   - Извлекать текст JSON (UTF‑8) после префикса и до окончания DCS.
   - Парсить JSON в `BlockMeta` и `BlockPhase` по схеме из RFC:
     - `v`, `id`, `phase` (обязательные поля);
     - `cmd`, `cwd`, `time`, `exit_code`, `shell` (опциональные).
   - Учесть лимиты:
     - ограничить длину `cmd` (например, до 1024 символов);
     - ограничить длину `cwd` (например, до 512 символов);
     - ограничить общий размер JSON (например, до 4096 байт); при превышении — возвращать ошибку парсинга и игнорировать DCS.
   - При ошибках парсинга:
     - писать предупреждение в лог (через существующий логгер);
     - не генерировать `Action::BlockEvent` и не прерывать обработку оставшихся байт.

3. Прокинуть `Action::BlockEvent` до `TerminalSurfaceActor`:
   - Обновить `TerminalSurfaceActor::process_action`:
     - добавить ветку `Action::BlockEvent(event)` с временным поведением: запись события в лог и игнорирование (без изменений `surface`).
   - Убедиться, что `EscapeActor`/`TerminalSurfaceActor` компилируются с новым вариантом `Action`.

## Ожидаемый результат

- `otty-escape` корректно распознаёт DCS‑строки с префиксом `otty-block;` и JSON‑метаданными.
- При получении валидного JSON создаётся `Action::BlockEvent(BlockEvent)`.
- `TerminalSurfaceActor` принимает `Action::BlockEvent`, логирует его и не влияет на состояние `Surface`.
- При отсутствии DCS или при ошибках парсинга терминал ведёт себя так же, как до изменений.

## Как протестировать

1. Автоматические тесты:
   - Добавить модуль тестов в `otty-escape`, который:
     - подаёт на парсер DCS‑строку с корректным JSON и проверяет, что вызывается `EscapeActor::handle(Action::BlockEvent(..))` с ожидаемыми полями;
     - подаёт DCS с некорректным JSON / слишком длинной строкой и проверяет, что `BlockEvent` не генерируется.

2. Интеграционный тест (опционально):
   - Использовать `FakeSession` в `otty-libterm` + `DefaultParser`, который генерирует bytes с DCS для блоков;
   - убедиться, что `TerminalEngine::on_readable` не падает, а `TerminalSurfaceActor` получает `BlockEvent`.

3. Ручной тест:
   - Собрать workspace: `cargo build --workspace`.
   - Запустить терминал без shell‑хуков: убедиться, что поведение идентично текущему (вывод обычных команд как и раньше).
   - Сымитировать DCS (например, вывести вручную echo с нужной последовательностью) и проверить по логам, что событие обрабатывается.

## Зависимости

- RFC: `specs/spec.md`, раздел 5 (DCS‑протокол).
- План: `specs/plan.md`, раздел 1.
- Не зависит от `BlockSurface` и изменений в UI — задача может быть выполнена первой, изолированно.

