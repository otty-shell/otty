# Фаза 3: Bash/Zsh lifecycle и bootstrap

Статус: **частично выполнено**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-030–B2-039.

## Цель

Обеспечить полный lifecycle root и nested Bash/Zsh: уникальные shell/block IDs, точный exit и
pipeline status, prompt boundaries, сохранение пользовательских hooks и явный integration
status. Ошибка bootstrap не должна ломать обычный terminal.

## Текущее состояние

Bash/Zsh assets переведены на protocol v2. Добавлены PID-scoped guards, parent links,
dependency-free JSON/hex encoding, OSC 133, exact completion events, atomic asset writes с
permissions `0600`, system-random session IDs, concurrent bootstrap tests и UI statuses
`Pending/Active/Degraded/Unsupported`. Bash OSC markers используют невидимый `PS1` syntax;
regression с Fedora/Bash 5.3 `${PROMPT_START@P}` не выводит служебный текст в prompt.

Фаза не завершена: assets ещё готовятся внутри общего `services.rs`, нет handshake timeout,
явного install/uninstall persistent loader и полного PTY test matrix для signal, Ctrl-C,
heredoc, hook reload и `exec shell`.

## Объём работ

- [ ] **B2-030** Расширить real-shell harness до полного PTY lifecycle matrix; текущие process
  tests покрывают source-twice, nested IDs, exit/pipeline и часть existing hooks.
- [x] **B2-031** Bash v2 с ранним захватом status.
- [x] **B2-032** Zsh v2 с `$?`, `pipestatus` и chaining существующих hooks.
- [x] **B2-033** OSC 133 A/B без изменения видимого prompt, включая Fedora/Bash 5.3
  `${PROMPT_START@P}` regression.
- [x] **B2-034** PID-scoped idempotency и parent/current shell context IDs.
- [x] **B2-035** Encoding без per-prompt `jq`, Python или Perl.
- [ ] **B2-036** Вынести versioned asset/bootstrap logic в focused
  `terminal_workspace/shell_integration/`; atomic write уже реализован.
- [ ] **B2-037** Добавить handshake timeout и переход `Pending` → `Degraded`; остальные status
  variants уже представлены в model/UI.
- [ ] **B2-038** Реализовать только явный install/uninstall persistent loader с preview и
  подтверждением; никогда не менять `.bashrc`/`.zshrc` автоматически.
- [x] **B2-039** Concurrent bootstrap test нескольких terminal sessions.

## Автоматическая проверка

```bash
bash -n assets/shell-integrations/otty.bash
zsh -n assets/shell-integrations/otty.zsh
cargo test -p otty --test shell_integration -- --nocapture
cargo test -p otty terminal_workspace::services
```

Capability-gated Zsh tests должны явно сообщать о skip, если binary отсутствует. Основные Bash
tests не должны silently skip.

## Ручная проверка

1. Запустить `cargo run -p otty` с Bash и убедиться, что status меняется с `Pending` на
   `Integration v2 active`.
2. На Fedora/Bash 5.3 убедиться, что prompt начинается с обычного `user@host`, без видимого
   `PROMPT_START@P}` или `PROMPT_END@P}`. OSC markers не должны менять расчёт ширины prompt.
3. Выполнить `false`, `false | true`, неизвестную команду и команду, завершённую Ctrl-C.
   Результат каждого command block должен соответствовать фактическому outcome.
4. Дважды source-нуть integration asset и выполнить одну команду. На каждую lifecycle phase
   должно приходиться одно событие и один блок.
5. Запустить nested interactive Bash, выполнить две команды, выйти и продолжить в parent.
   Child должен иметь другой shell instance и не менять parent blocks.
6. Повторить Bash lifecycle в Zsh, дополнительно проверив pipeline status.
7. Открыть одновременно несколько terminal tabs. Все должны получить разные session IDs и
   перейти в `Active`, без race при создании integration files.
8. Запустить профиль с неподдерживаемым shell и убедиться, что terminal работает, а UI
   показывает `Unsupported`, а не бесконечный `Pending`.
9. Смоделировать ошибку записи bootstrap directory. Terminal должен запуститься в
   `Degraded(bootstrap_failed)` без изменения пользовательских rc-файлов.
10. После реализации loader проверить preview, отказ от подтверждения, install, повторный
   install и uninstall на временном HOME; исходный rc-файл должен восстанавливаться точно.

Фаза готова, когда root/nested Bash и Zsh проходят полную PTY matrix, timeout различает
неактивную integration, а persistent loader управляется только явным действием пользователя.
