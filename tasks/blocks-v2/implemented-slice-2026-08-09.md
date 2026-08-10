# Выполненная итерация: первый вертикальный срез

Статус: **код реализован, автоматические проверки пройдены; ручная GUI-приёмка ещё не
зафиксирована**.

Дата среза: 2026-08-09. Родительский документ: [Blocks v2](../blocks-v2.md).

## Что представляет собой этот срез

Это реализация пяти шагов из раздела 17 общего roadmap и нескольких непосредственно связанных
частей parser, bootstrap, transport и UI. Срез пересекает фазы 0–7, но не заменяет и не
закрывает ни одну полную фазу. Подробные оставшиеся работы находятся в отдельных файлах фаз.

## Реализованный результат

- Typed `BlockId`, `TerminalSessionId`, `ShellInstanceId` и `ProtocolSequence`.
- Lifecycle reducer со sparse metadata merge, sequence validation и recovery diagnostics.
- Защита от duplicate/stale event и завершения соседнего block по повторному ID.
- Bounded `event-v2;h` parser, typed envelope, supported lifecycle events и OSC 133 A/B.
- System-random terminal session ID и rejection foreign session events.
- Bash/Zsh protocol v2 scripts с PID-scoped guard, nested IDs, command-end, exit/pipeline
  status, dependency-free encoding и сохранением существующих hooks.
- Bash OSC markers оформлены как невидимые `PS1` escapes и не раскрывают Fedora/Bash 5.3
  `${PROMPT_START@P}` как видимый текст.
- Atomic preparation integration assets и базовые `Pending/Active/Degraded/Unsupported` UI
  statuses.
- Базовые semantic prompt/command/output queries и `CommandRecord`.
- `ScrollPosition::FollowTail/Anchored`, block anchor и model-level off-screen
  `ScrollToBlock`.
- Latest-frame mailbox ёмкостью один, bounded lossless queues и replacement counter.
- Подключённый hover/click action control и базовое копирование semantic block content.

## Что намеренно не входит

- Удаление legacy v1 parser/action/lifecycle path и окончательная module decomposition;
  compatibility adapter для v1 не планируется.
- Настоящий PTY shell test harness и baseline benchmarks.
- Immutable freeze, `HeaderGrid`/`OutputGrid`, `OutputRouter`, budgets и alt-screen ownership.
- Height index, stable logical selection, lazy reflow и viewport-only snapshots.
- Render revisions/ticks/partial damage.
- Полная action/presentation model, export/save, persistence и дополнительные shells.
- Финальная v2 acceptance matrix и удаление v1 до релиза.

## Уже выполненные автоматические проверки

```bash
bash -n assets/shell-integrations/otty.bash
zsh -n assets/shell-integrations/otty.zsh
cargo +nightly fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
```

Все эти команды прошли. `cargo deny` оставляет существующие warnings для transitive
`core2 0.4.0` и `spin 0.9.8`, но завершается успешно.

Coverage-команда также выполнена:

```bash
cargo llvm-cov --workspace --all-features
```

Зафиксированный результат: **51.16% line coverage**. Фиксированный минимальный порог не
применяется: согласно `AGENTS.md`, общий line coverage после изменений не должен снижаться
относительно baseline до изменений. Для следующих итераций результат этого среза служит
зафиксированной точкой сравнения. Definition of Done всего Blocks v2 остаётся незакрытым из-за
перечисленных выше нереализованных частей и отсутствующей ручной приёмки, а не из-за порога 80%.

## Как вручную проверить реализованный результат

### 1. Handshake и обычный lifecycle

1. Из корня workspace запустить `cargo run -p otty`.
2. Открыть Bash terminal и дождаться badge `Integration v2 active`.
3. На Fedora/Bash 5.3 убедиться, что перед обычным prompt не отображается
   `PROMPT_START@P}`/`PROMPT_END@P}`.
4. Выполнить `printf 'ok\n'`, `false`, `false | true`, `pwd`, `cd /tmp`, `pwd`.
5. Проверить, что на каждую выполненную команду создан ровно один block, terminal остаётся
   рабочим после non-zero outcome, а новые prompts не меняют завершённые соседние blocks.

### 2. Idempotency и nested shell

1. Вернуться в корень repository и дважды выполнить
   `source assets/shell-integrations/otty.bash`.
2. Выполнить `printf 'source-twice\n'`; должен появиться один command block.
3. Запустить `bash --noprofile --norc -i`, выполнить `false`, выйти и выполнить `true` в parent.
4. Проверить, что child и parent commands не объединены и child exit не завершил parent block.
5. Повторить сценарий в Zsh с `assets/shell-integrations/otty.zsh`.

### 3. Parser recovery

1. В активном terminal выполнить `printf '\033Pevent-v2;h;zz\033\\'`.
2. Сразу выполнить `printf 'parser-recovered\n'`.
3. Malformed DCS не должна создать block event, повредить соседний block или остановить parser;
   вторая команда должна отобразиться нормально.

### 4. Anchored scroll

1. Создать достаточно history, затем выполнить медленно растущий output:

   ```bash
   bash -c 'i=0; while [ "$i" -lt 5000 ]; do i=$((i+1)); printf "line %05d\n" "$i"; if [ $((i % 50)) -eq 0 ]; then sleep 0.02; fi; done'
   ```

2. Во время вывода уйти в середину history и запомнить верхнюю видимую строку.
3. Пока active block продолжает расти, эта строка не должна сдвигаться.
4. Вернуться к нижнему краю: viewport должен снова следовать за новым output.
5. Вызвать переход к старому полностью off-screen block; navigation должна выполняться по
   `BlockId`, а не зависеть от его присутствия в текущем frame.

### 5. Latest-frame transport и несколько sessions

1. Запустить `yes frame | head -n 100000` и во время вывода переключать tabs и менять размер
   окна.
2. UI не должен зависнуть, а после окончания должен показать последнюю часть output.
3. Открыть несколько terminal tabs одновременно. Каждая должна получить `Active v2`, а blocks
   разных sessions не должны смешиваться.
4. Закрыть одну tab во время output; остальные sessions продолжают работать без panic/deadlock.

### 6. Базовые UI actions

1. Навести курсор на action button finished block и проверить соответствие hover нарисованной
   geometry.
2. Нажать Copy Output и Copy Whole, вставить результаты в текстовый редактор.
3. Output не должен включать prompt; Whole должен содержать доступный semantic block content.
4. Повторить для старого block после прокрутки. Эта проверка относится только к базовому
   snapshot path; полноценный off-screen export остаётся задачей фазы 8.

## Лист ручной приёмки

- [ ] Handshake и обычный Bash lifecycle проверены в GUI.
- [ ] Fedora/Bash 5.3 prompt не показывает `PROMPT_START@P}`/`PROMPT_END@P}`.
- [ ] Source-twice и nested Bash проверены в GUI.
- [ ] Zsh lifecycle проверен в GUI.
- [ ] Malformed protocol recovery проверен вручную.
- [ ] Anchored scroll при растущем output проверен вручную.
- [ ] Burst output и несколько terminal sessions проверены вручную.
- [ ] Базовые hover/copy actions проверены вручную.

После ручного прогона следует поставить отметки, записать OS/shell versions и обнаруженные
дефекты прямо в этот файл. Даже полный успех этого листа подтверждает только текущий срез, а не
готовность всего Blocks v2.
