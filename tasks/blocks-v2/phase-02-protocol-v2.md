# Фаза 2: protocol v2 parser

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-020–B2-027.

## Цель

Ввести bounded и версионированный wire protocol между shell hooks и terminal backend. Parser
должен переживать fragmentation, malformed/oversized input и чужие session events без panic,
unbounded allocation или повреждения block lifecycle.

## Текущее состояние

Реализованы `event-v2;h`, bounded hex/JSON parsing, typed envelope, все основные semantic
events, OSC 133 A/B, terminal session validation, per-shell sequence diagnostics и wire-format
documentation. Session ID создаётся из системного `/dev/urandom`, поэтому новая dependency не
потребовалась.

V1 `block` parser, schema и production action dispatch удалены; legacy-v1 fragmented и
malformed frames безопасно игнорируются, после чего parser продолжает принимать v2. Фаза не
завершена: остаётся randomized arbitrary-DCS allocation/panic test.

## Объём работ

- [x] **B2-020** Framing/schema/recovery tests для v2 и fragmented stream tests.
- [x] **B2-021** После падающего legacy-ignore regression из B2-002 удалить v1
  `dcs/block.rs`, `DcsMessageKind::Block` и production dispatch в
  `Action::BlockEvent`; legacy `otty-dcs;block` должен безопасно отбрасываться как
  unsupported DCS и не создавать semantic event.
- [x] **B2-022** Bounded framing, hex decode и обязательная major version.
- [x] **B2-023** Semantic events `shell_hello`, `prompt_prepare`, `command_start`,
  `command_end`, `context_update`, `shell_exit`, `integration_error`.
- [x] **B2-024** OSC 133 A/B и semantic prompt boundaries.
- [x] **B2-025** System-random session registration и rejection foreign/missing session.
- [x] **B2-026** Per-shell sequence validation и payload-free diagnostics.
- [x] **B2-027** Wire format, limits и Bash/Zsh examples в `otty-escape/README.md`.
- [ ] Добавить deterministic/random byte-stream test, доказывающий bounded buffer для
  произвольного DCS input.

## Контракт безопасности

- decoded payload не больше 32 KiB, encoded payload не больше 64 KiB;
- command и идентификаторы имеют отдельные меньшие limits;
- неизвестная major version не применяется к model;
- malformed hex/JSON/UTF-8 отбрасывается и parser продолжает принимать последующие bytes;
- diagnostic содержит только тип ошибки и безопасные IDs/revisions, но не command/output;
- terminal принимает event только для зарегистрированной `TerminalSessionId`.

## Автоматическая проверка

```bash
cargo test -p otty-escape dcs
cargo test -p otty-escape osc
cargo test -p otty-surface block::lifecycle
cargo clippy -p otty-escape --all-targets --all-features -- -D warnings
```

## Ручная проверка

1. Запустить `cargo run -p otty` и дождаться статуса `Integration v2 active`.
2. Выполнить несколько обычных команд и убедиться, что prompt/command completion продолжают
   создавать блоки.
3. Отправить malformed DCS: `printf '\033Pevent-v2;h;zz\033\\'`.
4. Сразу выполнить `printf 'parser-alive\n'`. Терминал должен продолжить работу, malformed
   sequence не должна создать или завершить блок.
5. Отправить encoded payload больше 64 KiB, затем снова выполнить обычную команду. UI не
   должен зависнуть или заметно увеличить retained memory после discard.
6. Открыть вторую terminal tab. События и block IDs двух вкладок не должны смешиваться.
7. В Bash и Zsh выполнить команды с Unicode, кавычками, newline/heredoc и текстом, похожим на
   ESC/BEL. В block metadata должен попасть исходный command без инъекции control sequence.
8. Повторить fragmented и unsupported-version cases через соответствующие parser tests с
   `--nocapture` и убедиться, что diagnostic не печатает payload.

Фаза готова после удаления v1 production parser, прохождения legacy-ignore теста и
malformed/oversized/foreign session scenarios без corruption соседних блоков.
