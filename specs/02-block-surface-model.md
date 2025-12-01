# 02. Много‑гридовая модель BlockSurface в otty‑surface

## Обзор

Вторая задача: реализовать в `otty-surface` модель `BlockSurface`, которая внутри хранит несколько `Surface` (по одному на блок) и реализует `SurfaceActor + SurfaceModel`. На этом шаге `BlockSurface` используется вместо базового `Surface`, но snapshot пока может экспортировать только активный блок, чтобы не менять визуальное поведение.

См. спецификацию: `specs/spec.md`, раздел 6.1.

## Что уже есть

- `otty-surface`:
  - `Surface` — основная модель поверхности с гридом, скроллом, режимами и т.д.
  - `SurfaceActor` — трейт с полным набором операций, которые вызываются из `TerminalSurfaceActor`.
  - `SurfaceModel` + `Surface::snapshot_owned` — экспорт плоского snapshot (cells, cursor, selection, damage, и т.п.).
- `otty-libterm`:
  - `TerminalEngine<P, E, S>` параметризован по `S: SurfaceActor + SurfaceModel`.
  - Типы `DefaultSurface`/`DefaultParser` в `terminal::builder` ещё используют обычный `Surface`.

## Что нужно сделать

1. Ввести новые структуры в `otty-surface`:
   - `BlockMeta` (внутреннюю, похожую на RFC):
     - минимум: `id`, `kind`, `started_at`, `finished_at`, `exit_code`, `is_alt_screen`.
   - `Block { meta: BlockMeta, surface: Surface }`.
   - `BlockSurface { blocks: Vec<Block>, active: usize, max_blocks: usize }`.
   - Значения по умолчанию:
     - один блок с пустым `Surface` и `kind = Command` (или `Prompt`) и `active = 0`;
     - `max_blocks` брать из конфига или константы (по умолчанию 1000).

2. Реализовать `SurfaceActor` для `BlockSurface`:
   - Для всех методов `SurfaceActor` делегировать вызовы в `self.blocks[self.active].surface`.
   - На этом шаге **не** менять active‑блок в ответ на какие‑либо события (это будет сделано в следующей задаче).

3. Реализовать `SurfaceModel` для `BlockSurface`:
   - `snapshot_owned()`:
     - пока возвращать snapshot **только активного** блока (используя существующий `Surface::snapshot_owned`);
     - damage/selection/цвета брать из активного `Surface`.

4. Подключить `BlockSurface` как дефолтную поверхность:
   - В `otty-libterm::terminal::builder` заменить `DefaultSurface = Surface` на `DefaultSurface = BlockSurface`.
   - При создании `BlockSurface` инициализировать его одним `Block` с новым `Surface`.

## Ожидаемый результат

- `BlockSurface` реализован и используется вместо `Surface` в `TerminalEngine`.
- При отсутствии DCS‑событий блоков и при `active = 0` поведение терминала полностью идентично прежнему:
  - отображается один грид;
  - snapshot выглядит так же (виден только один блок).

## Как протестировать

1. Автоматические тесты:
   - Тесты `otty-surface` должны проходить без изменений (для старых API).
   - Добавить простые тесты на `BlockSurface`:
     - инициализация с одним блоком;
     - делегирование `print`/`scroll` в активный `Surface` (сравнить с прямым вызовом на `Surface`).

2. Интеграция с `otty-libterm`:
   - `cargo test -p otty-libterm` — все существующие тесты должны проходить.

3. Ручной тест:
   - Собрать проект, запустить UI/пример.
   - Взаимодействовать с терминалом (набор команд, скролл, выделение) и убедиться, что всё ведёт себя так же, как до введения `BlockSurface`.

## Зависимости

- Требует выполненной задачи 01 (тип `Action::BlockEvent` уже может существовать, но `BlockSurface` пока его не использует).
- RFC: `specs/spec.md`, раздел 6.1.
- План: `specs/plan.md`, раздел 2.

