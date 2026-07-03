# Terminal text shaping

## Цель

Спроектировать и реализовать долгосрочную архитектуру отрисовки complex text в
терминале без отказа от cell-based модели. Grid остается источником истины для
терминального состояния, а renderer строит временные text runs только на время
кадра и только для передачи в text shaper.

Задача нужна из-за текущего дефекта: `otty-surface` хранит zero-width combining
marks в `Cell::zerowidth()`, но `otty-ui-terminal` при отрисовке передает в
`Text.content` только `cell.c`. В результате Thai tone marks, Arabic harakat,
Hebrew niqqud, decomposed Latin accents и другие combining marks могут быть
потеряны или расположены неправильно.

Эта задача не должна решаться набором коротких language-specific или per-cell
фиксов. Нужный результат - renderer, который корректно shape-ит текстовые runs
поверх terminal grid и сохраняет cell-based поведение TUI, selection, cursor,
links и resize.

## Текущая модель

- `otty-surface` хранит терминальную сетку: base character, zero-width marks,
  cell flags, colors, cursor, selection, hyperlinks, scrollback и wrapping.
- `otty-ui-terminal` рисует viewport из snapshot-а surface.
- Фон, cursor, selection и hyperlink hit testing уже естественно выражены через
  grid cells.
- Текст сейчас рисуется per cell: одна canvas `Text` команда получает один
  `cell.c`.

Эта модель хорошо подходит для TUI geometry, но плохо подходит для complex text:
shaper не видит полный grapheme cluster, соседние glyphs и contextual forms.

## Предлагаемый подход

Оставить surface cell-based, но заменить text rendering path на run-based
отрисовку поверх сетки.

Renderer должен делать два независимых прохода:

1. Cell-based geometry pass:
   - background rectangles;
   - cursor rectangle;
   - selection rectangles;
   - hyperlink hover underline;
   - block UI overlays.

2. Text shaping pass:
   - пройти по видимым строкам snapshot-а;
   - собрать текст каждой renderable cell как `cell.c + cell.zerowidth()`;
   - объединить соседние cells в `RenderRun`, пока их text style совместим;
   - передать весь run в shaper;
   - нарисовать shaped text в grid-aligned rectangle, начинающемся с первой
     колонки run-а.

Surface при этом не должен знать про Arabic shaping, bidi, Thai positioning,
font fallback или glyph placement. Его ответственность - терминальная модель, не
rendering.

## Non-goals

Эта задача не должна ограничиваться следующими частичными решениями:

- append `Cell::zerowidth()` к per-cell `canvas::Text`;
- language-specific storage hacks в `otty-surface` как основной путь решения;
- ручное позиционирование Thai/Arabic/Hebrew marks в renderer-е;
- включение Unicode normalization как замена shaping;
- изменение terminal grid в сторону plain text buffer.

Такие изменения могут быть полезны для диагностики или временного сравнения, но
они не закрывают требования задачи и не должны быть финальным design direction.

## Run-based renderer design

### RenderRun

Нужен внутренний тип только для renderer-а:

```rust
struct RenderRun {
    text: String,
    line: i32,
    start_column: usize,
    cell_columns: usize,
    style: RenderTextStyle,
}
```

`cell_columns` - terminal width run-а в cells. Для wide character он должен
учитывать две колонки. `WIDE_CHAR_SPACER` cells не должны добавлять отдельный
символ в run.

### RenderTextStyle

Run можно продолжать только пока совместимы:

- resolved foreground color;
- resolved font family;
- bold/dim-bold weight;
- italic style;
- selected/inverse state after color resolution;
- cursor text color override, если cursor меняет foreground.

Background не обязан участвовать в text run style, если он уже рисуется отдельным
cell-based pass-ом.

Hyperlink hover и selection могут менять foreground или underline. Если меняют,
run нужно split-ить на границах этих состояний.

### Grid alignment

Терминальная геометрия не должна зависеть от natural glyph advances.

Первый вариант реализации:

- run origin: `layout_x + start_column * cell_width`;
- run width: `cell_columns * cell_width`;
- line origin: текущая terminal row baseline/center;
- layout width для shaper-а равен run width;
- фон, cursor и selection остаются привязанными к cells.

Более строгий вариант для будущего:

- хранить cluster-to-cell mapping;
- контролировать, что visual advance cluster-а не сдвигает соседние terminal
  columns;
- не split-ить run внутри `base + zerowidth` cluster-а.

## TUI behavior

TUI layout должен остаться стабильным, если выполняются правила:

- grid остается единственным источником cursor position и hit testing;
- total rendered width строки не превышает terminal cell width allocation;
- runs split-ятся на style boundaries, но не меняют cell geometry;
- wide char spacer cells не создают отдельного glyph-а;
- tabs продолжают обрабатываться текущей terminal моделью.

Риск для TUI: если shaper natural advances будут использоваться без
ограничения grid width, borders и columns могут визуально съехать. Поэтому
renderer должен считать grid width сильным constraint-ом.

## Text clipping

Shaped text должен clip-иться по terminal viewport/widget bounds.

Правила:

- glyphs, combining marks, italic overhang и fallback glyphs не должны рисоваться
  за пределами terminal viewport;
- visual glyph bounds не расширяют terminal drawing area;
- tight per-run clipping по exact cell range не включается по умолчанию, потому
  что он может обрезать combining marks и полезный glyph overhang;
- небольшой overflow внутри terminal viewport допустим, если он нужен для
  корректного shaping;
- per-run clipping можно добавить только после измеренной regression и только с
  проверкой, что он не ломает complex-script marks.

## Hyperlink hover

Hyperlink hit testing должен остаться cell-based:

- mouse position -> grid point;
- grid point -> hyperlink span id;
- hover underline рисуется по cells;
- run splitting нужен только если hover меняет text style.

Риск: если run содержит cells из разных hyperlink states и hover меняет
foreground, часть текста может получить неверный стиль. Для этого run builder
должен учитывать hovered span id как style boundary.

Hover underline также должен оставаться cell-based даже при complex glyph
overflow:

- underline рисуется по terminal cell range, принадлежащему hyperlink span;
- visual overflow shaped glyphs не расширяет clickable area и underline bounds;
- glyph overflow должен решаться clipping-ом run/viewport, а не изменением
  hyperlink geometry;
- это сохраняет совпадение hover, selection, cursor и TUI grid semantics.

## Selection

Selection также остается cell-based:

- selected background рисуется прямоугольниками cells;
- selected foreground вычисляется как сейчас через resolved colors;
- text runs split-ятся на selected/unselected boundaries.

Риск: split selection посередине complex-script слова ухудшит shaping на границе
selection. Это типичное ограничение cell selection в терминалах. Минимальное
правило - не split-ить внутри одной cell cluster.

## Resize

Resize не требует persistent text layout state:

- surface пересчитывает grid/reflow;
- snapshot обновляется;
- renderer заново собирает runs из нового snapshot-а.

Кеши runs должны инвалидироваться при:

- изменении snapshot/damage;
- изменении font или font size;
- изменении cell size;
- изменении theme;
- изменении selection;
- изменении hovered hyperlink span;
- изменении display offset.

## Run caching decision

Начальный renderer не должен кешировать shaped glyph layouts до появления
benchmark-данных, показывающих, что shaping является bottleneck.

Первый этап:

- собирать `RenderRun` из visible snapshot rows;
- при необходимости кешировать или пересобирать только render runs по
  damaged/affected rows;
- каждый frame shape-ить актуальные runs заново;
- держать типы разделенными: `RenderRun` для text/cell/style mapping и
  `ShapedRenderRun` для будущего glyph layout cache.

Причины:

- run construction дешевле и проще инвалидируется;
- shaped glyph cache требует более сложного ключа: text, style, font metrics,
  cell width, shaping mode, line height, font system version и layout bounds;
- selection, cursor и hyperlink hover могут менять split/style boundaries;
- resize, font changes и theme changes легко делают cached glyph positions
  stale.

Архитектура должна позволять добавить shaped-run cache позже без переписывания
run builder-а. Решение о shaped cache принимается только после измерений на
ASCII logs, colorful TUI, scrollback и complex-script samples.

## CPU и память

Текущий renderer потенциально создает text draw/shaping operation на каждую
непустую cell. Run-based renderer может уменьшить количество операций до
количества style runs в viewport.

Ожидаемое влияние:

- plain output: CPU может снизиться из-за меньшего числа text operations;
- colorful TUI: CPU зависит от количества style boundaries и может остаться
  близким к текущему;
- complex scripts: shaping run-а дороже, но корректнее;
- память: временные strings и metadata по visible rows обычно десятки килобайт
  для обычного viewport-а.

Нужны измерения до включения по умолчанию:

- viewport 80x24, 120x40, 200x60;
- plain ASCII log;
- colorful TUI с частыми style changes;
- Thai/Arabic/Hebrew sample text;
- scroll performance;
- resize performance.

## Bidi

Run-based shaping не обязан сразу включать Unicode bidi reordering.

Причины:

- многие terminal applications уже выводят visual order;
- автоматический bidi может сломать TUI alignment и mixed LTR/RTL prompts;
- shaping Arabic contextual forms полезен отдельно от bidi.

Начальный scope: shape text in grid order. Bidi рассматривать как отдельную
явно спроектированную опцию после измерений и тестов.

Default renderer должен сохранять terminal grid order и не применять
автоматический line-wide Unicode bidi reordering.

Приемлемое поведение для этой задачи:

- renderer shape-ит runs без нарушения cell ownership;
- emitted/grid order остается источником истины для visual placement;
- selection, cursor, hyperlink mapping и mouse hit testing остаются в grid order;
- full bidi visual reordering считается out of scope для начального run-based
  renderer-а;
- bidi reordering можно добавлять позже только как explicit setting после
  отдельного design-а и regression-тестов на TUI, cursor, selection и links.

## Iced and cosmic-text capabilities

Текущий `iced` canvas `Text` недостаточен как основной API для строгого
run-based terminal renderer-а.

Причины:

- `canvas::Text` позволяет задать `max_width`, но не позволяет явно задать
  wrapping mode;
- внутри `canvas::Text` используется default wrapping, а default в `iced` это
  word wrapping;
- terminal renderer не должен получать paragraph-like переносы строк внутри
  одного grid row;
- `canvas::Text` не дает доступа к cluster/glyph mapping и не прокидывает
  `cosmic_text::Buffer::set_monospace_width`;
- для terminal grid важно контролировать run width, clipping, split boundaries и
  glyph positions более явно.

`cosmic-text` как нижний уровень выглядит достаточным для прототипа:

- `Buffer::set_size` задает bounds run-а;
- `Buffer::set_wrap(Wrap::None)` отключает переносы;
- `Buffer::set_monospace_width(Some(cell_width))` может помочь удерживать
  monospace glyphs в terminal grid;
- `layout_runs()` возвращает glyph positions, byte ranges и bidi levels.

Вывод: целевой renderer должен использовать lower-level путь: либо
`iced_graphics::text::Paragraph` с явным `Wrapping::None`, либо прямую работу с
`iced_graphics::text::cosmic_text`. Даже при использовании `cosmic-text` grid
alignment остается ответственностью renderer-а: нужно задавать run bounds,
clip-region и style/split boundaries.

## Renderer mode setting

Renderer mode для сравнения per-cell и run-based implementation должен жить в
application settings.

Правила:

- setting нужен для экспериментального rollout-а, диагностики и сравнения
  поведения на реальных TUI/screens;
- env var не подходит как основной механизм, потому что пользовательские и QA
  сценарии должны быть воспроизводимы через обычную конфигурацию приложения;
- compile-time switch не подходит как основной механизм, потому что он усложняет
  сравнение поведения в одном build-е;
- значение должно быть явным enum, например `Cell` и `Runs`;
- если setting временный, это нужно отметить в задаче rollout-а и удалить после
  стабилизации run-based renderer-а.

## Thai and Lao SARA AM

`U+0E33` Thai SARA AM и `U+0EB3` Lao VOWEL SIGN AM являются отдельным спорным
случаем.

Surface-level decomposition не является target design для этой задачи:

- `U+0E33` -> `U+0E4D` + `U+0E32`;
- `U+0EB3` -> `U+0ECD` + `U+0EB2`.

Такой подход может работать с текущей per-cell отрисовкой и grid storage, но он
закрепляет language-specific знание в terminal model.

Главный минус: меняет текстовую модель, copy может вернуть decomposed pair
вместо исходного codepoint. Это compatibility decomposition, не canonical NFC
equivalence. NFC не раскладывает `U+0E33`.

Целевой вариант для этой задачи: не добавлять language-specific storage hacks в
surface, а дать renderer-у shape-ить соседние cells как один run. Нужно отдельно
проверить, сможет ли выбранный text stack корректно расположить SARA AM при
run-based shaping без surface decomposition. Surface-level decomposition можно
рассматривать только как явно задокументированный fallback, если lower-level
shaping не способен корректно отрисовать этот случай при сохранении terminal
grid.

## Этапы реализации

### Этап 1. Зафиксировать целевое поведение

- Зафиксировать, что surface остается cell-based source of truth.
- Добавить regression samples для renderer-а:
  - `e\u{0301}`;
  - Thai `ที่นี่`;
  - Thai `น้ำ` и `กำลัง`;
  - Lao `ນ້ຳ` или аналогичный sample с `U+0EB3`;
  - Arabic base + harakat;
  - Hebrew base + niqqud.
- Добавить fixtures для ASCII и colorful TUI layouts, чтобы проверять отсутствие
  regressions в cell geometry.

### Этап 2. RenderRun model

- Добавить private run builder в `otty-ui-terminal`.
- Собрать runs per visible row из snapshot cells.
- Сохранять mapping между run text и terminal columns.
- Split по style boundaries: font, weight, italic, resolved foreground,
  selection, inverse, cursor text override, hovered hyperlink state.
- Не split-ить внутри `base + zerowidth` cluster-а.
- Учитывать wide chars и `WIDE_CHAR_SPACER`.
- Добавить unit-тесты run builder-а без canvas и без shaper-а.

### Этап 3. Lower-level shaping backend

- Реализовать shaping path через `iced_graphics::text::Paragraph` с
  `Wrapping::None` или напрямую через `cosmic_text::Buffer`.
- Явно задавать run bounds: `cell_columns * cell_width`.
- Явно отключать wrapping.
- Применять обязательный clipping по terminal viewport/widget bounds.
- Не включать tight per-run clipping по умолчанию; рассматривать его только
  после измеренной regression и проверки complex-script samples.
- Проверить применимость `set_monospace_width(Some(cell_width))`.
- Сохранять доступ к glyph positions и byte ranges для диагностики и будущего
  cluster-to-cell mapping.

### Этап 4. Интеграция с cell-based geometry

- Оставить background, cursor, selection, hyperlink hover и block UI
  cell-based.
- Заменить только text rendering path на shaped runs.
- Убедиться, что hover hit testing, selection ranges и cursor movement не
  зависят от shaped glyph positions.
- Проверить, что resize не хранит stale text layout state.
- Добавить внутренний renderer mode только если он нужен для сравнения и
  безопасного rollout-а.

### Этап 5. Validation and rollout

- Включать run-based renderer только после regression screenshots и benchmark
  данных.
- Сравнить per-cell и run-based rendering на sample screens.
- Измерить CPU и allocation behavior.
- Проверить hover links, selection, cursor, scroll и resize.
- Если TUI regressions значимые, исправить run construction/alignment, а не
  подменять задачу per-cell workaround-ом.

## Acceptance criteria

- Combining marks больше не теряются при отрисовке.
- `base + zerowidth` cluster всегда передается shaper-у вместе.
- Текстовый renderer shape-ит contiguous runs, а не отдельные cells.
- TUI geometry остается cell-aligned.
- Hyperlink hover hit testing остается cell-based.
- Selection background и cursor не зависят от text shaping.
- Resize не хранит stale layout state.
- Performance на ASCII/TUI viewport не хуже текущей без принятого объяснения.
- Thai/Lao SARA AM проверен на renderer-only shaping; любой fallback на уровне
  surface должен быть отдельно обоснован как последнее средство.

## Открытые вопросы

На текущем уровне проектирования открытых вопросов нет.
