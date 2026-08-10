# Фаза 5: height index, viewport и selection

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-050–B2-058.

## Цель

Сделать viewport устойчивым к росту active output, resize/reflow, freeze, truncation и
presentation changes. Scroll и selection должны ссылаться на stable block/logical positions,
а поиск видимого диапазона не должен сканировать всю историю.

## Текущее состояние

Реализованы базовые `ScrollPosition::FollowTail/Anchored`, block anchor при manual scroll,
поведение при удалении anchored block и model-level off-screen `ScrollToBlock` с alignment.

Фаза не завершена: отсутствуют `BlockHeightIndex`, единый `Viewport::apply_change`, stable
`LineId`/`BlockPoint`, lazy reflow, viewport-only snapshot и benchmark отсутствия full-history
scan. Selection всё ещё зависит от stitched visual coordinates.

## Объём работ

- [ ] **B2-050** Перенести полную regression matrix раздела 13.2 в focused viewport tests;
  существующие anchor/ScrollTo tests покрывают только часть.
- [ ] **B2-051** Реализовать `BlockHeightIndex` с randomized comparison против `Vec` reference.
- [ ] **B2-052** Завершить `FollowTail/Anchored` через единый `Viewport::apply_change`, через
  который проходят append, resize, freeze, truncate, collapse и presentation update.
- [ ] **B2-053** Ввести stable `LineId` и logical-line → wrapped-row mapping.
- [ ] **B2-054** Перевести selection на `BlockPoint` и удалить global stitched-row identity.
- [ ] **B2-055** Строить snapshot только для viewport + bounded overhang.
- [ ] **B2-056** Подключить `ScrollToBlock` к height index и проверить start/center/end/nearest;
  model request и базовый alignment уже существуют.
- [ ] **B2-057** Удалить resize всех historical surfaces, применять lazy frozen reflow и
  обновлять height cache только затронутых blocks.
- [ ] **B2-058** Benchmark-ом доказать отсутствие full-history grid scan на frame path.

## Инварианты viewport

- В `FollowTail` новый output удерживает нижний край видимым.
- В `Anchored` новый output ниже anchor не меняет верхнюю logical position пользователя.
- Resize меняет wrapping, но сохраняет anchored `LineId` и logical selection endpoints.
- При удалении/truncation anchor выбирает документированного ближайшего соседа.
- Collapse/reorder выше viewport компенсирует изменение высоты без visual jump.
- Off-screen lookup работает по ID независимо от наличия block в последнем snapshot.

## Автоматическая проверка

```bash
cargo test -p otty-surface block
cargo test -p otty-ui-term block_layout
```

Randomized test должен выполнять insert/remove/height-change/prefix-sum/range lookup и после
каждой операции сравнивать `BlockHeightIndex` с простым reference vector.

## Ручная проверка

1. Запустить `cargo run -p otty`, создать не менее 100 блоков и уйти в середину history.
2. Запустить active command с длинным output. Видимая anchored строка не должна двигаться;
   после возврата вниз viewport должен снова перейти в `FollowTail`.
3. Изменить ширину 80 → 200 → 40 columns. Верхняя logical line и выделенный текст должны
   остаться теми же, хотя visual wrapping изменится.
4. Выделить текст в старом блоке, продолжить output и повторить resize. Copy Selection должен
   вернуть те же logical cells.
5. Вызвать `ScrollToBlock` для полностью off-screen block с align start, center, end и nearest;
   каждый режим должен давать документированное положение.
6. Collapse/expand и переместить block выше viewport после появления presentation actions.
   Текущий видимый контент не должен прыгать.
7. Превысить history budget так, чтобы truncation затронул область до, на и после anchor;
   проверить три документированных recovery outcome.
8. Повторить вход/выход из alt screen и убедиться, что normal scroll state восстановлен.
9. На истории из 10 000 blocks включить metrics и подтвердить, что lookup/snapshot не
   посещают все grids и размер snapshot ограничен viewport + overhang.

Фаза готова, когда вся regression matrix не показывает scroll jumps, selection имеет stable
logical identity, а frame cost не зависит линейно от полной истории.

