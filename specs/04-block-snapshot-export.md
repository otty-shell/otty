# 04. Snapshot блоков и экспорт в SnapshotOwned

## Обзор

Четвёртая задача: расширить snapshot‑механику так, чтобы она экспортировала информацию обо всех блоках и их позициях в общем viewport’е. `BlockSurface::snapshot_owned` должен «сшивать» внутренние `Surface` блоков в единый плоский поток клеток и формировать `BlockSnapshot`.

См. спецификацию: `specs/spec.md`, раздел 7.1.

## Что уже есть

- Задачи 01–03:
  - `BlockSurface` хранит несколько блоков и умеет переключать активный по `BlockEvent`.
  - `snapshot_owned` пока возвращает только активный блок.
  - Внутренний `BlockMeta` содержит id/kind/timestamps/exit_code/is_alt_screen,
    но поля `cmd`/`cwd`/`shell` из DCS пока отбрасываются.
- `otty-surface::snapshot`:
  - умеет строить `SnapshotOwned` для одного `Surface`:
    - `cells`, `cursor`, `selection`, `display_offset`, `colors`, `mode`, `size`, `damage`, `hyperlinks`.

## Что нужно сделать

0. Перестать терять метаданные из DCS:

   - Расширить `otty-surface::block::BlockMeta` полями `cmd`, `cwd`, `shell`.
   - В `BlockSurface::begin_block` и `end_block_by_id` обновлять эти поля
     только когда приходят `Some(..)` (чтобы `None` не затирали значения,
     полученные на `Preexec`).
   - В `TerminalSurfaceActor::handle_block_event` прокидывать `cmd`/`cwd`/`shell`
     вместе с остальными полями в `BlockSurface`.

1. Ввести публичные структуры snapshot блоков:

```rust
struct BlockMetaPublic {
    id: String,
    kind: BlockKind,
    cmd: Option<String>,
    exit_code: Option<i32>,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    shell: Option<String>,
}

struct BlockSnapshot {
    id: String,
    meta: BlockMetaPublic,
    start_line: i32,
    line_count: usize,
    is_alt_screen: bool,
}
```

2. Расширить `SnapshotOwned`:
   - Добавить поле `pub blocks: Vec<BlockSnapshot>`.
   - Обновить конструктор/`Default` при необходимости.

3. Реализовать `BlockSurface::snapshot_owned`:
   - Итерироваться по `self.blocks` в порядке истории.
   - Для каждого блока:
     - получить временный `SnapshotOwned` вложенного `Surface`;
     - перенумеровать линии:
       - `start_line` = текущий накопленный offset;
       - `line_count` = количество строк в snapshot данного блока;
       - увеличить offset на `line_count`;
     - добавить клетки из вложенного snapshot в общий `cells`, смещая `point.line`
       на `start_line`, чтобы координаты перешли в общую систему.
     - сформировать `BlockSnapshot` c нужными полями.
   - Из активного блока (или из последнего) взять актуальное состояние курсора / selection / modes (на усмотрение, главное — стабильное поведение).
   - `HyperlinkMap::build` сейчас требует ссылку на `Surface`; после «сшивания»
     нужно либо научить её работать от плоского массива `SnapshotCell`, либо
     предоставить обёртку, которая даёт доступ ко всем блокам.

4. Расширить `SnapshotView`:
   - Реализовать:

```rust
impl<'a> SnapshotView<'a> {
    pub fn blocks(&self) -> &[BlockSnapshot];
    pub fn block_at_point(&self, p: Point) -> Option<&BlockSnapshot>;
}
```

   - `block_at_point` должен искать блок, диапазон линий которого содержит `p.line`.

5. Учесть scrollback и `display_offset`:
   - `BlockSurface` должен хранить суммарный `display_offset` (в строках),
     обновлять его в `scroll_display` и использовать при сборке snapshot.
   - `start_line` складывается в абсолютных координатах всей истории,
     а UI применяет `display_offset` поверх этих значений.

## Ожидаемый результат

- `SnapshotOwned` содержит:
  - плоский список клеток по всем блокам;
  - список `BlockSnapshot` с корректными `start_line`/`line_count`.
- `SnapshotView::blocks()` и `block_at_point` позволяют UI определять блок по координате.
- Существующий код, который смотрит только на `cells`, продолжает работать без изменений.

## Как протестировать

1. Юнит‑тесты в `otty-surface`:
   - Создать `BlockSurface` с 2–3 блоками разной высоты, заполненными символами.
   - Вызвать `snapshot_owned` и проверить:
     - что количество клеток равно сумме по блокам;
     - что `start_line`/`line_count` корректны;
     - что `block_at_point` возвращает ожидаемый блок для разных `Point`.

2. Интеграционный тест в `otty-libterm`:
   - Использовать `StubParser`, который создаёт несколько блоков и печатает разные символы;
   - через `TerminalEngine` получить `SnapshotOwned` и убедиться, что `blocks` и `cells` согласованы.

3. Ручной тест:
   - Включить простую отладочную печать количества блоков и их диапазонов при каждом кадре;
   - выполнить несколько команд с блок‑хуками — убедиться, что блоки растут и их координаты выглядят ожидаемо.

## Зависимости

- Требует выполненных задач 01–03.
- RFC: `specs/spec.md`, раздел 7.1.
- План: `specs/plan.md`, раздел 4.
