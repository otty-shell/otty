# 03. Связка Action::BlockEvent с BlockSurface

## Обзор

Третья задача: научить `BlockSurface` создавать и завершать блоки на основе
`Action::BlockEvent`, приходящих из `otty-escape` через `TerminalSurfaceActor`.
На этом шаге `BlockSurface` уже хранит несколько блоков, но snapshot
по‑прежнему может экспортировать только активный блок (визуально изменений
немного).

См. спецификацию: `specs/spec.md`, разделы 5.3 и 6.3.

## Что уже есть

- Задача 01:
  - В `otty-escape` реализован DCS‑протокол блоков (`block.rs`):
    - `BlockPhase` (`Preexec` / `Exit` / `Precmd`);
    - `BlockMeta` c полями `id`, `kind`, `cmd`, `cwd`, `started_at`,
      `finished_at`, `exit_code`, `shell`, `is_alt_screen`;
    - парсер `parse_block_dcs(..)` и новое действие
      `Action::BlockEvent(BlockEvent)`.
  - `Action::BlockEvent(BlockEvent)` корректно генерируется из DCS и
    доходит до `TerminalSurfaceActor`.
- Задача 02:
  - В `otty-surface` реализован `BlockSurface`, который:
    - хранит несколько блоков `Block { meta: BlockMeta, surface: Surface }`;
    - реализует `SurfaceActor + SurfaceModel`, делегируя операции
      в активный `Surface`;
    - пока экспортирует snapshot **только активного** блока.
  - Внутренний `BlockMeta` в `otty-surface` содержит минимальный набор
    полей: `id`, `kind`, `started_at`, `finished_at`, `exit_code`,
    `is_alt_screen` (без `cmd`/`cwd`/`shell` — они есть только в
    `otty-escape::BlockMeta`).
  - В `terminal::builder` (`otty-libterm`) `DefaultSurface = BlockSurface`,
    т.е. продакшен‑конфигурация уже использует блоковую поверхность.
- `TerminalSurfaceActor` (`otty-libterm/src/terminal/surface_actor.rs`):
  - добавлена ветка `Action::BlockEvent(event)`;
  - текущая реализация `handle_block_event` только логирует событие и
    не влияет на `Surface`/`BlockSurface`.

## Что нужно сделать

### 1. Расширить BlockSurface

В `otty-surface/src/block.rs`:

1.1. Расширить состояние блока:

- Добавить флаг завершённости блока, например:
  - либо поле `is_finished: bool` в `BlockMeta`,
  - либо поле `is_finished: bool` в `Block`.
- На этом шаге **достаточно** продолжать использовать минимальный
  `BlockMeta` (как в задаче 02); поля `cmd`/`cwd`/`shell` из RFC можно
  добавить отдельной задачей.

1.2. Методы управления блоками:

- Добавить метод начала блока:

  ```rust
  impl BlockSurface {
      /// Завершает текущий блок (если он ещё running), создаёт новый блок
      /// с новым `Surface` и делает его активным.
      pub fn begin_block(&mut self, meta: BlockMeta) -> &mut Surface { .. }
  }
  ```

  Поведение:

  - если активный блок не помечен как завершённый, пометить его
    `is_finished = true`;
  - создать новый `Block { meta, surface: Surface::new(..) }`:
    - использовать сохранённый `SurfaceConfig`,
    - размер брать из текущего активного `Surface`
      (через `Dimensions`‑интерфейс);
  - добавить новый блок в конец `blocks`;
  - обновить `active` на индекс нового блока;
  - применить политику `max_blocks` (см. ниже).

- Добавить метод завершения блока по `id`:

  ```rust
  impl BlockSurface {
      /// Обновляет метаданные и помечает блок с данным `id`
      /// как завершённый.
      pub fn end_block_by_id(&mut self, meta: &BlockMeta) { .. }
  }
  ```

  Поведение:

  - найти первый блок с `block.meta.id == meta.id`;
  - обновить его метаданные:
    - `finished_at` (если присутствует в `meta`);
    - `exit_code` (если присутствует в `meta`);
    - при необходимости скорректировать `kind`
      (например, для `Precmd` → `Prompt`);
    - `is_alt_screen`;
  - пометить блок `is_finished = true`.

1.3. Политика `max_blocks`:

- При `blocks.len() > max_blocks`:

  - удалять только самые старые блоки с `is_finished = true`,
    начиная с начала списка;
  - не удалять незавершённые блоки (`running`, активный промпт и т.п.);
  - если все блоки незавершённые и лимит превышен, можно:
    - либо временно не удалять ничего (безопасный вариант на старте),
    - либо оставить это поведение явно задокументированным
      как «открытый вопрос».

- Закрепить это поведение в комментарии к `BlockSurface`.

### 2. Обработка Action::BlockEvent в TerminalSurfaceActor

В `otty-libterm/src/terminal/surface_actor.rs`:

2.1. Преобразование метаданных:

- В `handle_block_event` требуется сопоставить
  `crate::escape::BlockEvent` (из `otty-escape`) с внутренними
  структурами `otty-surface`:

  - преобразовать `escape::BlockMeta` → `surface::BlockMeta`:
    - `id` копировать как есть;
    - `kind` — по фазе (`Command` / `Prompt` / `FullScreen`);
    - `started_at` / `finished_at` / `exit_code` / `is_alt_screen`
      копировать при наличии;
    - поля `cmd`/`cwd`/`shell` на этом шаге игнорировать.

- Важно: `TerminalSurfaceActor` параметризован по `S: SurfaceActor`.
  Для продакшен‑конфигурации, где `S = BlockSurface`, он должен
  вызывать специфичные методы `begin_block` / `end_block_by_id`.
  Для других реализаций `SurfaceActor` блоковые события можно
  игнорировать (то есть только логировать).

2.2. Обработка фаз:

- Для `BlockPhase::Preexec`:

  - на основе `event.meta` сформировать `BlockMeta` для `otty-surface`;
  - завершить текущий блок (если ещё не `is_finished`);
  - вызвать `BlockSurface::begin_block(meta)` и сделать новый блок активным.

- Для `BlockPhase::Exit`:

  - на основе `event.meta` вызвать
    `BlockSurface::end_block_by_id(&meta)`:
    - обновить `exit_code` и `finished_at`;
    - пометить соответствующий блок как `is_finished`.

- Для `BlockPhase::Precmd`:

  - использовать как сигнал окончания предыдущего командного блока:
    - если активный `Command`‑блок ещё не завершён, пометить его
      завершённым (`is_finished = true`, `finished_at` из `meta`);
  - опционально (минимальный вариант можно явно описать в коде):
    - либо сразу создавать новый блок `kind = Prompt`
      через `begin_block`,
    - либо оставить создание промпт‑блоков для отдельной задачи
      (важно зафиксировать выбранный подход в комментарии).

2.3. Поведение по умолчанию:

- Если по какой‑то причине `TerminalSurfaceActor` работает не с
  `BlockSurface`, а с обычной реализацией `SurfaceActor`:

  - допустимо оставить поведение «только логируем `BlockEvent`»;
  - это следует явно описать в комментарии к `handle_block_event`
    как «блоковая функциональность отключена для нестандартных поверхностей».

### 3. Политика ALT_SCREEN

- На этом шаге достаточно:

  - принимать `is_alt_screen` из `escape::BlockMeta`,
  - копировать его в `BlockSurface::BlockMeta`,
  - ни на что его пока **не** использовать.

- Привязка `is_alt_screen` к `SurfaceMode`/primary vs ALT‑гридам — это
  отдельная будущая задача (см. RFC).

## Ожидаемый результат

- При приходе DCS‑событий блоков `BlockSurface` создаёт новые блоки и
  завершает старые согласно метаданным `BlockMeta`.
- В рамках одной сессии история блоков хранится в `BlockSurface::blocks`:
  - новые команды/промпты создают новые блоки;
  - завершённые блоки помечаются `is_finished = true`;
  - при превышении `max_blocks` удаляются только старые завершённые блоки.
- Активным для рендера остаётся блок, соответствующий текущей
  команде/промпту.
- Визуально терминал продолжает отображать только активный блок
  (snapshot всё ещё плоский, без списка блоков).

## Как протестировать

1. Юнит‑тесты для `BlockSurface`:

   - Создание нескольких блоков через `begin_block`:
     - при последовательных вызовах `begin_block` должно увеличиваться
       число блоков, а `active` должен указывать на последний.
   - Завершение блоков через `end_block_by_id`:
     - по `id` обновляются `finished_at` / `exit_code` / `is_finished`.
   - Проверка политики `max_blocks`:
     - при превышении лимита удаляются только старые завершённые блоки;
     - незавершённые блоки (включая активный) не удаляются.

2. Интеграционный тест с `StubParser` в `otty-libterm`:

   - Написать тест, где `StubParser` генерирует последовательность:

     ```text
     BlockEvent(Preexec id=1), Print("a"), BlockEvent(Exit id=1),
     BlockEvent(Preexec id=2), Print("b"), BlockEvent(Exit id=2)
     ```

   - После выполнения проверить (через API, доступное из теста), что
     в `BlockSurface::blocks` два блока с корректными метаданными
     (`id`, `kind`, `started_at`/`finished_at`, `exit_code`) и, как
     минимум, что активным является второй блок.

3. Ручной тест:

   - Запустить терминал с прототипом shell‑хука, который отправляет
     DCS для нескольких команд.
   - Временно логировать структуру `BlockSurface::blocks` (через
     debug‑лог) и убедиться, что блоки создаются и завершаются ожидаемо:
     - при выполнении команды появляется новый блок;
     - по завершении команды он помечается завершённым;
     - при навигации по истории видно, что блоки не исчезают, пока
       не сработает политика `max_blocks`.

## Зависимости

- Требует полностью выполненных задач 01 и 02.
- RFC: `specs/spec.md`, раздел 6.3.
- План: `specs/plan.md`, раздел 3.
