# Фаза 9: дополнительные shells и сложные окружения

Статус: **не начато**.

Родительский документ: [Blocks v2](../blocks-v2.md). Идентификаторы: B2-090–B2-094.

## Цель

Расширить protocol v2 за пределы локальных Bash/Zsh только там, где integration можно
проверить реальным shell lifecycle test. Неподдерживаемое окружение должно честно показывать
`Unsupported`/`Degraded`, а terminal обязан оставаться полностью пригодным без hooks.

## Зависимости

Фаза начинается после стабилизации protocol v2, lifecycle reducer, output routing и
integration status. Установка новых test/runtime dependencies требует предварительного
согласования. Отсутствующий shell binary в CI оформляется capability-gated skip, а не ложный
success.

## Объём работ

- [ ] **B2-090** Fish protocol-v2 hooks и real-shell tests полного lifecycle.
- [ ] **B2-091** PowerShell protocol-v2 hooks и tests на доступных Linux/macOS/Windows CI
  platforms с учётом различий quoting/status.
- [ ] **B2-092** tmux/screen detection, self-test и документированный passthrough без
  автоматического изменения user config.
- [ ] **B2-093** Спроектировать explicit SSH/container bootstrap с threat model, capability
  negotiation, versioning и cleanup review.
- [ ] **B2-094** Добавлять Nushell/другой shell только одновременно с real-shell tests; до
  этого оставлять его `Unsupported`.

## Общая lifecycle matrix для каждого shell

- source/import integration дважды в одном process;
- root и nested shell с уникальными IDs и parent link;
- success, non-zero exit, pipeline status, command-not-found, signal и Ctrl-C;
- multiline command, Unicode, quotes и control-like text;
- сохранение существующих prompt/preexec/precmd/exit hooks;
- shell restart/`exec`, missing loader и unsupported protocol version;
- отсутствие optional external tools;
- clean shell exit и recovery после потерянного command-end.

## Правила безопасности окружений

- Никогда не менять tmux/screen/SSH/shell user config автоматически.
- Remote bootstrap запускается только явным действием и показывает устанавливаемую версию и
  target path.
- Session ID и доверие к local terminal нельзя переносить на remote side без negotiation.
- Временные remote assets имеют ограниченные permissions и документированный cleanup.
- Shell command/output не попадают в connection diagnostics.

## Автоматическая проверка

После реализации каждого adapter добавить отдельную capability-gated команду, например:

```bash
cargo test -p otty --test shell_integration -- --nocapture
```

CI matrix должна явно перечислять найденные `bash`, `zsh`, `fish`, `pwsh`, `tmux` и `screen`.
Merge конкретного adapter запрещён, если его binary доступен, но lifecycle tests skipped.

## Ручная проверка

1. Запустить OTTY с Fish, дождаться `Active v2` и пройти общую lifecycle matrix выше.
2. Повторить в PowerShell на каждой поддерживаемой платформе, отдельно проверив `$LASTEXITCODE`,
   native process exit и pipeline semantics.
3. Запустить Bash/Zsh/Fish внутри tmux и screen. Выполнить команды, nested shell и alt-screen
   приложение; protocol events не должны появляться как видимый мусор или теряться.
4. Отключить passthrough/configuration. UI должен показать диагностируемый degraded status и
   предложить инструкцию, но не редактировать config.
5. Подключиться по SSH/container без remote bootstrap. Terminal работает обычно, integration
   честно сообщает unsupported/degraded.
6. Выполнить explicit remote bootstrap после просмотра target/version. Проверить lifecycle,
   reconnect, version mismatch и cleanup.
7. Отказать в bootstrap или оборвать соединение посередине. Remote user files не должны быть
   повреждены; partial temporary assets удаляются или документированно обнаруживаются.
8. Выбрать неизвестный shell. Приложение не падает и не имитирует block lifecycle, а показывает
   `Unsupported(<shell>)`.

Фаза готова только для тех environments, которые прошли реальный lifecycle matrix. Наличие
непроверенного script prototype не считается поддержкой.

