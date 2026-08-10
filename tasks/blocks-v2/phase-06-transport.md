# Фаза 6: latest-frame transport

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-060–B2-066.

## Цель

Разделить заменяемое визуальное состояние и lossless terminal events. Медленный UI должен
получать последнюю доступную frame revision, не создавая backlog полных snapshots, при этом
child exit, errors и другие критические события не теряются.

## Текущее состояние

Добавлен latest-frame mailbox ёмкостью один, frame replacement counter, отдельная
`FrameReady` notification и bounded default capacities для event/request queues. Slow/full/
disconnected channel cases покрыты tests.

Фаза не завершена: PTY reads не coalesce-ятся в render ticks, отсутствуют model/viewport
revisions, stale coordinate handling и partial damage. Нужен полноценный burst test с
искусственно медленным consumer и гарантированным lossless child exit.

## Объём работ

- [ ] **B2-060** Расширить slow-consumer tests до burst PTY output + lossless child exit;
  текущие mailbox/channel tests являются частью пункта.
- [x] **B2-061** Разделить replaceable frame notification и lossless terminal events.
- [x] **B2-062** Latest-frame mailbox ёмкостью один без новой dependency.
- [x] **B2-063** Bounded default queues и явные full/disconnected semantics.
- [ ] **B2-064** Coalesce PTY reads в render ticks; resize/scroll/selection должны запрашивать
  immediate render.
- [ ] **B2-065** Передавать `model_revision`/`viewport_revision` и reject/re-resolve stale
  coordinate requests через stable positions.
- [ ] **B2-066** Реализовать partial damage для output и full damage для resize/reset/
  alt-screen transitions.

## Инварианты transport

- В памяти находится не более одного непрочитанного replaceable frame.
- Замена frame не блокирует PTY reader и учитывается безопасным counter.
- Lossless event никогда не маскируется frame notification и явно обрабатывает full queue.
- Presented revision не должна откатываться назад.
- Coordinate request либо относится к объявленной revision, либо разрешается через stable ID.
- Shutdown/disconnect не оставляет producer в бесконечном retry loop.

## Автоматическая проверка

```bash
cargo test -p otty-libterm terminal::channel
cargo test -p otty-libterm --all-features
cargo test -p otty-ui-term --all-features
```

Добавить deterministic test с paused/slow consumer: producer отправляет burst frames и child
exit, backlog остаётся ≤ 1 frame, consumer получает последнюю revision и child exit.

## Ручная проверка

1. Запустить `cargo run -p otty` и команду `yes transport | head -n 1000000`.
2. Во время вывода менять размер окна, переключать terminal tabs и прокручивать history. UI
   должен оставаться отзывчивым, а PTY output — завершиться без зависания.
3. На время искусственно замедлить UI consumer через test/debug option. По diagnostics
   проверить: replaceable backlog ≤ 1, `replaced_frames` растёт, lossless depth ограничена.
4. Запустить короткоживущий child одновременно с burst output. Его exit event должен прийти
   даже если несколько frames были заменены.
5. Выполнить scroll/selection request на старой revision. Backend должен явно отклонить его
   либо повторно разрешить через stable `BlockPoint`, но не применить к другому тексту.
6. Проверить resize, reset и alt-screen transition: они создают full damage; обычный append
   output сообщает только изменённую область.
7. Закрыть terminal tab во время burst. Процессы и channels должны завершиться без panic,
   deadlock или бесконечного CPU loop.

Фаза готова, когда миллион строк не создаёт frame backlog, последняя revision достигает UI, а
lossless lifecycle events подтверждённо доставляются.

