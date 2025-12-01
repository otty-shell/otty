# 06. Визуальная отрисовка рамок блоков в TerminalView

## Обзор

Шестая задача: визуально выделить блоки в `TerminalView`, реализовать hover/selected состояние и закрепление активного блока по клику. На этом шаге не меняется логика ввода/выбора текста, только добавляется визуальный слой поверх существующего рендера.

См. спецификацию: `specs/spec.md`, раздел 8.1 и 8.2.

## Что уже есть

- Задачи 01–05:
  - `SnapshotView` предоставляет `blocks()` и `block_at_point`.
  - `Terminal`/`Engine` уже обновляют snapshot при `ContentSync`.
- UI:
  - `TerminalViewState` (`otty-ui/terminal/src/view.rs`) хранит состояние фокуса, drag, позицию мыши, hovered hyperlink и т.д.; `TerminalViewState::new()` инициализирует поле `hovered_span_id`.
  - `TerminalView::draw` (`otty-ui/terminal/src/view.rs`) уже вычисляет геометрию клеток (`cell_width`, `cell_height`, `layout_offset`, `display_offset`) и рисует фон, текст, курсор, используя `Frame`.
  - `InputManager` (`otty-ui/terminal/src/input.rs`) вычисляет `mouse_position_on_grid` в `handle_cursor_moved`, публикует `Event::Redraw` при смене `hovered_span_id` и обрабатывает клики/scroll в `handle_left_button_pressed`/`handle_button_released`.

## Что нужно сделать

1. Расширить состояние `TerminalViewState`:
   - Добавить поля:

```rust
pub hovered_block_id: Option<String>,
pub selected_block_id: Option<String>,
```

   - Инициализировать их в `TerminalViewState::new()` значением `None`.

2. Обновление `hovered_block_id`:
  - В `InputManager::handle_cursor_moved` (где уже обновляется `mouse_position_on_grid`):
    - получить snapshot (`terminal_state.view()`);
    - вызвать `block_at_point` с `state.mouse_position_on_grid`;
    - если найденный блок имеет другой `id` (или блок отсутствует), обновить `view_state.hovered_block_id` (`Some(block.id.clone())`/`None`) и опубликовать событие `Event::Redraw`.
    - переиспользовать существующую логику для hyperlink’ов: смена hover блока и ссылок может происходить независимо, поэтому после вычисления `hovered_span_id` нужно сравнивать оба состояния и запрашивать перерисовку, если хоть одно изменилось.

3. Обновление `selected_block_id`:
   - В обработке клика ЛКМ (`handle_left_button_pressed`/`handle_button_released` в `InputManager`):
     - при отпускании кнопки мыши:
       - если `hovered_block_id.is_some()` и значение отличается от `selected_block_id`, присвоить его (клон строки) в `selected_block_id` и опубликовать `Event::Redraw { id: self.terminal_id }`;
       - если курсор не над блоком, сбросить `selected_block_id` в `None`, тоже с `Redraw`, чтобы убрать рамку.
     - не менять существующую логику selection и MouseReport.

4. Отрисовка рамок в `TerminalView::draw`:
   - Получить `view = content.view()`.
   - На основе `view.blocks()` и `view.size`:
     - для каждого блока вычислить:
       - `y` по `start_line`, `line_count`, `display_offset`, `cell_height`;
       - `x` = `layout.position().x`, ширина = `layout.bounds().width`.
      - пропускать блоки с `line_count == 0`, чтобы не рисовать пустые рамки.
    - Определить целевые цвета через тему (`self.term.theme.get_color(ansi::Color::Std(..))`), например:
      - hover — `StdColor::Blue` с уменьшенной `alpha`;
      - selected — `StdColor::BrightBlue`.
    - Перед циклом по клеткам нарисовать рамки с помощью `Path::rectangle` + `Stroke::default().with_width(1.0)`:
      - сначала все `selected`, затем `hovered` (так Hover может быть поверх, если блок ещё не выбран);
      - следить, чтобы прямоугольники находились в пределах `layout.bounds()` (при необходимости ограничить высоту/ширину по viewport).
    - Рамки рисовать до основного текста, чтобы фон и символы остались поверх линий (стандартный Canvas позволяет сначала `frame.stroke`, затем `frame.fill_text`).

5. Сохранить существующее поведение:
   - Не модифицировать логику фоновых прямоугольников для текста и selection.
   - Не менять обработку hyperlink hover (она уже использует `hovered_span_id`; обновление блоков добавляет второе состояние, но логика ссылок остаётся прежней).

## Ожидаемый результат

- При наведении мыши рамка (hover) появляется вокруг блока, под которым находится курсор.
- При клике рамка закрепляется (selected); дальнейшее наведение на другие блоки не меняет выбранный блок, пока не произойдёт новый клик.
- Поведение выделения текста, прокрутки, кликов по ссылкам остаётся прежним.

## Как протестировать

1. Ручной тест:
   - Запустить терминал с включёнными shell‑хуками блоков.
   - Выполнить несколько команд:
     - при перемещении мыши вверх/вниз рамка должна подсвечивать соответствующие блоки;
     - при клике ЛКМ рамка должна закрепляться на выбранном блоке.

2. Визуально убедиться:
   - что рамки не «дрожат» при прокрутке;
   - что рамки корректно учитывают `display_offset`.

3. (Опционально) Юнит‑тесты:
   - можно выделить небольшую функцию вычисления геометрии блока по `BlockSnapshot` и `SnapshotSize` и протестировать её отдельно.

## Зависимости

- Требует выполненных задач 01–05.
- RFC: `specs/spec.md`, разделы 8.1–8.2.
- План: `specs/plan.md`, раздел 6.
