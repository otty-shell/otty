# Миграция OTTY с Iced на GPUI

## Статус

- Состояние: запланировано.
- Приоритет: высокий.
- Тип работы: поэтапная миграция GUI с сохранением терминального ядра.
- Целевые платформы: Linux (X11 и Wayland) и macOS (Intel и Apple Silicon).
- Ориентировочный объём: 30–49 инженерных дней, включая стабилизацию и упаковку.

## Цель

Перенести приложение OTTY с Iced 0.14 на GPUI, сохранив существующее
терминальное ядро, пользовательские данные и поведение продукта.

Миграция должна дать:

- GPUI-приложение с тем же набором пользовательских возможностей;
- собственный GPUI terminal element с эффективной GPU-отрисовкой;
- независимое от GUI терминальное и бизнес-состояние;
- полное удаление Iced из итогового runtime dependency graph;
- сохранение поддержки Linux и macOS и текущих форматов пакетов;
- отсутствие регрессий в PTY, SSH, escape parsing, scrollback, blocks,
  Unicode shaping, selection, clipboard и shell integration.

## Краткое обоснование

Репозиторий уже имеет подходящую для миграции границу:

- `otty-pty` отвечает за local PTY и SSH;
- `otty-vte` и `otty-escape` разбирают terminal sequences;
- `otty-surface` хранит screen, history, selection, blocks и render snapshots;
- `otty-libterm` связывает session, parser, surface и runtime через
  `TerminalRequest`, `TerminalEvent`, `TerminalHandle` и `SnapshotOwned`;
- `otty-ui/terminal` и `otty` содержат Iced-specific rendering, input,
  subscriptions, tasks, focus, geometry и widgets.

Поэтому миграция не должна переписывать терминальный backend. Основная работа
сосредоточена в presentation/runtime слое и в отделении бизнес-состояния от
типов Iced.

## Обязательные ограничения

- Писать тесты до реализации каждого бизнес-значимого изменения.
- Не добавлять новые зависимости без предварительного согласования с
  владельцем проекта. В запросе на согласование указать назначение, версию,
  лицензию, альтернативы и причину добавления.
- Для Rust mocks использовать только `mockall` и только когда mock решает
  реальную задачу на инфраструктурной границе.
- Не создавать универсальный GUI abstraction layer между Iced и GPUI.
- Не создавать speculative traits, managers, factories или services.
- Выносить общую логику только при наличии текущей границы между backend и
  frontend либо двух реальных потребителей во время параллельной миграции.
- Сохранять простые конкретные типы и явные feature effects.
- Не использовать `unwrap()` в production-коде.
- Новые публичные элементы API документировать краткими doc comments и
  примерами, где пример действительно помогает понять контракт.
- Сохранять префикс `otty-` у новых crates.
- Портировать приложение вертикальными срезами, не одним big-bang commit.

## Вне области задачи

- Переписывание VTE parser, escape parser, terminal surface или PTY без
  подтверждённой необходимости.
- Добавление новых продуктовых возможностей одновременно с миграцией.
- Редизайн UX, тем, command model или форматов пользовательских данных.
- Добавление Windows в release matrix. Архитектура не должна блокировать
  Windows, но parity и packaging для Windows являются отдельной задачей.
- Использование внутренних GPL crates Zed в OTTY.
- Обязательный переход на стороннюю component library до окончания spike.

## Предварительные решения по зависимостям

Перед первым изменением `Cargo.toml` запросить отдельное согласование.

Минимально ожидаемые зависимости:

- `gpui = "=0.2.2"` — core UI, entities, elements, drawing, input и testing;
- `gpui_platform = "=0.2.2"` — platform application, Metal, X11 и Wayland.

Версии должны быть зафиксированы точно. Использовать `*`, плавающий Git branch
или неприкреплённый `main` запрещено: GPUI pre-1.0 и допускает breaking changes.

Отдельно оценить, но не добавлять автоматически:

- `gpui-component` — готовые Input, Button, Tree, Menu и Dock/Tiles;
- assets crate для иконок, если он потребуется component library.

Решение о `gpui-component` принять после короткого spike по следующим
критериям:

- совместимость с выбранной точной версией GPUI;
- стабильность API и возможность pin на release/commit;
- Apache-2.0 совместимость и результат `cargo deny`;
- наличие нужных Input, Tree, context menu и split/dock contracts;
- стоимость адаптации темы OTTY;
- размер dependency graph и влияние на build/package size;
- отсутствие необходимости тянуть несвязанные editor/LSP/chart features.

Если библиотека не проходит spike, реализовать небольшой набор конкретных
OTTY components поверх GPUI primitives. Не создавать общую UI-библиотеку.

## Целевая архитектура

```text
otty-gpui
  AppView
    ChromeView
    SidebarView
      ExplorerView
      QuickLaunchView
    TabsView
    TerminalWorkspaceView
      PaneTree
        Entity<TerminalView>
          TerminalElement
    SettingsView

otty-app-core
  feature state
  domain intents
  concrete feature effects
  storage and validation

otty-ui-gpui-terminal
  TerminalView entity
  TerminalElement
  input translation
  render-run construction
  GPUI text shaping and painting

otty-libterm
  TerminalBuilder
  Runtime
  TerminalHandle / TerminalEvents
  TerminalRequest / TerminalEvent
  SnapshotOwned

otty-surface / otty-escape / otty-vte / otty-pty
```

Названия новых crates являются рабочими и должны быть подтверждены при первом
структурном PR. `otty-app-core` оправдан только как временно общий потребитель
для Iced и GPUI и как постоянное место бизнес-состояния после cutover.

### Границы ответственности

`otty-libterm`:

- владеет terminal runtime и protocol между frontend и backend;
- не зависит от GPUI, Iced, font system, clipboard или window geometry;
- выдаёт immutable `Arc<SnapshotOwned>`;
- принимает конкретные terminal requests.

`otty-app-core`:

- хранит только domain/business state и framework-neutral identifiers;
- не содержит `iced::Task`, `iced::Subscription`, `gpui::Entity`,
  `FocusHandle`, framework geometry или renderer types;
- reducers возвращают изменения состояния и конкретные feature effects;
- async execution остаётся обязанностью frontend runtime.

`otty-ui-gpui-terminal`:

- владеет GPUI entity terminal view;
- хранит последний `Arc<SnapshotOwned>`, focus/input state и render caches;
- переводит GPUI input в `TerminalRequest`;
- отвечает за shaping, painting, selection hit testing, hyperlinks и block UI;
- не дублирует surface или escape state.

`otty-gpui`:

- открывает window и собирает root view;
- исполняет feature effects через GPUI executor/platform APIs;
- управляет entities, subscriptions, focus и window lifecycle;
- содержит только application composition и presentation state.

### Модель владения и потоков

- GPUI entities изменяются только на UI thread через `Context<T>`.
- Terminal runtime продолжает работать в существующем background thread.
- Между runtime и UI используются существующие bounded/unbounded channels из
  `otty-libterm`.
- `TerminalView` хранит `TerminalHandle` и последний `Arc<SnapshotOwned>`.
- Event consumer не держит mutable reference на GPUI entity через `await`.
  Он обновляет `WeakEntity<TerminalView>` через async context и вызывает
  `cx.notify()`.
- Закрытие entity должно останавливать terminal runtime через
  `TerminalRequest::Shutdown`; shutdown обязан быть идемпотентным.
- Не копировать terminal grid на каждый frame. Использовать `Arc` snapshots и
  borrowing внутри layout/paint phase.
- Не блокировать UI thread ожиданием PTY, SSH, filesystem watcher или storage.

## Миграционная стратегия

Текущий Iced frontend остаётся рабочим эталоном до достижения feature parity.
GPUI frontend добавляется рядом и собирается отдельным binary package.

Каждый этап оформляется отдельным небольшим PR:

1. Сначала failing tests или characterization tests.
2. Затем минимальная реализация этапа.
3. Затем проверки и ручной сценарий.
4. Только после выполнения exit criteria начинается следующий этап.

Нельзя удалять Iced implementation до окончания release candidate и проверки
rollback path.

## Этап 0. Зафиксировать baseline и решения

Оценка: 1–2 дня.

### Задачи

- [ ] Зафиксировать актуальные поддерживаемые сценарии в feature matrix.
- [ ] Записать baseline startup time, idle CPU, memory, frame time и input
  latency на одной Linux и одной macOS машине.
- [ ] Сохранить набор visual reference screenshots для основных экранов.
- [ ] Зафиксировать terminal replay fixtures:
  - ASCII и ANSI colors;
  - wide CJK characters;
  - combining marks и Hebrew niqqud;
  - emoji/fallback fonts;
  - hyperlinks;
  - alternate screen;
  - mouse reporting;
  - block begin/end metadata;
  - большой поток вывода и scrollback.
- [ ] Исправить или отдельно зарегистрировать существующий baseline failure
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` в
  `otty-libterm/examples/unix_shell.rs`.
- [ ] Согласовать расхождение coverage: фактический baseline line coverage
  около 67.66%, тогда как repository policy требует не менее 80%.
- [ ] Запросить согласование `gpui` и `gpui_platform`.
- [ ] Выполнить отдельный spike `gpui-component` без включения в production
  dependency graph либо документировать решение не использовать его.
- [ ] Подтвердить имена новых crates и временного GPUI binary.

### Exit criteria

- Baseline воспроизводим и приложен к задаче/PR.
- Все известные исходные failures отделены от миграционных regressions.
- Dependency decision принят владельцем проекта.
- Feature matrix и target platforms утверждены.

## Этап 1. Отделить framework-neutral state

Оценка: 3–5 дней.

### Test-first задачи

- [ ] Добавить characterization tests для reducers, которые сейчас возвращают
  `iced::Task`.
- [ ] Добавить tests для порядка и состава effects при:
  - открытии/закрытии/активации tabs;
  - terminal title change и shutdown;
  - синхронизации explorer с terminal CWD;
  - загрузке и сохранении settings;
  - подготовке и отмене Quick Launch;
  - обработке context menu guards.
- [ ] Добавить tests для framework-neutral pane tree, если выбран собственный
  split tree вместо стороннего Dock/Tiles.

### Реализация

- [ ] Выделить business/domain state в `otty-app-core` либо в library target
  существующего `otty`, согласно утверждённому structural decision.
- [ ] Заменить возвращаемые reducers `iced::Task` на конкретные feature
  effects без универсального command abstraction.
- [ ] Оставить focus handles, pointer coordinates, hover state и overlay
  placement во frontend-specific слое.
- [ ] Заменить `iced::Point`/`Size` в business code на локальные простые types
  только там, где geometry действительно является частью domain contract.
- [ ] Не переносить `iced::widget::Id` или `gpui::FocusHandle` в app core.
- [ ] Перенести backend session configuration из `otty-ui-term` ближе к
  `otty-libterm`, поскольку local/SSH session options не являются UI state.
- [ ] Оставить font/theme configuration в конкретных frontends.
- [ ] Обеспечить совместимость форматов `settings.json` и Quick Launch data.
- [ ] Подключить существующий Iced frontend к выделенному state, чтобы
  подтвердить отсутствие поведенческой регрессии.

### Exit criteria

- App core собирается без Iced и GPUI.
- Business reducers тестируются без UI runtime.
- Существующий Iced binary продолжает работать.
- Форматы пользовательских данных не изменились.

## Этап 2. GPUI application shell и terminal event bridge

Оценка: 2–3 дня.

### Test-first задачи

- [ ] Добавить tests на преобразование `TerminalEvent` в состояние
  `TerminalView`.
- [ ] Проверить, что `Frame` заменяет snapshot и инициирует notify.
- [ ] Проверить title/reset title, bell и child exit.
- [ ] Проверить shutdown при drop/release entity.
- [ ] Проверить закрытый channel и backpressure без panic.

### Реализация

- [ ] Добавить согласованные GPUI dependencies.
- [ ] Создать временный package/binary `otty-gpui`.
- [ ] Настроить `gpui_platform::application()`:
  - window title;
  - minimum size 800×600;
  - app icon;
  - undecorated/custom titlebar behavior;
  - embedded fonts;
  - Linux X11/Wayland features;
  - macOS Metal/font-kit configuration.
- [ ] Создать root `AppView` с минимальным placeholder layout.
- [ ] Создать `TerminalView` entity и подключить `TerminalBuilder` напрямую к
  `otty-libterm`.
- [ ] Подключить background event consumer к `WeakEntity<TerminalView>`.
- [ ] Реализовать clean shutdown без утечек threads/tasks/subscriptions.
- [ ] Показывать backend initialization error внутри GPUI window, а не только
  в log.

### Exit criteria

- GPUI binary открывает окно и запускает один local shell.
- Snapshot updates доходят до entity без polling UI thread.
- Закрытие окна завершает terminal thread.
- На idle нет постоянного redraw loop.

## Этап 3. GPUI terminal renderer

Оценка: 4–7 дней.

### Test-first задачи

- [ ] Портировать framework-neutral tests из:
  - `render_runs.rs`;
  - `shaped_text.rs`;
  - `block_layout.rs`;
  - `block_controls.rs`.
- [ ] Добавить tests на преобразование surface cells в render runs.
- [ ] Проверить wide char spacers, zero-width combining marks и fallback fonts.
- [ ] Проверить стабильность glyph geometry при изменении только colors.
- [ ] Проверить cursor shapes и hidden cursor.
- [ ] Проверить full и partial snapshot damage.
- [ ] Проверить block highlight/divider/action geometry.

### Реализация

- [ ] Реализовать конкретный `TerminalElement` через GPUI `Element` или
  `Canvas`; выбор зафиксировать в коротком ADR в PR description.
- [ ] В layout phase вычислять cell size и terminal rows/columns.
- [ ] Throttle/coalesce resize events и отправлять `TerminalRequest::Resize`.
- [ ] В prepaint phase:
  - получить `SnapshotView` по borrow;
  - сгруппировать visible cells в render runs;
  - выполнить shaping через GPUI text system;
  - построить hitboxes для hyperlinks и block actions;
  - не выполнять blocking I/O.
- [ ] В paint phase рисовать в стабильном порядке:
  1. default background;
  2. non-default cell background batches;
  3. selection и block highlights;
  4. cursor background;
  5. shaped glyph runs;
  6. hyperlink underline;
  7. block dividers и action controls.
- [ ] Сохранять monospace grid advances независимо от shaping clusters.
- [ ] Использовать snapshot damage и caches только после проверки корректности;
  сначала реализовать правильный full redraw path.
- [ ] Не хранить ссылки на GPUI `Window`, `App` или paint context между frames.
- [ ] Обеспечить корректный scale factor и HiDPI rendering.

### Exit criteria

- Terminal replay fixtures визуально совпадают с Iced reference.
- Нет смещения cursor/selection после resize или scale-factor change.
- Wide/combining Unicode tests проходят.
- `cargo run -p otty-gpui` выдерживает продолжительный большой вывод без
  роста memory queue.

## Этап 4. Terminal input и interaction

Оценка: 4–7 дней.

### Test-first задачи

- [ ] Портировать business-significant tests из `input.rs` и `bindings.rs`.
- [ ] Добавить table-driven tests GPUI keystroke → terminal bytes/action.
- [ ] Проверить Ctrl/Alt/Command/Shift на Linux и macOS.
- [ ] Проверить application cursor/keypad modes.
- [ ] Проверить IME composition contract отдельно от raw terminal keys.
- [ ] Проверить single/double/triple click selection.
- [ ] Проверить drag selection, semantic/line selection и scroll.
- [ ] Проверить SGR/UTF-8/normal mouse reporting.
- [ ] Проверить clipboard copy/paste и bracketed paste, если он активен.
- [ ] Проверить hyperlink hover/click и block selection modes.

### Реализация

- [ ] Использовать `FocusHandle` для каждого terminal entity.
- [ ] Разделить application actions и raw terminal input.
- [ ] Переводить printable input и control keys в существующий escape keyboard
  encoder, не создавать второй encoder внутри GPUI frontend.
- [ ] Реализовать mouse hit testing в координатах terminal grid.
- [ ] Реализовать scroll pixel accumulation и перевод в line deltas.
- [ ] Использовать GPUI clipboard APIs.
- [ ] Использовать platform URL opener из GPUI.
- [ ] Реализовать IME через поддерживаемый GPUI input handler contract.
- [ ] Сохранять terminal focus при clicks, splits, tab activation и закрытии
  overlays.
- [ ] Не отправлять terminal input, если focus находится в form/context menu.

### Exit criteria

- Shell интерактивно пригоден для работы.
- Работают common shortcuts, selection, paste, hyperlinks и mouse apps.
- Проверены `vim`, `less`, `top` или эквивалентные alternate-screen apps.
- Проверена хотя бы одна IME/input method на каждой целевой ОС.

## Этап 5. Tabs и split panes

Оценка: 3–5 дней.

### Решение по pane model

До реализации выбрать один вариант:

1. Небольшой OTTY-specific binary split tree.
2. Проверенный Dock/Tiles из согласованной component library.

Собственная модель должна поддерживать только текущие требования:

- horizontal/vertical split;
- ratio изменения;
- focus traversal;
- close pane и выбор sibling;
- mapping pane → terminal id;
- deterministic serialization не требуется, пока layout не сохраняется.

### Test-first задачи

- [ ] Tests split/resize/close/focus пишутся до pane implementation.
- [ ] Портировать существующие terminal workspace state tests без Iced types.
- [ ] Проверить закрытие последнего pane и tab.
- [ ] Проверить terminal shutdown при закрытии pane.
- [ ] Проверить title активного pane и block selection между panes.

### Реализация

- [ ] Реализовать tab bar и tab content switching.
- [ ] Реализовать horizontal/vertical splits и draggable separators.
- [ ] Хранить каждый terminal как отдельный `Entity<TerminalView>`.
- [ ] Не пересоздавать terminal entities при каждом render.
- [ ] Реализовать context menu pane actions.
- [ ] Восстанавливать focus после split/close/tab activation.
- [ ] Синхронизировать terminal size после изменения sidebar/window/pane ratio.

### Exit criteria

- Все tab/pane lifecycle tests проходят.
- Несколько одновременно выводящих terminals не блокируют UI.
- Нет orphan terminal threads после закрытия panes/tabs.

## Этап 6. Application chrome, sidebar и explorer

Оценка: 4–6 дней.

### Test-first задачи

- [ ] Сохранить reducer/state tests sidebar и explorer.
- [ ] Добавить tests для framework-neutral tree flatten/sort model.
- [ ] Проверить watcher event coalescing и stale root protection.
- [ ] Проверить terminal CWD → explorer root synchronization.
- [ ] Проверить menu guards и overlay dismissal.

### Реализация

- [ ] Перенести custom titlebar/action bar.
- [ ] Реализовать window drag, fullscreen, close и Linux resize grips.
- [ ] Реализовать sidebar show/hide и resize.
- [ ] Реализовать explorer tree, expansion, selection и scrolling.
- [ ] Подключить filesystem watcher через GPUI async/background execution.
- [ ] Реализовать context menus и anchored overlay placement.
- [ ] Перенести icons и theme colors без визуального редизайна.

### Exit criteria

- Explorer синхронизируется с active shell CWD.
- Window controls работают на Linux и macOS.
- Sidebar resize не вызывает terminal geometry drift.
- Watcher не блокирует UI и корректно завершается.

## Этап 7. Quick Launch и settings

Оценка: 5–8 дней.

### Test-first задачи

- [ ] Сохранить существующие Quick Launch reducer/state/storage tests.
- [ ] Сохранить settings normalization/storage tests.
- [ ] Добавить tests focus routing между terminal и form inputs.
- [ ] Проверить autosave/debounce/tick behavior без Iced time subscription.
- [ ] Проверить cancel/kill lifecycle запущенных commands.

### Реализация

- [ ] Перенести Quick Launch tree и folders.
- [ ] Перенести inline create/rename/edit.
- [ ] Перенести context menus, drag/drop и launch indicators.
- [ ] Перенести command/SSH wizard form.
- [ ] Перенести settings sections и form controls.
- [ ] Подключить async storage effects через GPUI executor.
- [ ] Применять theme/font changes ко всем открытым terminal entities.
- [ ] Сохранить совместимость JSON и обработку повреждённых files.

### Exit criteria

- CRUD Quick Launch, custom commands и SSH targets работают.
- Settings сохраняются, загружаются и применяются без перезапуска там, где это
  поддерживается текущим приложением.
- Form focus/IME не передают ввод в terminal.
- Старые пользовательские files открываются без миграции или потери данных.

## Этап 8. Cutover, packaging и стабилизация

Оценка: 4–6 дней плюс время на platform-specific fixes.

### Test-first и regression задачи

- [ ] Выполнить полный feature matrix на Linux X11.
- [ ] Выполнить полный feature matrix на Linux Wayland.
- [ ] Выполнить полный feature matrix на macOS Intel или CI target.
- [ ] Выполнить полный feature matrix на macOS Apple Silicon.
- [ ] Проверить local shell, SSH password/key/passphrase и cancellation.
- [ ] Проверить long-running sessions, high output и multiple panes.
- [ ] Проверить corrupted settings/Quick Launch files.
- [ ] Проверить clean shutdown и отсутствие zombie processes.

### Реализация

- [ ] Переключить основной binary/package output на GPUI frontend.
- [ ] Обновить DEB, RPM, AppImage и DMG scripts/assets.
- [ ] Проверить Linux runtime dependencies на Ubuntu 20.04 baseline.
- [ ] Проверить icons, desktop entry, bundle metadata и code signing flow.
- [ ] Зафиксировать GPUI versions в `Cargo.lock`.
- [ ] Удалить Iced frontend и `otty-ui-term` только после принятия release
  candidate.
- [ ] Удалить Iced dependencies и подтвердить через `cargo tree`.
- [ ] Обновить README и contributor documentation.
- [ ] Добавить migration notes для разработчиков.

### Exit criteria

- GPUI binary является единственным production frontend.
- В workspace runtime graph отсутствуют Iced crates.
- Release artifacts запускаются на заявленных платформах.
- Feature matrix подписан владельцем проекта.
- Rollback tag/branch создан до удаления Iced implementation.

## Матрица функционального parity

### Terminal

- [ ] Local interactive shell.
- [ ] One-shot command tabs.
- [ ] SSH session creation и cancellation.
- [ ] Resize PTY при window/sidebar/pane изменениях.
- [ ] ANSI indexed/RGB colors и attributes.
- [ ] Bold, italic, dim, inverse и underline.
- [ ] Cursor shapes, visibility и blinking behavior.
- [ ] Scrollback и alternate screen.
- [ ] Simple, semantic, line и block selection.
- [ ] Copy selection, block content, prompt и command.
- [ ] Paste и terminal write batching.
- [ ] Hyperlink hover/click.
- [ ] SGR и legacy mouse reporting.
- [ ] Unicode wide characters и combining marks.
- [ ] Font fallback и HiDPI.
- [ ] Shell integration и block metadata.
- [ ] Title changed/reset и child exit.

### Workspace

- [ ] Tab open/activate/close/title update.
- [ ] Horizontal/vertical pane split.
- [ ] Pane resize/focus/close.
- [ ] Pane context menu.
- [ ] Active terminal focus restoration.
- [ ] Explorer synchronization with active shell CWD.

### Application

- [ ] Custom chrome и window controls.
- [ ] Fullscreen.
- [ ] Sidebar collapse/expand/resize.
- [ ] Explorer tree и filesystem watcher.
- [ ] Quick Launch folders/commands/SSH targets.
- [ ] Quick Launch wizard, inline editing и context menus.
- [ ] Settings load/edit/reset/save.
- [ ] Theme и terminal font application.
- [ ] Error tabs и initialization errors.
- [ ] Existing JSON data compatibility.

## Performance validation

Не задавать абсолютные цифры до этапа baseline. Сравнение выполнять на одной
машине и одинаковом terminal replay workload.

Минимальные требования:

- p95 frame time GPUI не хуже Iced baseline;
- p95 input-to-notify/paint latency не хуже Iced baseline;
- idle CPU не выше baseline более чем на согласованный noise margin;
- memory при одном terminal не выше baseline более чем на 15%;
- memory стабилизируется после закрытия tabs/panes;
- output queue не растёт без границ при непрерывном выводе;
- resize coalescing не отправляет PTY resize чаще одного раза за frame;
- shaping cache переиспользуется, когда изменились только position/colors;
- приложение сохраняет отзывчивость при четырёх одновременно активных panes.

Измерять отдельно:

- cold startup;
- first interactive terminal frame;
- idle window;
- `cargo build` с большим объёмом вывода;
- scroll через длинную history;
- resize window с четырьмя panes;
- смену theme/font для нескольких terminals.

## Стратегия тестирования

### Unit tests

- Domain reducers, validation, storage и pane tree.
- Terminal input translation и hit testing.
- Render-run construction, shaping constraints и block geometry.
- Event/effect translation и shutdown state.

### Integration tests

- Local PTY → terminal event → GPUI entity snapshot.
- Terminal input → `TerminalRequest` → fake/runtime session.
- Tab/pane lifecycle с terminal entity release.
- Settings/Quick Launch compatibility fixtures.

Не добавлять тесты для тривиального application bootstrap, logging или platform
wiring, если в них нет бизнес-значимого поведения.

### GPUI tests

Использовать `#[gpui::test]` только для важного focus/input/entity behavior,
которое нельзя надёжно проверить framework-neutral unit test.

### Visual/manual tests

- Reference screenshots для основных экранов.
- Terminal replay fixture comparison.
- Native manual smoke test на каждой target platform/backend.
- IME, clipboard, mouse reporting и custom window controls.

## Обязательные проверки каждого PR

```bash
cargo +nightly fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

Дополнительно после появления GPUI frontend:

```bash
cargo tree -p otty-gpui
cargo run -p otty-gpui
```

На финальном cutover проверить отсутствие Iced:

```bash
cargo tree --workspace -i iced
```

Команда должна завершиться без найденного package после удаления старого
frontend из workspace.

## Риски и меры снижения

### Unicode shaping и terminal grid

Риск: GPUI shaping может выдавать advances, не совпадающие с фиксированной
terminal cell grid.

Меры:

- портировать существующие combining/wide/Hebrew tests до renderer;
- отделить shaping clusters от terminal column accounting;
- сравнивать glyph positions с reference fixtures;
- не оптимизировать damage/caches до корректного full redraw.

### IME и raw terminal input

Риск: composition events конфликтуют с raw keybindings и escape encoding.

Меры:

- отдельный input contract и platform smoke tests;
- не отправлять промежуточный composition text в PTY;
- проверить Linux IME и macOS input method до feature cutover.

### Pane grid

Риск: в GPUI core нет прямого drop-in аналога `iced::pane_grid`.

Меры:

- принять решение после отдельного spike;
- ограничить собственный split tree текущими требованиями;
- покрыть lifecycle и ratio tests до rendering.

### Pre-1.0 GPUI API

Риск: breaking changes и недостаток официальных high-level components.

Меры:

- точный version/commit pin;
- обновлять GPUI отдельными PR;
- не смешивать framework upgrade и feature work;
- не копировать большие внутренние Zed crates.

### Platform packaging

Риск: новые system/runtime dependencies нарушат Ubuntu 20.04 или DMG flow.

Меры:

- собирать package artifacts до удаления Iced;
- smoke-test X11/Wayland/macOS;
- проверить dynamic libraries и minimum OS requirements.

### Параллельные frontends

Риск: временное дублирование views увеличит стоимость изменений.

Меры:

- заморозить новые UI features во время миграции;
- делить business state, но не создавать общий rendering abstraction;
- держать этапы короткими и удалить Iced сразу после принятого RC.

## Rollback plan

- До cutover текущий Iced binary остаётся собираемым и тестируемым.
- Перед удалением Iced создать tag/branch последнего принятого Iced release.
- Не изменять пользовательские storage formats без backward-compatible reader.
- Не удалять packaging scripts старого frontend до успешной проверки GPUI
  artifacts.
- При критической platform regression release переключается на последний Iced
  tag без отката пользовательских данных.

## Definition of Done

- [ ] Все пункты feature parity выполнены.
- [ ] GPUI является единственным production frontend.
- [ ] `otty-libterm`, `otty-surface`, `otty-escape`, `otty-vte` и `otty-pty`
  не зависят от GUI framework.
- [ ] Business state не зависит от Iced или GPUI.
- [ ] Iced отсутствует в workspace dependency graph.
- [ ] Нет `unwrap()` в новом production code.
- [ ] Новые public items документированы.
- [ ] Все новые business-significant изменения реализованы test-first.
- [ ] Форматирование, Clippy, deny, tests и coverage проходят.
- [ ] Line coverage не ниже согласованного baseline и repository threshold.
- [ ] Нет известных regressions в Unicode, IME, selection, mouse reporting,
  blocks, SSH или shell integration.
- [ ] Linux X11, Linux Wayland и macOS artifacts проверены.
- [ ] README, build instructions и packaging metadata обновлены.
- [ ] Владелец проекта принял release candidate по feature matrix.

## Официальные ссылки GPUI

- https://gpui.rs/
- https://github.com/zed-industries/zed/tree/main/crates/gpui
- https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
- https://docs.rs/gpui/0.2.2/gpui/
- https://docs.rs/gpui/0.2.2/gpui/trait.Element.html
- https://docs.rs/gpui/0.2.2/gpui/struct.Context.html
- https://docs.rs/gpui/0.2.2/gpui/struct.ShapedLine.html
- https://github.com/longbridge/gpui-component
