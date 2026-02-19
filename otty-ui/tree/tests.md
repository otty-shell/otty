# Тест-план для `otty-ui-tree` (цель: coverage > 80%)

## 1. Цель
- Покрыть ветви в `src/model.rs` и `src/view.rs`, а не только happy-path.
- Достичь минимум 80% line coverage по crate `otty-ui-tree`.
- Проверить корректность публичного API (`flatten_tree`, `TreeView`) и критичных приватных веток через unit-тесты в соответствующих модулях.

## 2. Базовые тестовые фикстуры
- `TestNode` с вариантами `Folder`/`File`, полями `title`, `expanded`, `children`, и `TreeNode` impl.
- В `view`-тестах `Message` enum и счетчики через `Rc<Cell<usize>>`/`Rc<RefCell<Vec<_>>>`, чтобы проверять, какие callback-ы реально вызываются при сборке `view()`.
- Вспомогательные фабрики:
  - `sample_tree_flat()` (2-3 уровня вложенности, смесь файлов и папок).
  - `sample_tree_with_numbers()` (`file1`, `file02`, `file10`, `a2`, `a10`).
  - `sample_tree_hidden_children()` (папка `expanded=false`).

## 3. Кейсы для `src/model.rs`

### 3.1 `flatten_tree` / `push_node`
- `M01`: пустой входной список возвращает пустой результат.
- `M02`: один файл -> один элемент, `depth=0`, путь из одного сегмента.
- `M03`: папка `expanded=false` -> в результате только папка, дети скрыты.
- `M04`: папка `expanded=true` с детьми -> дети видимы, `depth` увеличен на 1.
- `M05`: вложенные папки 2+ уровней -> корректные `depth` и полный путь.
- `M06`: две ветви на одном уровне -> path не «протекает» между sibling-узлами.
- `M07`: папка с `children() = None` и `expanded=true` -> паники нет, рекурсии нет.
- `M08`: сортировка применяется на каждом уровне дерева, не только в корне.

### 3.2 `sorted_indices` / `compare_titles`
- `M09`: папки сортируются перед файлами.
- `M10`: текстовая сортировка case-insensitive (`alpha` и `Bravo`).
- `M11`: если lower-case равны (`a` vs `A`), срабатывает tie-break `left.cmp(right)`.
- `M12`: числовая сортировка `file2 < file10`.
- `M13`: ведущие нули (`file2`, `file02`, `file002`) сравниваются по длине исходного сегмента при равном значении.
- `M14`: чисто нулевые сегменты (`0`, `00`, `000`) обрабатываются как число 0 без ошибок.
- `M15`: digit-segment vs text-segment (`a1` vs `aa`) ветка `Digits < Text`.
- `M16`: text-segment vs digit-segment (`aa` vs `a1`) ветка `Text > Digits`.
- `M17`: один заголовок является префиксом другого (`abc` vs `abc1`) покрытие веток `(Some(_), None)` и `(None, Some(_))`.
- `M18`: полностью равные заголовки -> `Ordering::Equal`.
- `M19`: смешанные сегменты (`a9b10`, `a9b2`) корректно сравниваются по первому отличию.
- `M20`: не-ASCII цифры (например `١٢`) не считаются digits (`is_ascii_digit`), остаются text.

### 3.3 `split_segments` / сегментация
- `M21`: пустая строка -> пустой список сегментов.
- `M22`: строка только из текста -> один `Text`.
- `M23`: строка только из цифр -> один `Digits`.
- `M24`: чередование text/digits (`ab12cd34`) -> 4 сегмента в правильном порядке.
- `M25`: переходы типа digits->text и text->digits покрывают обе ветки `Some(kind) => ...`.

### 3.4 Проперти/инварианты (рекомендуется)
- `M26`: антисимметрия компаратора заголовков: `cmp(a,b) == reverse(cmp(b,a))`.
- `M27`: транзитивность сортировки на случайных заголовках.
- `M28`: для каждого результата `flatten_tree`: `entry.path.len() == entry.depth + 1`.

## 4. Кейсы для `src/view.rs`

### 4.1 Builder API и базовый рендер
- `V01`: `TreeView::new(...).view()` на пустом дереве не паникует.
- `V02`: `selected(Some(path))` корректно помечает `is_selected=true` только у нужной строки.
- `V03`: `hovered(Some(path))` корректно помечает `is_hovered=true` только у нужной строки.
- `V04`: при `selected/hovered=None` оба флага всегда `false`.
- `V05`: `on_press` генерирует сообщение при клике по строке.
- `V06`: `indent_width(<0)` клампится к `0.0`.
- `V07`: `toggle_width(<0)` клампится к `0.0`.
- `V08`: `spacing` принимает заданное значение (smoke: без паники при `view()`).

### 4.2 Видимость/интерактивность строк
- `V09`: без `row_visible` все строки считаются видимыми.
- `V10`: `row_visible=false` скрывает строку (render_row и row_style для нее не вызываются).
- `V11`: `before_row` и `after_row` вызываются даже для невидимой строки.
- `V12`: без `row_interactive` строка интерактивна по умолчанию.
- `V13`: `row_interactive=false` отключает mouse handlers, но строка продолжает рендериться.

### 4.3 Callback-ы мыши (`wrap_mouse_area`)
- `V14`: если все callbacks `None`, `wrap_mouse_area` возвращает исходный element.
- `V15`: `on_press` вызывается для каждой видимой интерактивной строки.
- `V16`: `on_release` вызывается аналогично.
- `V17`: `on_right_press` вызывается аналогично.
- `V18`: `on_hover` генерирует `Some(path)` при входе и `None` при выходе.
- `V19`: `on_hover` получает корректный `TreePath` (глубокий путь для вложенных узлов) при `Some(path)`.
- `V20`: при `row_interactive=false` hover-события строки не генерируются.

### 4.4 Toggle slot (`build_toggle_slot`)
- `V24`: если `toggle_width==0` и `toggle_content=None`, toggle-slot не создается.
- `V25`: если `toggle_width>0`, slot создается даже без `toggle_content`.
- `V26`: если `toggle_content=Some`, slot создается даже при `toggle_width==0`.
- `V27`: кастомный `toggle_content` вызывается для видимых строк при создании slot.
- `V28`: для folder + `on_toggle_folder` генерируется toggle message.
- `V29`: для file + `on_toggle_folder` toggle message не генерируется.
- `V30`: если toggle нет, но есть `on_hover`, slot остается hover-интерактивным.
- `V31`: если нет toggle и нет `on_hover`, slot остается статичным.

### 4.5 Decorators (`before/after`)
- `V32`: `before_row(Some(...))` может вставлять элемент до конкретной строки.
- `V33`: `before_row(None)` ветка без вставки.
- `V34`: `after_row(Some(...))` может вставлять элемент после конкретной строки.
- `V35`: `after_row(None)` ветка без вставки.

### 4.6 Стили и компоновка
- `V40`: `row_style` вызывается для каждой видимой строки.
- `V41`: без `row_style` стиль-контейнер не применяется.
- `V42`: при `indent_width>0` и `depth>0` добавляется левый отступ (ветка `indent > 0.0`).
- `V43`: при `indent_width>0`, но `depth=0` ветка без добавления `Space`.
- `V44`: полный комбинированный сценарий (selected+hovered+visible filter+interactive filter+toggle) без паник и с ожидаемым числом вызовов callback-ов.

## 5. Интеграционные кейсы через публичный API (`src/lib.rs`)
- `I01`: `otty_ui_tree::{flatten_tree, TreeView, TreeNode, TreePath}` доступны и компилируются при внешнем использовании.
- `I02`: end-to-end smoke: собрать `TreeView` с минимумом колбеков и вызвать `.view()` на дереве с 1 папкой и 1 файлом.

## 6. Минимальный набор для уверенного >80%
- Покрыть все кейсы `M01..M25` и `V01..V31`, плюс `V40`, `V41`, `V44`.
- Этого достаточно, чтобы закрыть почти все условные ветви в `model.rs` и основной pipeline рендера в `view.rs`.
- Property-тесты (`M26..M28`) усиливают надежность сортировки, но не обязательны для метрики 80%.

## 7. Команды проверки
```bash
cargo test -p otty-ui-tree
cargo llvm-cov -p otty-ui-tree --lib --summary-only
```

Если в окружении нет `cargo-llvm-cov`, установить:
```bash
cargo install cargo-llvm-cov
```
