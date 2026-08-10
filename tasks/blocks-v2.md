# Blocks v2: стабильные и расширяемые блоки OTTY

Статус: архитектурный план и поэтапный roadmap; фаза 0 ожидает ручное GUI-подтверждение,
остальные фазы частично выполнены.

## Статус реализации на 2026-08-10

Blocks v2 **не завершён** и пока не соответствует Definition of Done из раздела 16. Текущая
итерация реализует рекомендованный в разделе 17 первый вертикальный срез, полный
автоматический scope baseline-фазы и отдельные части последующих этапов. Оставшиеся пункты
roadmap должны выполняться отдельными изменениями в порядке зависимостей из раздела 15.

В текущую итерацию вошли:

- typed IDs, lifecycle reducer, sparse metadata merge и защита от duplicate/stale protocol
  events;
- bounded protocol v2 parser, session validation, per-shell sequence diagnostics и OSC 133
  A/B;
- Bash/Zsh protocol v2 lifecycle, process-scoped guards, nested shell IDs и атомарный
  bootstrap integration assets;
- базовое семантическое разделение prompt/command/output для активного блока;
- `ScrollPosition::FollowTail/Anchored`, model-level off-screen `ScrollToBlock` и сохранение
  block anchor при ручной прокрутке;
- latest-frame mailbox ёмкостью один, bounded lossless queues, базовые copy/action controls и
  отображение integration status в UI;
- regression, parser и real-shell PTY tests для реализованного среза;
- ignored baseline-сценарий, snapshot/block memory estimates и queue observability;
- удаление production v1 parser/action/lifecycle path с legacy-ignore regression.

Текущая реализация является переходной и имеет следующие известные границы:

- finished blocks всё ещё не замораживаются в read-only logical lines и продолжают хранить
  mutable `Surface`;
- не реализованы полный `HeaderGrid`/`OutputGrid`, `OutputRouter`, budgets/truncation и
  ownership alt-screen;
- отсутствуют `BlockHeightIndex`, stable `LineId`/`BlockPoint`, ленивый reflow и snapshot только
  для видимого диапазона;
- transport ещё не поддерживает model/viewport revisions, render ticks и partial damage;
- не завершены presentation actions, collapse/pin/reorder/group/split, единый export/save
  pipeline и session persistence;
- не добавлены Fish/PowerShell и сложные tmux/SSH environments;
- stitched-history snapshot ещё не удалён;
- полный миллион строк и остальные финальные нагрузочные критерии раздела 13.5 ещё не
  являются acceptance gate; baseline B2-004/B2-005 зафиксирован отдельно.

Проверки текущего среза: shell syntax, `cargo +nightly fmt`, Clippy с `-D warnings`,
`cargo deny check`, все workspace tests и `cargo llvm-cov` проходят. Line coverage вырос с
51.05% до 51.16%. Фиксированного минимального порога нет: согласно `AGENTS.md`, общий line
coverage после изменений не должен снижаться относительно baseline до изменений. Поэтому
coverage не блокирует B2-101; общий Definition of Done остаётся незакрытым из-за перечисленных
выше нереализованных частей и отсутствующей ручной приёмки.

### Решение по protocol v1

Protocol v1 не является публичным compatibility mode. OTTY не добавляет runtime switch
`v1/v2`, compatibility adapter и migration window. Старые emitter, parser, actions и lifecycle
path удалены из production; framing остаётся только в legacy-ignore tests и historical notes.
Rollback выполняется предыдущим application artifact, а не вторым protocol path в новом
бинарнике. Legacy `otty-dcs;block` безопасно игнорируется и не создаёт semantic event/block.

### Отдельные задачи по фазам

Подробный индекс находится в [tasks/blocks-v2/README.md](blocks-v2/README.md). Каждый файл
содержит собственный scope, текущий статус, критерии готовности, автоматические команды и
пошаговую ручную проверку:

- [Описание и ручная проверка уже реализованного первого вертикального
  среза](blocks-v2/implemented-slice-2026-08-09.md)

1. [Фаза 0 — baseline и regression-сценарии](blocks-v2/phase-00-baseline.md)
2. [Фаза 1 — typed identity и lifecycle reducer](blocks-v2/phase-01-lifecycle.md)
3. [Фаза 2 — protocol v2 parser](blocks-v2/phase-02-protocol-v2.md)
4. [Фаза 3 — Bash/Zsh lifecycle и bootstrap](blocks-v2/phase-03-shell-bootstrap.md)
5. [Фаза 4 — раздельный block content и freeze](blocks-v2/phase-04-content-freeze.md)
6. [Фаза 5 — height index, viewport и selection](blocks-v2/phase-05-viewport.md)
7. [Фаза 6 — latest-frame transport](blocks-v2/phase-06-transport.md)
8. [Фаза 7 — UI actions и presentation model](blocks-v2/phase-07-presentation.md)
9. [Фаза 8 — export и сохранение](blocks-v2/phase-08-export.md)
10. [Фаза 9 — дополнительные shells и окружения](blocks-v2/phase-09-shells.md)
11. [Фаза 10 — финальная активация v2 и удаление legacy](blocks-v2/phase-10-rollout.md)

Обозначения в этом документе:

- **A / OTTY** — текущая реализация блоков в OTTY;
- **B / Warp** — реализация блоков в локальном checkout `../warp`, используемая как источник проверенных архитектурных принципов, а не как код для прямого копирования;
- **Blocks v2** — целевая реализация OTTY.

## 1. Итоговое решение

Текущий BlockUI OTTY нельзя сделать сравнимым по стабильности с Warp только правками `block_layout.rs`, рамок, hover или кнопок. Нужны четыре системных изменения:

1. Один канонический `BlockList`, в котором блоки имеют стабильные ID, явное состояние жизненного цикла и раздельные секции prompt/command/output.
2. Viewport с якорем на блок и строку, а не только с числом строк от нижнего края.
3. Версионированный shell-протокол с `session_id`, `shell_instance_id`, порядковым номером события, явным `command_end` и восстановлением после пропущенных событий.
4. Ограниченный transport кадров: устаревшие кадры заменяются последним, а завершение процесса и другие критические события не теряются.

После этого операции над блоком — копирование команды, копирование output, сохранение, сворачивание, перенос, повторный запуск и группировка — должны работать через `BlockId` и секции блока, а не через геометрию текущего кадра или эвристику «первая строка — prompt».

### Главные продуктовые инварианты

- Уже завершённый terminal-content блока неизменяем. Пользовательские свойства вроде `collapsed`, `pinned`, label и порядка представления хранятся отдельно.
- Новый output не двигает экран пользователя, если он ушёл с нижнего края истории.
- Любая команда UI адресуется стабильным `BlockId`; наличие блока в текущем viewport для операции не требуется.
- Ровно один блок владеет обычным PTY-output активного shell-контекста.
- Повторное, запоздавшее или повреждённое shell-событие не может завершить другой блок.
- При неработающей shell-интеграции терминал продолжает работать как обычный terminal и явно показывает degraded-state.
- Очередь UI-кадров имеет жёсткую верхнюю границу; медленный UI не должен останавливать чтение PTY и накапливать сотни полных snapshots.

## 2. Исходные проблемы OTTY, которые устраняет roadmap

Исходный поток данных до первого vertical slice:

```text
shell hook
  -> raw JSON в DCS
  -> otty-escape BlockEvent
  -> BlockSurface { Vec<Block<Surface>>, display_offset }
  -> полный SnapshotOwned после чтения PTY
  -> unbounded flume channel
  -> промежуточный bounded Tokio channel
  -> TerminalView + viewport-relative BlockRect
```

### 2.1. Модель блоков

Основной код находится в `otty-surface/src/block.rs`.

- Каждый блок владеет полноценным `Surface`. У каждого такого `Surface` по умолчанию может быть до 10 000 строк history (`otty-surface/src/surface.rs`). При лимите в 1000 блоков это слишком дорогая и плохо ограниченная модель памяти.
- `resize()` проходит по всем историческим блокам и resize-ит каждый `Surface`. Стоимость resize растёт вместе со всей историей, хотя на экране видны единицы блоков.
- Высота блока вычисляется сканированием grid, включая проверку пустых viewport-строк. `block_slices()` повторяет это для всей истории при построении кадра.
- Все блоки каждый раз «сшиваются» в один viewport. Snapshot всегда получает `SnapshotDamage::Full`.
- Завершённый блок остаётся полноценным terminal emulator state, хотя после завершения ему обычно нужны только неизменяемые строки, стили, hyperlinks и метаданные.

### 2.2. Lifecycle сейчас неполный

`assets/shell-integrations/otty.bash` и `otty.zsh` отправляют только:

- `preexec` с command/cwd;
- `precmd` с cwd.

Они не отправляют `exit`, несмотря на то что Rust-парсер и `BlockSurface` такую фазу понимают. Следствия:

- exit code практически не попадает в блок;
- окончание команды синтезируется по приходу следующего `precmd`;
- зависшая, завершившая shell или перешедшая в subshell команда может остаться в неверном состоянии;
- нельзя надёжно отличить «команда завершилась», «пользователь нажал Enter на пустой строке», «prompt перерисовался» и «shell-интеграция перестала работать».

Дополнительно `end_block_by_id()` заменяет `block.meta` целиком данными события. Если добавить текущим скриптам sparse `exit`, он сотрёт ранее известные `cmd`, `cwd` и `started_at`. Обновление metadata должно быть patch/merge, а не replacement.

ID вида `cmd-1` и `prompt-1` уникальны только внутри одного процесса shell. Новый или вложенный shell начинает счётчик заново. В одном terminal session это приводит к коллизиям, после чего ветка обработки duplicate ID может завершить старый блок.

### 2.3. Scroll хранит число, а не намерение пользователя

`BlockSurface::display_offset` — количество строк от нижнего края. Этого недостаточно, когда:

- активный блок растёт;
- старый output truncate-ится;
- resize меняет wrapping и высоты всех блоков;
- блок сворачивается, скрывается или переносится;
- вставляется synthetic/rich-content блок.

Нужно хранить не только offset, но и намерение: «следовать за концом» либо «оставить такую-то логическую строку такого-то блока на таком-то месте экрана».

### 2.4. UI знает только текущий кадр

В `otty-ui/terminal/src/view.rs` и `block_layout.rs` блоковые прямоугольники получаются из viewport-relative snapshot.

- `ScrollTo(block_id)` ищет прямоугольник блока в текущем layout. Off-screen блок не имеет полезной видимой геометрии, поэтому навигация к нему фактически не работает.
- `BlockUiVisuals::from_rects()` не создаёт `action_buttons`, а `input.rs` всегда присваивает `action_hover = None`. Геометрия кнопки существует, но интерактивный путь не завершён.
- Copy prompt сейчас основан на первой строке command-блока. Для multiline prompt, multiline command, right prompt и перерисовок line editor это неверная граница.
- Координата selection вычисляется по `display_offset` последнего известного snapshot, а применяется позже в backend. Между этими моментами output или scroll может изменить layout.

### 2.5. Кадры и backpressure

`otty-libterm/src/terminal/mod.rs` строит полный snapshot после обработки PTY и кладёт его в очередь. Default `ChannelConfig` создаёт unbounded flume channels. Затем `otty-ui/terminal/src/engine.rs` пересылает события через bounded Tokio channel ёмкостью 100 с `blocking_send`.

При большом output и медленной отрисовке это даёт неправильную семантику:

- промежуточные полные кадры уже устарели, но продолжают занимать память и обрабатываться;
- upstream-очередь остаётся unbounded;
- latency растёт именно тогда, когда output наиболее интенсивен;
- критические события и заменяемые кадры идут одним способом доставки.

## 3. Какие принципы Warp дают стабильность

Ниже сравнивается только код из текущего checkout `../warp`.

| Область | OTTY сейчас | Warp | Решение для Blocks v2 |
|---|---|---|---|
| Каноническая история | `Vec<Block>`, каждый с полным `Surface` | `BlockList` с индексом ID и отдельным индексом высот | Один `BlockList`, быстрый lookup по ID, отдельный height index |
| Состояние блока | `kind` + `is_finished` | Явный `BlockState`: before/executing/done/background/static | Явный state machine без комбинаций конфликтующих bool |
| Содержимое | Один grid, prompt/output разделяются эвристикой | Header/prompt/rprompt и output представлены раздельно | `HeaderGrid` с отмеченными ranges + отдельный `OutputGrid` |
| ID | Строковый локальный счётчик | Session ID + монотонный ID, manual blocks — UUID | Typed ID с namespace terminal session и shell instance |
| Высоты | Полный линейный пересчёт | `SumTree<BlockHeightItem>` и измерение нужного диапазона | Собственный height index без новой dependency, O(log n) lookup/update |
| Scroll | Offset от низа | Явные follow/fixed/long-running состояния | `FollowTail` или устойчивый block-local anchor |
| Finished block | Продолжает быть mutable `Surface` | Завершённый terminal block считается immutable | Freeze в компактное read-only содержимое |
| Rich content | Только terminal block snapshots | В block list есть typed rich-content items и cached heights | Сначала явный enum реальных типов, без speculative plugin framework |
| Shell lifecycle | `precmd/preexec`, exit не посылается | `InitShell`, `Bootstrapped`, `Preexec`, `CommandFinished`, `Precmd`, `ExitShell` | Версионированный lifecycle с handshake и recovery |
| Prompt boundaries | Не маркируются | Используются OSC 133 prompt markers | Поддержать OSC 133 A/B и хранить точные section ranges |
| Integrity | Любой подходящий DCS принимается | Client-generated random session ID регистрируется в model | Принимать события только зарегистрированной terminal session |
| Subshell | Нет модели shell-контекста | Есть init/bootstrap subshell, session info и separators | Stack/tree `ShellContext`, parent block и capability status |

Важно перенять именно эти инварианты. Копировать весь Warp `BlockList`, bootstrap и UI не нужно: в Warp есть логика AI/rich content, SSH, sharing и нескольких frontend-ов, которой OTTY сейчас не требуется.

## 4. Целевая архитектура Blocks v2

```text
PTY byte stream
  -> VTE parser
     -> printable/control actions --------------------+
     -> Protocol v2 + OSC 133 semantic events --------|
                                                       v
                                             BlockLifecycle reducer
                                                       |
                                                       v
                      +---------------- canonical BlockList ----------------+
                      | ID index | height index | ShellContext tree          |
                      | HeaderGrid + ranges | OutputGrid | immutable history |
                      +------------------------------------------------------+
                                  |                         |
                          latest viewport frame       lossless events
                                  |                         |
                                  +----------- UI ----------+
                                             BlockId actions
```

### 4.1. Ответственность слоёв

`otty-escape`:

- безопасно выделяет framing;
- декодирует protocol v2 и OSC 133;
- ничего не знает о текущем активном блоке;
- выдаёт typed semantic events.

`otty-surface`:

- единолично владеет `BlockList` и применяет PTY actions;
- маршрутизирует байты в header, output, alt-screen или background block;
- управляет lifecycle, высотами, selection и viewport anchor;
- предоставляет snapshot только видимого окна и отдельные block queries.

`otty-libterm`:

- гарантирует порядок PTY actions и lifecycle events;
- coalesce-ит render invalidations;
- хранит latest-frame mailbox и отдельную lossless очередь событий;
- не даёт UI затормозить чтение PTY.

`otty-ui/terminal`:

- рисует уже рассчитанное видимое окно;
- hit-test-ит по stable `BlockPoint` и revision;
- отправляет typed block actions;
- не вычисляет глобальную позицию off-screen блока из rectangles.

Приложение `otty`:

- устанавливает и диагностирует shell integration;
- показывает статус protocol capabilities;
- выполняет файловый export по явно выбранному пути;
- управляет пользовательским порядком/группами блоков, не ломая canonical transcript.

## 5. Целевая модель данных

Названия ниже задают контракт. В реализации их следует разнести из нынешнего монолитного `otty-surface/src/block.rs` по небольшим модулям с одной ответственностью.

```rust
struct BlockId(String);
struct TerminalSessionId(String);
struct ShellInstanceId(String);

struct TerminalBlock {
    id: BlockId,
    shell: ShellRef,
    parent_block_id: Option<BlockId>,
    state: BlockState,
    header: HeaderGrid,
    output: OutputGrid,
    command: Option<CommandRecord>,
    metadata: BlockMetadata,
    revision: u64,
}

enum BlockState {
    BeforeExecution,
    Executing,
    Finished(BlockOutcome),
    BackgroundRunning,
    BackgroundFinished,
    Static,
}

enum BlockOutcome {
    Exited(i32),
    Signaled(i32),
    Cancelled,
    ShellExited,
    Unknown,
}
```

### 5.1. ID

Формат terminal block ID:

```text
<terminal-session-id>:<shell-instance-id>:<monotonic-block-seq>
```

Требования:

- ID создаётся для будущего input-block на `prompt_prepare`, а не меняется на `preexec`;
- prompt, typed command и output одной команды имеют один ID;
- synthetic blocks получают отдельный app-generated ID;
- публичный API принимает `&BlockId`, а не произвольную строку;
- lookup выполняется через `HashMap<BlockId, BlockIndex>`;
- при удалении или reorder индекс обязательно обновляется и проверяется тестом инварианта.

### 5.2. Lifecycle

Нормальный путь:

```text
shell_hello
  -> prompt_prepare(block-1)
  -> OSC 133 prompt_start
  -> OSC 133 prompt_end
  -> command_start(block-1)
  -> command_end(block-1, exit_code, next_block_id=block-2)
  -> prompt_prepare(block-2)
```

Правила reducer-а:

- `command_start` переводит только указанный `BeforeExecution` block в `Executing`;
- `command_end` только patch-ит completion fields и никогда не заменяет всю metadata;
- повтор события с теми же `(shell_instance_id, seq)` идемпотентен;
- событие со старым `seq` игнорируется;
- gap в `seq` помечает integration degraded и включает recovery, но не останавливает terminal;
- `prompt_prepare` после незавершённой команды синтезирует `Finished(Unknown)` и считает recovery в diagnostics;
- `command_end` для неизвестного ID сохраняется как orphan diagnostic и не применяется к «похожему» блоку;
- закрытие shell завершает его активный блок как `ShellExited` и возвращает routing родительскому shell context;
- terminal child exit завершает все оставшиеся contexts в порядке от вложенного к корневому.

### 5.3. Разделение prompt, command и output

Один terminal block должен содержать:

- `HeaderGrid` — то, что реально было нарисовано до начала выполнения: prompt, right prompt, редактирование и echo команды;
- `PromptRange` и `CommandEchoRange` внутри header, отмеченные semantic markers;
- `CommandRecord` — канонический текст команды из лучшего доступного источника;
- `OutputGrid` — байты после `command_start` и до `command_end`;
- optional `AltScreenCapture`/summary для TUI-приложения;
- metadata: cwd before/after, shell, timestamps, exit status, duration, protocol confidence.

Приоритет источников `CommandRecord`:

1. input-buffer, известный самому OTTY перед отправкой Enter;
2. shell `command_start.command`;
3. извлечение из `CommandEchoRange` как fallback.

Нужно сохранять `CommandSource`, чтобы UI не выдавал эвристически извлечённую строку за точную. Bash preexec может терять части сложной команды; это ограничение следует тестировать и показывать в diagnostics.

`CopyPrompt`, `CopyCommand` и `CopyOutput` больше не должны зависеть от первой видимой строки или текущего snapshot.

### 5.4. Active и frozen content

Для активного блока нужен mutable emulator state. После завершения:

- cursor, tab stops, modes и другие runtime-only данные освобождаются;
- logical lines, cell styles, hyperlinks и section ranges freeze-ятся;
- height для текущей ширины кешируется;
- reflow выполняется лениво для видимых и близких к viewport блоков;
- у блока хранится число удалённых сверху output lines, чтобы scroll/selection могли скорректироваться после truncation.

Лимиты должны быть двух уровней:

- per-block output line/byte budget;
- global transcript memory budget.

При превышении head output удаляется предсказуемо, `truncated_lines` увеличивается, а UI показывает marker. Нельзя молча удалять выбранную или сохраняемую строку.

### 5.5. Alt screen

Alt screen — режим активного command block, а не новый независимый lifecycle block.

- Пока alt screen активен, viewport показывает только его buffer и временно отключает обычную block navigation.
- После выхода block возвращается к normal output.
- По умолчанию в history сохраняется финальный normal-screen output и метаданные запуска; полную историю кадров TUI хранить не нужно.
- Если процесс умер в alt screen, состояние должно корректно вернуться в normal viewport и завершить owning block.

## 6. Height index и стабильный viewport

### 6.1. Индекс высот

Вместо пересчёта всех `block_slices()` на каждый frame нужен `BlockHeightIndex`:

- высота каждого item в visual lines;
- prefix sum для поиска item по global row;
- update высоты активного/переформатированного блока;
- insert/remove/move;
- поиск global top блока по `BlockId`;
- поиск диапазона блоков, пересекающих viewport.

Для P0 не нужна новая dependency. При текущем масштабе достаточно собственного Fenwick tree либо prefix index с точечной invalidation; выбор закрепить benchmark-ом. Добавление crate возможно только после отдельного согласования, как требует `AGENTS.md`.

### 6.2. Scroll state

```rust
enum ScrollPosition {
    FollowTail,
    Anchored(ViewportAnchor),
}

struct ViewportAnchor {
    block_id: BlockId,
    section: BlockSection,
    logical_line_id: LineId,
    wrapped_row_in_line: u32,
    viewport_row: u16,
    truncated_generation: u64,
}
```

Поведение:

- ввод пользователем новой команды возвращает `FollowTail`;
- ручной scroll вверх создаёт anchor на верхней видимой логической строке;
- рост output ниже anchor не меняет его экранную координату;
- изменение wrapping после resize повторно разрешает `LineId` в visual row;
- если anchored line truncate-нулась, anchor переходит на первую оставшуюся строку и показывает truncation marker;
- если блок скрыт/удалён, выбирается ближайший видимый сосед в детерминированном направлении;
- collapse/expand, filter, reorder и вставка rich item проходят через один `Viewport::apply_change`, а не исправляют offset в разных местах;
- `ScrollTo(BlockId)` вычисляется через height index и работает для любого retained block.

### 6.3. Selection и hit testing

Selection хранится в block-local координатах:

```text
BlockPoint { block_id, section, logical_line_id, cell_offset }
```

Snapshot получает `model_revision` и `viewport_revision`. Pointer event либо содержит уже разрешённый `BlockPoint`, либо revision, по которому backend может отклонить устаревшую viewport-coordinate и запросить новый hit-test. Нельзя применять старую глобальную строку к уже изменившемуся layout.

Cross-block selection хранит два `BlockPoint` и порядок canonical/presentation list. Copy собирает данные из модели, а не только из cells текущего frame.

## 7. Render transport и snapshots

### 7.1. Разделить заменяемые и lossless события

Заменяемые:

- `FrameReady` / latest viewport snapshot;
- cursor blink/redraw invalidation;
- progress/hover visual state.

Lossless:

- child exit;
- title/reset title;
- integration status change;
- export/save result;
- ошибки backend-а.

Для кадра нужен mailbox ёмкостью 1: новая версия заменяет старую, если UI ещё её не забрал. Для lossless events остаётся bounded queue с явной обработкой backpressure. Не следует использовать unbounded queue как скрытый буфер полных `SnapshotOwned`.

### 7.2. Когда строить frame

- PTY bytes обрабатываются непрерывно одним writer-ом модели.
- Несколько read chunks coalesce-ятся в render tick, например 8–16 ms при активном потоке.
- UI scroll/selection/resize могут запросить immediate frame.
- Snapshot включает только visible window плюс небольшой overhang.
- Список `BlockSummary` или отдельный query API позволяет найти off-screen блок без передачи его cells.
- Damage должен отражать реально изменённые rows; `Full` остаётся для resize/theme/reset и перехода alt screen.

### 7.3. Snapshot API

Минимально нужны:

- `ViewportSnapshot { revision, cells, visible_blocks, cursor, selection, damage }`;
- `BlockSummary { id, state, command, cwd, exit, height, flags }` без полного output;
- `GetBlockText { id, section, format }`;
- `ScrollToBlock { id, alignment }`;
- `ApplyBlockAction { id, action }`.

Finished block text нельзя копировать через дублирование полного `cached_text` в каждом frame. Текст извлекается по запросу из frozen content; небольшой result возвращается отдельным event-ом.

## 8. Расширяемость и операции над блоками

### 8.1. Не смешивать transcript и пользовательскую композицию

Canonical transcript должен оставаться хронологическим журналом PTY. Произвольный reorder живых execution blocks ломает routing и смысл shell history. Поэтому нужны два уровня:

- `Transcript` — append-oriented source of truth;
- `BlockPresentation` — порядок, hidden/collapsed/pinned, группы и ссылки на canonical blocks.

Пользователь может переставлять завершённые блоки в presentation, группировать их и скрывать. Активный блок остаётся закреплён в live-tail. Удаление из presentation не удаляет transcript content; окончательный purge — отдельное подтверждаемое действие.

### 8.2. Built-in actions

Первый стабильный набор:

- select block;
- copy selection;
- copy command;
- copy output как plain text;
- copy prompt/header;
- copy whole block;
- save command/output/whole block;
- rerun command в текущем или новом terminal;
- collapse/expand output;
- pin/unpin;
- hide/restore;
- move before/after другого завершённого блока в presentation;
- group/ungroup;
- split presentation at output line или selection;
- add/edit label/note.

`split` не режет исходный frozen terminal buffer физически. Он создаёт несколько presentation slices, ссылающихся на диапазоны одного canonical block. Так copy/save остаются точными, а исходная запись не повреждается.

### 8.3. Типы content

Не следует сразу строить динамическую plugin-систему с `Any`, фабриками и generic renderer traits. Для реальных текущих сценариев достаточно явного enum:

```rust
enum BlockItem {
    Terminal(TerminalBlock),
    StaticText(StaticTextBlock),
    IntegrationNotice(IntegrationNoticeBlock),
}
```

Когда появятся минимум два действительно внешних renderer-а, можно выделить interface. До этого новый вариант должен явно реализовать layout, selection, export и persistence — это предотвращает «расширяемость», при которой часть операций молча не работает.

### 8.4. Action API

UI использует exhaustive `BlockAction` enum. Доступность кнопок вычисляет чистая функция из `BlockSummary`:

```text
available_actions(summary, capabilities) -> Vec<BlockAction>
```

Кнопка и context menu вызывают один command path. Внутренний BlockUI и внешний overlay не должны иметь разные реализации copy/save/select.

## 9. Protocol v2

### 9.1. Достаточен ли текущий protocol OTTY

Нет. Для demo он достаточен, для устойчивой block model — нет.

Текущая рамка:

```text
ESC P otty-dcs;block;<raw-json> ESC \
```

Проблемы:

- shell пишет `"v": 1`, но Rust schema не содержит `v`; версия фактически игнорируется;
- нет terminal session и shell instance;
- нет порядка событий и защиты от duplicate/stale event;
- нет handshake/capabilities;
- нет фактически испускаемого `exit`;
- нет parent context для subshell;
- нет prompt boundaries;
- raw JSON и общий лимит 4096 байт дают хрупкий transport для длинных/Unicode commands;
- неизвестные/повреждённые сообщения только логируются и теряются, model не знает, что lifecycle стал неполным;
- любой процесс, способный вывести подходящий DCS, может подделать block event.

Warp заметно полнее: hex-encoded JSON hooks, random registered session ID, `InitShell`/`Bootstrapped`, `CommandFinished` с `next_block_id`, `Precmd`, `Preexec`, `ExitShell`, input-buffer/clear events, OSC 133 prompt boundaries и отдельные subshell/SSH bootstrap paths. Это не означает, что Warp transport надо копировать побайтно, но текущего OTTY protocol относительно него недостаточно.

### 9.2. Framing

Предлагаемый native framing:

```text
ESC P otty-dcs;event-v2;h;<hex-encoded-utf8-json> ESC \
```

Где `h` означает hex JSON. Hex увеличивает payload вдвое, зато payload не может случайно содержать ST, BEL или управляющую последовательность.

Ограничения P0:

- максимум 32 KiB decoded JSON и 64 KiB encoded data;
- максимум 16 KiB command text;
- sender выставляет `command_truncated: true`, а receiver повторно применяет лимит;
- при overflow parser переходит в discard-until-ST без неограниченного allocation;
- chunking в P0 не нужен: lifecycle event должен оставаться маленьким; полный input при необходимости берётся из UI-side buffer;
- v1 framing не имеет compatibility adapter: после удаления legacy parser он безопасно
  отбрасывается как unsupported DCS и не попадает в block model.

### 9.3. Envelope

```json
{
  "v": 2,
  "event": "command_end",
  "terminal_session_id": "01J...",
  "shell_instance_id": "01J...:bash:4812:1",
  "seq": 18,
  "block_id": "01J...:01J...:7",
  "sent_at_unix_ms": 1785680123456,
  "payload": {
    "exit_code": 0,
    "pipe_status": [0],
    "next_block_id": "01J...:01J...:8"
  }
}
```

Обязательные поля проверяются строго. Неизвестные дополнительные поля того же major version допускаются для forward compatibility. Неизвестный major version не применяется к модели и переводит integration в `UnsupportedVersion`.

### 9.4. Набор событий

| Event | Когда | Основные поля |
|---|---|---|
| `shell_hello` | Сразу после загрузки integration | shell/version, pid, parent shell, capabilities |
| `prompt_prepare` | В начале precmd, после completion предыдущей команды | next/current block ID, cwd, prompt metadata |
| `command_start` | В preexec | block ID, exact/best-effort command, cwd |
| `command_end` | Первое действие следующего precmd | block ID, exit/signal, pipeline statuses, next block ID |
| `context_update` | cwd/venv/git context изменился вне обычного precmd | только изменившиеся поля |
| `input_buffer` | По явному запросу OTTY, если shell поддерживает | line-editor buffer и cursor |
| `shell_exit` | EXIT/zshexit/эквивалент | shell instance, active block, status |
| `integration_error` | Hook не удалось установить/выполнить | безопасный reason code, без command text |

OSC 133 A/B используется параллельно только как точный marker начала/конца prompt в grid. Он не заменяет rich lifecycle event и не должен дважды открывать блок.

### 9.5. Session validation и threat model

OTTY генерирует случайный `TerminalSessionId` до запуска shell и регистрирует его в terminal model. Событие другой или отсутствующей session отвергается до lifecycle reducer-а.

ID должен создаваться из системного cryptographic RNG, а не из PID и времени. Если в workspace нет уже одобренного прямого источника random bytes, перед добавлением `getrandom`, `rand`, `uuid` или другой dependency нужно отдельно запросить согласование согласно `AGENTS.md`.

Это защита от случайных/stale escape sequences, а не полноценная аутентификация. Процесс, который выполняется в текущем shell, может прочитать environment и писать в PTY, поэтому способен имитировать integration. HMAC не решает это без отдельного защищённого side channel. В P0 достаточно:

- случайного непредсказуемого session ID;
- строгого ID/seq/lifecycle validation;
- запрета логировать полный payload и команды по умолчанию;
- bounded parser;
- явного degraded status при нарушении последовательности.

### 9.6. Recovery

Protocol работает поверх упорядоченного PTY byte stream, поэтому ack/retry для каждого hook не нужен. Нужны детерминированные recovery rules:

- duplicate seq — ignore;
- seq gap — diagnostic + degraded;
- `prompt_prepare` без `command_end` — завершить active block с unknown outcome;
- `command_start` без prepared block — создать recovered block с тем же ID;
- `command_end` дважды — второй не меняет metadata;
- shell exit без command end — завершить active block как `ShellExited`;
- новый `shell_hello` с child parent ID — push context;
- возврат parent output после child exit — resume parent routing;
- malformed event никогда не печатается как обычный terminal text и не рушит parser state.

## 10. Shell integration и subshell

### 10.1. Что работает сейчас

| Сценарий | Текущий статус |
|---|---|
| Root interactive Bash | Запускается через generated `--rcfile` |
| Root interactive Zsh | Запускается через generated `ZDOTDIR/.zshrc` |
| Nested Zsh | Часто повторно читает унаследованный `ZDOTDIR`, но ID сталкиваются, guard не process-scoped |
| Nested Bash | Обычный `bash` читает `~/.bashrc`, а не root `--rcfile`; integration обычно теряется |
| Fish/PowerShell/Nushell/POSIX sh | Не поддерживаются |
| SSH/containers/sudo/su | Integration автоматически не переносится |
| tmux/screen | Custom DCS может быть поглощён или потребовать passthrough |

Поэтому ощущение, что shell integration «не прокидывается в subshell», для Bash подтверждается архитектурой запуска.

### 10.2. Process-scoped idempotency

Текущего `OTTY_*_HOOK_INITIALIZED=1` недостаточно. Guard должен сравнивать PID текущего interactive shell:

```text
if installed_for_pid == current_shell_pid: return
parent_shell_id = inherited_current_shell_id
current_shell_id = make_id(session_id, shell, pid, depth)
export current_shell_id for future child shells
installed_for_pid = current_shell_pid   # не экспортировать как глобальный bool
```

Повторный source в том же процессе ничего не добавляет. Новый interactive child с другим PID создаёт новый context и свои counters.

Hook installation обязана:

- сохранять `$?` и pipeline statuses до любых subprocess/local helper calls;
- не затирать существующий `PROMPT_COMMAND`, DEBUG trap, `precmd_functions` и `preexec_functions`;
- корректно переживать повторный source после Oh My Zsh, starship или другого prompt framework;
- не запускать `jq`, Python или Perl на каждый prompt;
- использовать dependency-free escaping/encoding с тестовым fallback;
- chain-ить существующий EXIT trap;
- не добавлять bootstrap-команды в history;
- не менять пользовательские `HISTCONTROL`, shell options и prompt после завершения bootstrap.

### 10.3. Надёжная доставка integration в child shell

Полностью прозрачного универсального способа заставить произвольный child shell прочитать дополнительный rc-файл нет. Поэтому должны существовать два режима.

**Zero-config / best effort**

- root Bash получает `--rcfile`;
- root и обычный nested Zsh используют wrapper через `ZDOTDIR`;
- при новом `shell_hello` строится child context;
- если preexec указывает запуск interactive shell, но `shell_hello` не пришёл за ограниченный период/до следующего prompt, UI показывает `subshell integration unavailable`.

**Persistent integration / recommended**

- OTTY предлагает пользователю явную one-line установку loader-а в `.bashrc`/`.zshrc` с begin/end markers;
- loader активируется только если установлены OTTY session variables, поэтому вне OTTY ничего не делает;
- изменение пользовательского rc-файла выполняется только после подтверждения, атомарно и с возможностью uninstall;
- loader source-ит versioned script из config/cache OTTY;
- этот режим является обязательным для гарантированного nested Bash.

`BASH_ENV` нельзя считать решением: он относится к non-interactive Bash и не гарантирует hooks в новом interactive `bash`. Подмена executable через PATH или shell function `bash()` слишком рискованна и не должна использоваться.

### 10.4. Shell context tree

```text
root zsh (shell-1)
  block: `bash`
    child bash (shell-2, parent=shell-1, spawning_block=`bash`)
      block: `python`
  parent resumes and finishes block `bash`
```

`ShellContext` содержит active block и parent/spawning block. Пока child активен, его output маршрутизируется в child blocks. Родительский block становится container/group для child session. После `shell_exit` child закрывается, а последующие bytes снова принадлежат родителю до его `command_end`.

Background processes остаются физическим ограничением одного PTY stream: байты нескольких процессов могут смешаться без дополнительной маркировки. Когда output приходит при готовом prompt и не относится к active execution, он должен попадать в typed `Background` block, а не менять последний finished command.

### 10.5. Матрица поддержки

Порядок реализации:

1. Bash и Zsh local root + nested — P0.
2. Fish — P1: native `fish_preexec`/`fish_prompt` events.
3. PowerShell — P1: PSReadLine/prompt wrapper с сохранением `$LASTEXITCODE`.
4. tmux/screen — P1: detection, documented passthrough и integration self-test; настройки пользователя автоматически не менять.
5. SSH/container — P2: явный remote bootstrap/`otty ssh` либо установленный remote loader. Не обещать автоматическую интеграцию без remote code.
6. Nushell и другие shells — P2 после отдельного lifecycle design; до этого обычный terminal с `Unsupported` status.

Для каждого terminal session UI должен показывать `Pending`, `Active(v2)`, `Degraded(reason)` или `Unsupported(shell)`. Молчаливый fallback недопустим.

### 10.6. Установка файлов

Сейчас integration files перезаписываются через `fs::write` в `~/.config/otty`. Для нескольких одновременно открываемых terminal tabs это даёт ненужную гонку.

Нужно:

- versioned immutable files, например `shell-integrations/v2/otty.bash`;
- запись во временный файл и atomic rename;
- hash/version check до записи;
- generated wrapper с фиксированной версией;
- permissions без world-writable;
- fallback на обычный shell при IO error плюс видимый degraded status;
- unit test конкурентной подготовки нескольких sessions.

## 11. Copy, save и persistence

### 11.1. Единый export pipeline

Все copy/save используют один запрос:

```text
ExportBlock {
  block_id,
  range: Command | Prompt | Output | Whole | Selection,
  format: PlainText | AnsiText | Markdown | Json,
}
```

Clipboard получает результат этого pipeline. Save пишет тот же результат атомарно в выбранный пользователем файл. Это исключает расхождение «Copy показывает одно, Save сохраняет другое».

Правила plain text:

- terminal trailing fill spaces удаляются, значимые внутренние пробелы сохраняются;
- soft-wrap не превращается в hard newline;
- wide-char spacer не копируется;
- hidden/collapsed состояние не влияет на экспорт;
- truncated output содержит явный marker и количество потерянных строк;
- prompt не включается в `Output`;
- `Whole` имеет стабильный порядок prompt/command/output;
- command берётся из `CommandRecord`, а не из визуального prompt.

### 11.2. Форматы

- Plain text — основной clipboard/save output.
- ANSI text — opt-in, восстанавливает SGR/hyperlinks настолько, насколько это возможно.
- Markdown — command fenced block + output fenced block + cwd/exit/duration.
- JSON — versioned schema `otty.block`, отдельно от live protocol version.

Не следует сериализовать внутренний `Surface` напрямую: его runtime layout, cursor и mode fields не являются стабильным storage format.

### 11.3. Session persistence

Первая версия может сохранять export-файлы без базы данных. Полное восстановление sessions следует делать после freeze-модели:

- metadata + logical styled lines + section ranges;
- presentation order/groups/slices отдельно от canonical content;
- schema version и миграции;
- streaming write или periodic checkpoints, чтобы большой session не сериализовался целиком на UI thread;
- политика секретов/redaction до включения автоматического сохранения.

Новые dependencies для БД, compression или serialization нельзя добавлять без отдельного согласования.

## 12. Наблюдаемость

Нужны counters и debug diagnostics, но command/output не должны попадать в обычные logs.

- protocol events accepted/rejected по reason;
- duplicate/stale/sequence-gap events;
- synthesized completions;
- integration status и negotiated capabilities;
- blocks/lines/bytes in memory;
- truncation per block/global;
- height-index update и visible-range lookup latency;
- snapshot build duration/bytes;
- replaced frames и максимальная глубина lossless queue;
- UI time from latest model revision to presented frame.

Debug panel для session должен показывать shell, integration version, shell instance tree, last seq/event и degraded reason. Это позволит отличить «BlockUI сломан» от «hooks не загрузились в child shell».

## 13. Тестовая стратегия и критерии стабильности

Правило `AGENTS.md` обязательно: для каждого изменения business logic сначала добавляется падающий тест, затем implementation.

### 13.1. Lifecycle tests

- normal prompt -> command -> exit -> next prompt;
- empty Enter/no preexec;
- Ctrl-C до preexec и во время command;
- duplicate preexec/command_end;
- missing command_end;
- event sequence gap;
- exit event не стирает command/cwd/start time;
- unknown block ID не завершает соседний блок;
- root shell exit и nested shell exit;
- alt-screen enter/exit/crash;
- background output между prompts.

### 13.2. Viewport regression tests

- пользователь находится в середине history, active block растёт на 100 000 lines — anchor не двигается;
- head truncation происходит ниже/выше/на anchored line;
- resize 80 -> 200 -> 40 columns сохраняет логическую anchored line;
- finished block freeze/reflow не меняет соседний visible content;
- collapse/expand блока выше viewport;
- insert/remove/reorder presentation item выше viewport;
- `ScrollTo` для полностью off-screen блока;
- selection остаётся на тех же logical cells после output/resize;
- удаление anchored block выбирает документированный сосед;
- переход alt screen и возврат восстанавливает normal scroll state.

### 13.3. Protocol parser tests

- fragmented DCS по одному байту и нескольким read chunks;
- несколько событий в одном chunk;
- malformed hex/JSON/UTF-8;
- embedded ESC/BEL/control chars в command;
- oversized payload переходит в bounded discard state;
- unsupported version;
- missing session/shell/block/seq;
- Unicode, newline, quotes и очень длинная command;
- legacy v1 framing safely ignored без semantic event, block и panic;
- случайные DCS bytes не вызывают panic и allocation выше лимита.

### 13.4. Реальные shell tests через PTY

Для Bash и Zsh:

- integration source дважды в одном process — одно событие на lifecycle phase;
- nested interactive shell — новый unique `shell_instance_id` и unique block IDs;
- `exit 7`, pipeline, command-not-found, signal и Ctrl-C дают правильный outcome;
- multiline command/heredoc;
- Unicode/quotes/control-like text;
- существующий `PROMPT_COMMAND`, DEBUG trap, zsh hook arrays сохраняются;
- prompt framework перезагрузил hooks — self-check обнаруживает или чинит безопасно;
- `exec bash`/`exec zsh`;
- отсутствие external `jq/python/perl` не ломает encoding;
- child без persistent loader даёт видимый degraded status, не corrupted blocks.

tmux, SSH и дополнительные shells запускаются отдельной capability-gated матрицей, чтобы отсутствие binary в CI не скрывало failures основных Bash/Zsh tests.

### 13.5. Нагрузочные критерии

- 10 000 маленьких finished blocks: visible-range lookup не сканирует все grids;
- 1 000 000 строк output при искусственно медленном UI: frame backlog не превышает один replaceable frame, PTY продолжает читаться;
- память ограничивается заданным global budget и стабилизируется после truncation/freeze;
- snapshot содержит `viewport rows * columns`, а не cells всей history;
- 100 последовательных resize не вызывают O(all blocks * full Surface state) работу на каждый frame;
- нет scroll jumps во всех viewport regression tests;
- copy/save off-screen block работает без предварительного scroll;
- все invalid lifecycle transitions дают diagnostic, но не panic.

## 14. Поэтапный план реализации

Задачи ниже сгруппированы по dependency order. Внутри каждого этапа test-задача выполняется раньше implementation-задачи.
Этот checklist сохраняется как полный master scope. Актуальный item-level прогресс (`[x]`,
частично выполнено и оставшиеся подпункты) ведётся в связанных файлах фаз, чтобы не объявлять
исходный крупный пункт завершённым по неполному вертикальному срезу.

### [Этап 0. Зафиксировать baseline](blocks-v2/phase-00-baseline.md)

- [x] **B2-001** Добавить в `otty-surface/src/block.rs` regression tests для роста active block при manual scroll, resize, truncation и off-screen `ScrollTo` на уровне модели.
- [x] **B2-002** Сохранить v1 framing только как legacy test fixture и добавить regression:
  после удаления v1 старые, fragmented и malformed `otty-dcs;block` sequences не создают
  semantic event/block и не вызывают panic.
- [x] **B2-003** Добавить test harness запуска Bash/Zsh через PTY в `otty/tests/shell_integration.rs`; тест сначала должен воспроизвести потерю nested Bash integration и collision ID.
- [x] **B2-004** Добавить измерение snapshot size/time, frame queue depth и block memory estimate без логирования command/output.
- [x] **B2-005** Сохранить benchmark-сценарии в `otty/benches/blocks.rs` либо в ignored integration tests, используя только уже согласованные dependencies.

Выход этапа: текущие failures воспроизводимы автоматически и есть числа, с которыми сравнивается v2.

### [Этап 1. Typed identity и lifecycle reducer](blocks-v2/phase-01-lifecycle.md)

- [ ] **B2-010** Сначала добавить table-driven tests всех state transitions и recovery cases в новом `otty-surface/src/block/lifecycle.rs`.
- [ ] **B2-011** Ввести private/public по необходимости newtypes `BlockId`, `TerminalSessionId`, `ShellInstanceId` и `ProtocolSequence` в `otty-surface/src/block/id.rs`.
- [ ] **B2-012** Ввести `BlockState`, `BlockOutcome`, `MetadataPatch` и reducer; запретить прямую замену всей metadata на completion event.
- [ ] **B2-013** Добавить `block_id_to_index` и tests его корректности после append/remove/reorder.
- [x] **B2-014** Удалить v1 `BlockEvent` lifecycle path; `LifecycleInput` и reducer принимают
  только protocol-v2 semantic events. Compatibility adapter не добавлять.
- [ ] **B2-015** Удалить логику duplicate string ID, которая вызывает `end_block_by_id()`; stale/duplicate должны решаться reducer-ом.
- [ ] **B2-016** Разбить монолитный `otty-surface/src/block.rs` на focused modules `block/model.rs`, `block/lifecycle.rs`, `block/id.rs` и `block/list.rs`, сохранив минимальный public API.

Выход этапа: protocol-v2 block lifecycle детерминирован, metadata не теряется,
а v1 lifecycle path удалён.

### [Этап 2. Protocol v2 parser](blocks-v2/phase-02-protocol-v2.md)

- [ ] **B2-020** Сначала добавить framing/schema/recovery tests в `otty-escape/src/dcs/event_v2.rs` и parser stream tests в `otty-escape/src/dcs/mod.rs`.
- [x] **B2-021** После падающего legacy-ignore regression из B2-002 удалить v1 parser/schema и
  production dispatch в `Action::BlockEvent`; legacy
  `otty-dcs;block` безопасно отбрасывается как unsupported DCS.
- [ ] **B2-022** Реализовать bounded `event-v2;h` framing, hex decode и typed envelope с обязательным major version.
- [ ] **B2-023** Добавить semantic actions для `shell_hello`, `prompt_prepare`, `command_start`, `command_end`, `context_update`, `shell_exit` и `integration_error`.
- [ ] **B2-024** Добавить OSC 133 A/B parsing в `otty-escape/src/osc.rs` и semantic prompt boundary actions.
- [ ] **B2-025** Зарегистрировать созданный системным RNG session ID в terminal model и отвергать events с missing/unknown session до reducer-а; если подходящей dependency ещё нет, сначала запросить её согласование.
- [ ] **B2-026** Добавить per-shell sequence validation и diagnostics, не содержащие payload.
- [ ] **B2-027** Документировать wire format и limits в `otty-escape/README.md` с валидными Bash/Zsh examples.

Выход этапа: backend надёжно принимает v2, production parser не декодирует v1, а legacy
framing проверен только как safely ignored input.

### [Этап 3. Bash/Zsh lifecycle и bootstrap](blocks-v2/phase-03-shell-bootstrap.md)

- [ ] **B2-030** Сначала расширить real-shell tests: exit code, pipeline status, prompt boundaries, source-twice, nested shell, existing hooks.
- [ ] **B2-031** Переписать `assets/shell-integrations/otty.bash` на protocol v2 с захватом status первым действием `precmd`.
- [ ] **B2-032** Переписать `assets/shell-integrations/otty.zsh` на protocol v2 с корректным `$?`, `pipestatus` и `zshexit` chaining.
- [ ] **B2-033** Добавить OSC 133 A/B вокруг prompt без изменения видимого пользовательского prompt.
- [ ] **B2-034** Заменить global boolean guard на process-scoped PID guard и генерировать child `shell_instance_id` с parent link.
- [ ] **B2-035** Удалить per-prompt вызовы `jq/python/perl`; реализовать и протестировать dependency-free escaping/hex encoding path.
- [ ] **B2-036** Версионировать integration assets и атомарно готовить их в новом focused модуле `otty/src/widgets/terminal_workspace/shell_integration/`.
- [ ] **B2-037** Добавить `Pending/Active/Degraded/Unsupported` status и handshake timeout в terminal workspace state.
- [ ] **B2-038** Добавить явный, подтверждаемый пользователем install/uninstall persistent loader для `.bashrc`/`.zshrc`; не менять rc-файлы автоматически.
- [ ] **B2-039** Добавить concurrent bootstrap test для нескольких terminal tabs.

Выход этапа: root и persistent nested Bash/Zsh имеют полный lifecycle и уникальные IDs; отсутствие hooks видно пользователю.

### [Этап 4. Раздельный block content и freeze](blocks-v2/phase-04-content-freeze.md)

- [ ] **B2-040** Сначала добавить tests section boundaries для single/multiline/right prompt, edited command, output, empty command и background output.
- [ ] **B2-041** Ввести `HeaderGrid`, prompt/command ranges и `OutputGrid` в `otty-surface/src/block/content.rs`.
- [ ] **B2-042** Реализовать `OutputRouter`, выбирающий root/child active block, header/output/background/alt-screen destination.
- [ ] **B2-043** Ввести `CommandRecord` с source/confidence и перестать извлекать command из первой визуальной строки как основной путь.
- [ ] **B2-044** Реализовать freeze finished block в read-only logical lines и освобождение runtime-only `Surface` state.
- [ ] **B2-045** Добавить per-block/global budgets, explicit truncation metadata и корректировку anchors/selections.
- [ ] **B2-046** Сделать alt screen режимом owning command block с корректным restore normal viewport.
- [ ] **B2-047** Заменить `cached_text` в каждом snapshot на query к frozen content.

Выход этапа: prompt/command/output имеют точные границы, finished history компактна и не мутирует.

### [Этап 5. Height index, viewport и selection](blocks-v2/phase-05-viewport.md)

- [ ] **B2-050** Сначала перенести viewport regression matrix из раздела 13.2 в `otty-surface/src/block/viewport.rs` tests.
- [ ] **B2-051** Реализовать `BlockHeightIndex` в `otty-surface/src/block/height_index.rs` с randomized invariant test против простого `Vec` reference model.
- [ ] **B2-052** Ввести `ScrollPosition::FollowTail/Anchored` и единый `Viewport::apply_change`.
- [ ] **B2-053** Добавить stable `LineId` и mapping logical line -> wrapped visual rows для resize/reflow.
- [ ] **B2-054** Перевести selection на `BlockPoint`; удалить зависимость selection state от глобальной stitched-history строки.
- [ ] **B2-055** Реализовать visible-range lookup и snapshot только viewport + overhang.
- [ ] **B2-056** Перевести `ScrollToBlock` на model/height index и добавить align start/center/end/nearest.
- [ ] **B2-057** Удалить resize всех historical `Surface`; reflow finished blocks выполнять лениво и обновлять height cache.
- [ ] **B2-058** Проверить benchmark-ами, что frame path не вызывает full history grid scan.

Выход этапа: рост output, resize, collapse и navigation не вызывают scroll jumps и линейный scan history на каждый frame.

### [Этап 6. Latest-frame transport](blocks-v2/phase-06-transport.md)

- [ ] **B2-060** Сначала добавить slow-consumer tests в `otty-libterm/src/terminal/` с burst output и lossless child exit.
- [ ] **B2-061** Разделить replaceable frame notification и lossless terminal events.
- [ ] **B2-062** Реализовать latest-frame mailbox ёмкостью один без новой dependency; новый frame атомарно заменяет непрочитанный старый.
- [ ] **B2-063** Убрать unbounded default для очередей, содержащих большие owned payloads; задать и документировать bounded semantics.
- [ ] **B2-064** Coalesce-ить PTY reads в render ticks и разрешить immediate render для resize/scroll/selection.
- [ ] **B2-065** Передавать `model_revision`/`viewport_revision`; stale coordinate request должен отклоняться или разрешаться через stable `BlockPoint`.
- [ ] **B2-066** Поддержать partial damage для обычного output, сохранив full damage для resize/reset/alt-screen transition.

Выход этапа: terminal остаётся отзывчивым при burst output, UI всегда стремится к последней revision.

### [Этап 7. UI actions и presentation model](blocks-v2/phase-07-presentation.md)

- [ ] **B2-070** Сначала добавить tests единого action path для internal controls, external overlay и keyboard/context menu.
- [ ] **B2-071** Ввести exhaustive `BlockAction` и `available_actions()` в `otty-ui/terminal/src/block_actions.rs`.
- [ ] **B2-072** Подключить существующую action-button geometry к реальному hover/click; удалить `action_hover = None`.
- [ ] **B2-073** Перевести copy prompt/command/output/whole на backend `ExportBlock` query.
- [ ] **B2-074** Реализовать collapse/expand, pin/hide и rerun через stable BlockId.
- [ ] **B2-075** Ввести presentation order/groups/slices отдельно от canonical transcript.
- [ ] **B2-076** Реализовать move/group/split как изменения presentation references; запретить physical split/move active content.
- [ ] **B2-077** Синхронизировать внутренний BlockUI и внешний overlay на одном `BlockSummary`/`BlockAction` API.
- [ ] **B2-078** Добавить keyboard accessibility и focus для block actions без перехвата terminal input в обычном режиме.

Выход этапа: блоками и их секциями можно управлять независимо от их видимости и без нарушения PTY history.

### [Этап 8. Export и сохранение](blocks-v2/phase-08-export.md)

- [ ] **B2-080** Сначала добавить golden tests plain/ANSI/Markdown/JSON export, soft wrap, wide chars, hyperlinks и truncation markers.
- [ ] **B2-081** Реализовать единый `ExportBlock` pipeline в `otty-surface/src/block/export.rs`.
- [ ] **B2-082** Подключить clipboard actions к export result вместо текущего viewport/cached-text path.
- [ ] **B2-083** Реализовать atomic save в application layer с явным result/error event и без I/O на render thread.
- [ ] **B2-084** Определить versioned `otty.block` JSON schema и round-trip tests.
- [ ] **B2-085** До автоматической session persistence определить secret/redaction policy и запросить согласование любых новых storage/compression dependencies.

Выход этапа: copy и save дают один и тот же детерминированный результат для on-screen и off-screen blocks.

### [Этап 9. Дополнительные shells и сложные окружения](blocks-v2/phase-09-shells.md)

- [ ] **B2-090** Добавить Fish protocol-v2 hooks и real-shell tests.
- [ ] **B2-091** Добавить PowerShell protocol-v2 hooks и tests на доступных CI platforms.
- [ ] **B2-092** Добавить tmux/screen detection, self-test и документацию passthrough без автоматической правки user config.
- [ ] **B2-093** Спроектировать explicit remote bootstrap для SSH/container с отдельным threat/cleanup review.
- [ ] **B2-094** Добавлять Nushell/прочие shells только вместе с real-shell lifecycle tests; иначе оставлять `Unsupported`.

### [Этап 10. Финальная активация v2 и удаление legacy](blocks-v2/phase-10-rollout.md)

- [x] **B2-100** Зафиксировать single-path release policy: runtime switch, compatibility adapter и
  migration window не добавляются; rollback выполняется предыдущим application artifact.
- [x] **B2-101** Запустить полный набор `cargo +nightly fmt`, clippy, deny, workspace tests и llvm-cov согласно `AGENTS.md`; подтвердить, что общий line coverage не снизился относительно baseline до изменений.
- [ ] **B2-102** После завершения целевой архитектуры повторить B2-004/B2-005 и сравнить с ранее
  сохранённым baseline report, не добавляя второй engine path в один бинарник.
- [ ] **B2-103** Подтвердить v2-only для всех новых supported sessions после Bash/Zsh,
  viewport, export и burst-output acceptance matrix; bootstrap failure даёт `Degraded`, а не v1 fallback.
- [ ] **B2-104** До релиза завершить и подтвердить removal из B2-014/B2-021: удалить
  оставшиеся v1 script emission, parser/schema/actions/handlers/tests кроме legacy-ignore fixture,
  old stitched-history snapshot, временный
  `integration_status_badge` и относящийся к нему `is_shell` plumbing из pane-grid; финальные
  integration diagnostics оставить в итоговом status/debug UI.
- [ ] **B2-105** Обновить `otty-surface/README.md`, `otty-escape/README.md` и `otty-ui/terminal/README.md` финальными контрактами.

## 15. Dependency order и рекомендуемые PR

```text
PR 1:  B2-001..005  baseline/regressions
PR 2:  B2-010..016  typed IDs + protocol-v2 lifecycle reducer
PR 3:  B2-020..027  protocol v2 parser + OSC 133
PR 4:  B2-030..039  Bash/Zsh v2 + integration status
PR 5:  B2-040..047  sectioned content + freeze + routing
PR 6:  B2-050..058  height index + anchored viewport + selection
PR 7:  B2-060..066  latest-frame transport
PR 8:  B2-070..078  actions + presentation manipulation
PR 9:  B2-080..085  export/save
PR 10: B2-090..094  additional environments
PR 11: B2-100..105  v2 finalization and legacy removal
```

Прямые зависимости:

- sectioned content зависит от lifecycle и prompt markers;
- anchored viewport зависит от typed ID, logical lines и height index;
- UI actions зависят от stable query API и sectioned content;
- nested shell routing зависит от protocol v2 и `ShellContext`;
- persistence зависит от frozen content и стабильной export schema;
- удаление v1 выполняется сразу после production-подобной проверки v2 scripts и до
  релиза; migration window и dual-path нет.

Не следует объединять весь roadmap в один большой rewrite PR. На границах каждого PR terminal
должен оставаться запускаемым, ordinary shell без integration — рабочим, а v2 invariants —
покрыты тестами. Это не требует сохранения v1 protocol path.

## 16. Definition of Done

Blocks v2 считается готовым, когда одновременно выполнено следующее:

- Bash и Zsh root/nested lifecycle tests проходят без duplicate/missing blocks;
- command exit code и cwd не теряются, metadata update не заменяет ранее известные поля;
- prompt, command и output копируются независимо и корректно для multiline cases;
- off-screen block можно выбрать, прокрутить к нему, скопировать и сохранить;
- manual scroll не прыгает при long-running output, truncation и resize;
- burst output не создаёт unbounded frame backlog;
- finished history не хранит по полному mutable `Surface` на каждый блок;
- UI показывает integration status и причину degraded mode;
- presentation reorder/split/group не изменяет canonical transcript;
- malformed/spoofed/stale protocol events не corrupt-ят соседние блоки;
- memory и latency соответствуют критериям раздела 13.5;
- старый v1 path удалён до релиза, а legacy v1 DCS безопасно игнорируется;
- все обязательные проверки из `AGENTS.md` проходят, coverage не снижен.

## 17. Что делать первым

Самый безопасный первый вертикальный срез:

1. Написать regression tests на lifecycle metadata merge и scroll anchor.
2. Ввести typed IDs + protocol-v2 lifecycle reducer; v1 adapter не добавлять.
3. Добавить `command_end` и unique session/shell IDs в Bash/Zsh v2.
4. Разделить header/output для одного активного блока.
5. Перевести scroll на `FollowTail/Anchored` и только затем менять UI controls.

Это даст заметный прирост стабильности раньше, чем будут готовы reorder, rich content и persistence. Начинать с кнопок, сохранения или косметики BlockUI до этих пяти пунктов нецелесообразно: они продолжат опираться на нестабильные ID, неверные section boundaries и viewport-only данные.
