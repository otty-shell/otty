# План создания встраиваемого терминального виджета на GPUI

Дата анализа: 2026-07-26.

Связанный документ: [план миграции всего приложения на GPUI](./gpui.md).
Этот документ намеренно ограничен новым терминальным виджетом и его примерами.

## Итог

Создать `otty-ui/terminal-gpui` с пакетом `otty-ui-term-gpui` возможно без
переписывания VTE, terminal surface, PTY и SSH. Основная часть терминала уже
находится в независимых от GUI crates:

- `otty-vte` и `otty-escape` разбирают управляющие последовательности;
- `otty-surface` хранит grid, scrollback, selection, hyperlinks и blocks;
- `otty-pty` реализует local PTY и SSH;
- `otty-libterm` связывает backend, отдаёт `Arc<SnapshotOwned>` и принимает
  `TerminalRequest`.

Переписывать нужно только frontend-слой `otty-ui/terminal`: lifecycle,
настройки с Iced-типами, input adapter, canvas rendering, text shaping,
clipboard/focus и Iced subscription. Это делает задачу технически
реализуемой с высокой уверенностью.

Рекомендуемый результат — не замена существующего `otty-ui/terminal` на месте,
а второй crate рядом с ним:

```text
otty-ui/terminal/       -> otty-ui-term       (существующий Iced reference)
otty-ui/terminal-gpui/  -> otty-ui-term-gpui  (новый GPUI widget)
```

Iced-версию нужно сохранить собираемой до достижения функционального parity.
Так можно сравнивать поведение и визуальный результат, а также откатиться без
изменения terminal core.

Оценка для одного разработчика с учётом GPUI-native public API и безопасной
runtime-замены backend:

- строгий parity с фактически работающими возможностями Iced-виджета и
  сменяемыми local/SSH/custom backends: 18–29 инженерных дней;
- production-ready вариант с IME, правильными cursor shapes/blink,
  backpressure, Linux/macOS проверками и всеми примерами: 28–43 инженерных
  дня, то есть примерно 6–9 недель;
- ожидание согласования dependency и platform-specific исправления в оценку
  не входят.

## Границы задачи

В scope входят:

- новый library crate `otty-ui-term-gpui`;
- встраивание нескольких независимых terminal entities в любое GPUI view;
- local shell и SSH sessions через `otty-libterm`;
- terminal grid rendering, resize и scrollback;
- ANSI 16/256/truecolor, terminal attributes и cursor;
- keyboard, clipboard, IME, pointer selection и terminal mouse reporting;
- hyperlinks;
- block metadata, selection, copy-команды и external overlay;
- runtime theme/font/binding changes;
- host-facing events;
- аналоги всех примеров из `otty-ui/terminal/examples`;
- README с коротким примером встраивания;
- unit, integration, GPUI и ручные visual tests.

Не входят:

- перенос всего приложения `otty` с Iced на GPUI;
- встроенный tabs/panes manager внутри terminal widget;
- удаление `otty-ui-term` или Iced dependencies;
- изменение VTE semantics ради нового renderer;
- Windows и WebAssembly до отдельного решения о поддерживаемых платформах;
- зависимость от `gpui-component`: для terminal custom element она не нужна;
- общий trait для Iced и GPUI renderer. Он скроет различия event loop и paint
  API, не уменьшив текущую сложность.

`split_view` остаётся примером композиции нескольких виджетов на стороне host,
а не функцией самого terminal crate.

## Что есть в текущем Iced-виджете

### Публичный контракт

`otty-ui-term` экспортирует:

- `Terminal`, `TerminalView` и `Event`;
- local/SSH, font, theme и interaction settings;
- custom keyboard/mouse bindings;
- `BlockCommand` и `BlockUiMode`;
- `snapshot_arc()`, `blocks()`, `block_text()` и `block_prompt_text()`;
- block layout/action-button geometry;
- runtime `change_theme()`, `change_font()` и `add_bindings()`.

### Реально реализованное поведение

- отдельный blocking `mio` runtime thread для local или SSH terminal;
- Iced subscription, проксирующая `TerminalEvent`;
- frame, title/reset title и child-exit events;
- PTY resize по layout bounds и измерению monospace cell;
- ANSI standard/indexed/truecolor backgrounds и foregrounds;
- bold, italic, dim и inverse text;
- wide cells, combining marks, Hebrew niqqud и font shaping tests;
- простая, semantic и line selection по single/double/triple click;
- drag selection;
- SGR, UTF-8 и legacy mouse reports, а также mouse motion;
- alternate-screen scrolling через cursor-key sequences;
- copy/paste, custom bindings и application-cursor mode;
- modifier-click hyperlinks;
- block highlight/dividers, block commands и внешний overlay;
- resize throttling примерно до 30 Hz;
- несколько terminals в `split_view`.

### Важные ограничения текущего reference

Их нельзя случайно превратить в неподтверждённые обещания нового crate:

- `TerminalEvent::Bell`, cursor style/icon и hyperlink events сейчас в основном
  игнорируются frontend-адаптером;
- cursor рисуется прямоугольником независимо от bar/underline shape;
- clipboard paste отправляется как raw bytes без отдельной проверки bracketed
  paste;
- полноценного IME contract в Iced frontend нет;
- underline/strikethrough terminal flags не видны в текущем render path;
- internal block action button state существует, но фактический рабочий
  пример кнопок — `blocks_overlay`;
- README заявляет общий xterm/VT parity, но frontend parity нужно доказывать
  fixture-тестами, а не текстом README.

Для GPUI следует определить два уровня приёмки:

1. `P0 parity` — не потерять реально работающие возможности Iced.
2. `P1 widget quality` — закрыть IME, cursor shapes/blink, bracketed paste,
   backpressure и terminal decorations до production release.

## Почему GPUI подходит

GPUI предоставляет все необходимые primitive APIs:

| Требование | GPUI-механизм | Решение для нового crate |
|---|---|---|
| Встраиваемое состояние | `Entity<T>` и `Render` | `Entity<Terminal>` является дочерним element любого host view |
| Низкоуровневый renderer | `Element::{request_layout, prepaint, paint}` | private `TerminalElement` управляет grid layout и paint order |
| Дешёвые кадры | `PrepaintState`, `ShapedLine`, `paint_quad` | snapshot берётся через `Arc`, layout владеет только данными текущего frame |
| Text shaping | `WindowTextSystem::shape_line` и `TextRun` | группировка cell runs, shaping и paint в фиксированную cell grid |
| Focus | `FocusHandle`, `Focusable`, `track_focus` | каждая terminal entity имеет независимый focus |
| Keyboard/actions | key context, actions, `on_key_down` | shortcuts отделены от raw terminal keystrokes |
| IME | `InputHandler`/`EntityInputHandler`, `Window::handle_input` | marked text хранится во view state, commit уходит в PTY один раз |
| Pointer | hitboxes и mouse/scroll listeners | selection, hover, links и terminal mouse modes |
| Clipboard/URL | `read_from_clipboard`, `write_to_clipboard`, `open_url` | не нужны direct dependencies `open` и clipboard crates |
| Async updates | `Context::spawn`, `WeakEntity`, `cx.notify()` | event receiver обновляет entity без mutable borrow через `await` |
| Host signals | `EventEmitter` и `cx.subscribe`/`subscribe_in` | lifecycle, title, bell, link/context intent и coarse selection events |
| UI tests | `#[gpui::test]` и test context | focus/input/entity behavior проверяется без реального окна |

Дополнительное доказательство — production terminal Zed. Он уже использует
`Entity<TerminalView>`, private `TerminalElement`, hitbox/focus, GPUI text
system, mouse listeners и `InputHandler`. Код Zed нельзя копировать целиком:
у него другая terminal model и application infrastructure, но архитектурный
паттерн непосредственно применим к OTTY.

## Решение по версии GPUI и dependency gate

Перед изменением `Cargo.toml` требуется явное согласование владельца проекта,
поскольку repository policy запрещает устанавливать новые зависимости без
предварительного запроса.

Рекомендуемый первый вариант:

```toml
[dependencies]
gpui = "=0.2.2"
```

Почему именно так:

- `0.2.2` — опубликованная и документированная версия;
- exact pin защищает от pre-1.0 breaking changes;
- опубликованный crate содержит platform application и использует
  `Application::new()` в примерах;
- wildcard запрещён `deny.toml` и неприемлем для воспроизводимой сборки.

Не начинать с `main` branch Zed. На исследованном commit
`30730a305ae235f3be44643d5895e142048ef701` platform bootstrap уже выделен в
отдельный `gpui_platform`, хотя `gpui` всё ещё имеет номер `0.2.2`. Поэтому
пример из `main` нельзя считать API опубликованного `0.2.2` без проверки.

Перед согласованием dependency нужно показать владельцу:

- `gpui = "=0.2.2"`: GPU UI framework, нужен для entity, custom element,
  text shaping, input, focus и paint;
- ожидаемый объём transitive graph через `cargo tree`;
- Linux system requirements для X11 и Wayland;
- macOS требования Xcode/Metal;
- лицензии и Git sources.

Особое внимание `cargo deny`: GPUI 0.2.2 использует pinned Git revisions
`zed-font-kit` и `zed-xim`, тогда как текущий `deny.toml` содержит
`unknown-git = "deny"`. Нельзя молча ослаблять policy. На этапе spike нужно:

1. получить согласование GPUI;
2. сгенерировать lockfile;
3. выполнить `cargo deny check`;
4. либо явно разрешить только конкретные upstream URL/revisions с
   комментарием, либо отказаться от выбранной версии;
5. повторно согласовать изменение `deny.toml`.

Новые direct dependencies `tokio`, `open`, `cosmic-text` и
`gpui-component` для этого crate не планируются:

- `otty-libterm::TerminalEvents::recv_async()` уже работает через `flume`;
- URL открывает GPUI;
- shaping предоставляет GPUI;
- внешний component toolkit не нужен custom terminal element.

Ручные mocks не создавать. Если позже действительно потребуется mock
infrastructure boundary, сначала запросить согласование `mockall`, затем
использовать только его согласно правилам репозитория. Начальный план обходится
реальными channels, snapshot fixtures и local PTY integration tests без mock
trait.

## Целевая архитектура

```text
Host GPUI view
  ├── Entity<otty_ui_term_gpui::Terminal>       public, Render + Focusable
  ├── Subscription                              TerminalEvent callbacks
  └── .child(terminal.clone())                   ordinary child element

Entity<Terminal>
        ├── BackendSlot { generation, session } private replaceable backend
        │     ├── BackendSession                result of one backend start
        │     ├── named blocking runtime thread
        │     ├── TerminalHandle
        │     └── TerminalEvents
        ├── Arc<SnapshotOwned>                   latest immutable frame
        ├── TerminalState                        focus/hover/selection/IME/resize
        ├── TerminalConfig                       frontend-only configuration
        │     ├── TerminalTheme + TerminalFont
        │     ├── TerminalAppearance
        │     ├── TerminalBehavior
        │     └── TerminalBindings
        └── TerminalElement                      private GPUI Element
              ├── request_layout: fill parent
              ├── prepaint: metrics, grid, hitboxes, shaped lines
              └── paint: quads, glyphs, cursor, decorations

TerminalElement / input
  └── TerminalRequest
        └── active BackendSlot
              └── otty_libterm::TerminalEvent
                    └── GPUI task -> generation check -> Entity update
                          ├── cx.notify()         render-state invalidation
                          └── cx.emit(TerminalEvent) semantic host signal
```

### Ownership и threading

- GPUI entity владеет frontend state только на UI thread.
- Последний кадр хранится как `SnapshotArc`; grid не копируется на каждый
  `render()`.
- `SnapshotView<'_>` существует только внутри prepaint/paint preparation и не
  сохраняется между frames.
- Blocking `Runtime::run` остаётся на отдельном именованном OS thread. Его
  нельзя выполнять на GPUI foreground executor.
- `TerminalEvents::recv_async()` читается foreground task, созданной через
  `cx.spawn` или `cx.spawn_in`.
- Task хранит `WeakEntity<Terminal>` и вызывает `update` только после `await`;
  mutable entity borrow через `await` не удерживается.
- Каждый backend получает монотонный `BackendGeneration`. Событие старого
  backend после замены отбрасывается до изменения entity state.
- `on_release` и `Drop` отправляют `TerminalRequest::Shutdown` идемпотентно.
- Ожидание `JoinHandle` не блокирует UI thread. Нормальный shutdown выполняет
  reaper/background path; `Drop` остаётся аварийным non-blocking fallback.
- Несколько terminal entities не разделяют mutable renderer/input state.

`TerminalBackend` — единственная заранее запланированная публичная
инфраструктурная абстракция. Она оправдана уже существующими двумя реальными
реализациями (local PTY и SSH), необходимостью runtime replacement и границей
между UI thread и terminal runtime. Dynamic dispatch происходит только при
старте/замене backend: `Box<dyn TerminalBackend>` потребляется методом
`start`, а active slot хранит готовую `BackendSession`. Dispatch не попадает
в shaping/paint hot path.

### Почему не нужен общий Iced/GPUI frontend core сразу

На первом этапе Iced остаётся read-only behavioral reference. Выделение общего
GUI-neutral frontend crate потребовало бы нового key/modifier/color/geometry
слоя и одновременно изменило бы стабильный Iced frontend. Это увеличивает
риск до получения первого GPUI frame.

Допустима только точечная экстракция после появления двух реальных
реализаций, если код имеет одинаковый business meaning и действительно должен
меняться синхронно. Не создавать заранее renderer trait, widget factory,
manager или универсальный command bus.

## Предлагаемый публичный API

API не копирует Iced `Message`, `Subscription`, `Terminal::handle(Event)` или
обязательный внешний reducer. Его базовые единицы — `Entity<Terminal>`,
`Render`, `Focusable`, GPUI actions, `EventEmitter<TerminalEvent>` и
`Subscription`.

### Принципы контракта

- Host создаёт terminal через `cx.new(...)` и вставляет клонированный
  `Entity<Terminal>` обычным `.child(...)`.
- `Terminal::new` возвращает `Self`, а не создаёт entity внутри себя. Entity
  всегда создаёт владеющий GPUI context.
- Конструктор не принимает `window`: focus handle получается через
  `cx.focus_handle()`, а window-specific работа выполняется во время render и
  input callbacks.
- Конструктор не принимает обязательный `u64 id`. В GPUI entity уже имеет
  `EntityId`, а callback подписки получает emitting entity. Если приложению
  нужен persistent domain ID, оно хранит его рядом с entity.
- Backend запускается асинхронно после создания entity. Ошибка запуска —
  состояние `BackendState::Failed` и сигнал, а не невозможность создать
  widget. Host может сразу показать connecting/error presentation.
- Presentation/interaction config и backend lifecycle разделены. Смена темы
  не пересоздаёт PTY, смена backend не сбрасывает настройки widget.
- Семантические события идут через `cx.emit`; `cx.notify` используется только
  для invalidation/observation и не является публичным signal protocol.
- Нет универсального `execute(Command)` и нет публичного потока внутренних
  `Frame`, `Write`, `Resize`, `Scroll` messages.

### Создание и встраивание

Ожидаемый базовый контракт после compile spike:

```rust,ignore
pub struct Terminal { /* private fields */ }

impl Terminal {
    pub fn new(
        config: TerminalConfig,
        backend: impl TerminalBackend,
        cx: &mut Context<Self>,
    ) -> Self;
}

impl Render for Terminal {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement;
}

impl Focusable for Terminal {
    fn focus_handle(&self, cx: &App) -> FocusHandle;
}

impl EventEmitter<TerminalEvent> for Terminal {}
```

Пример host view:

```rust,ignore
struct WorkspaceView {
    terminal: Entity<Terminal>,
    _terminal_events: Subscription,
    title: SharedString,
}

impl WorkspaceView {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Self, ConfigError> {
        let config = TerminalConfig::builder()
            .font(TerminalFont::monospace(14.0)?)
            .behavior(TerminalBehavior::default())
            .build()?;

        let terminal = cx.new(|cx| {
            Terminal::new(
                config,
                LocalBackend::new(LocalOptions::default()),
                cx,
            )
        });

        let terminal_events = cx.subscribe_in(
            &terminal,
            window,
            |host, terminal, event, window, cx| {
                host.on_terminal_event(&terminal, event, window, cx);
            },
        );

        Ok(Self {
            terminal,
            _terminal_events: terminal_events,
            title: "Terminal".into(),
        })
    }
}

impl Render for WorkspaceView {
    fn render(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().size_full().child(self.terminal.clone())
    }
}
```

`Subscription` хранится в host столько же, сколько нужна подписка. Его drop
отменяет callback. Безусловный `.detach()` в основном примере не использовать:
он допустим только когда lifetime сознательно привязан к обеим entities.

### Конфигурация frontend

Backend не является полем `TerminalConfig`. Конфигурация описывает только то,
что widget может менять без перезапуска session:

```rust,ignore
pub struct TerminalConfig {
    theme: TerminalTheme,
    font: TerminalFont,
    appearance: TerminalAppearance,
    behavior: TerminalBehavior,
    bindings: TerminalBindings,
}
```

Назначение частей:

- `TerminalTheme` — palette, selection, cursor и block colors;
- `TerminalFont` — family/fallbacks, size, weight, line height;
- `TerminalAppearance` — padding, border, radius, focused border;
- `TerminalBehavior` — focus-on-click, copy-on-select, scroll multiplier,
  middle-click paste, link/bell policies, block UI mode и paste policy;
- `TerminalBindings` — terminal-local bindings поверх GPUI keystrokes.

Минимальные behavior policies должны быть значениями, а не callbacks:

```rust,ignore
pub enum LinkPolicy {
    EmitOnly,
    OpenAndEmit,
    Disabled,
}

pub enum BellPolicy {
    SystemAndEmit,
    EmitOnly,
    Disabled,
}

pub enum ContextMenuPolicy {
    Emit,
    Disabled,
}

pub struct TerminalBehavior {
    focus_on_click: bool,
    copy_on_select: bool,
    middle_click_paste: bool,
    scroll_multiplier: f32,
    link_policy: LinkPolicy,
    bell_policy: BellPolicy,
    context_menu_policy: ContextMenuPolicy,
    block_ui_mode: BlockUiMode,
}
```

`scroll_multiplier` валидируется как конечное положительное число. Paste
всегда учитывает active bracketed-paste mode; config не должен разрешать
policy, нарушающую terminal protocol.

У всех structs поля private, есть getters. Невалидные размеры/цвета
отклоняются в `try_new`/builder, поэтому entity всегда хранит валидный config.
Для runtime updates нужны batch и точечные методы:

```rust,ignore
impl Terminal {
    pub fn config(&self) -> &TerminalConfig;

    pub fn set_config(
        &mut self,
        config: TerminalConfig,
        cx: &mut Context<Self>,
    );

    pub fn set_theme(
        &mut self,
        theme: TerminalTheme,
        cx: &mut Context<Self>,
    );

    pub fn set_font(
        &mut self,
        font: TerminalFont,
        cx: &mut Context<Self>,
    );

    pub fn set_appearance(
        &mut self,
        appearance: TerminalAppearance,
        cx: &mut Context<Self>,
    );

    pub fn set_behavior(
        &mut self,
        behavior: TerminalBehavior,
        cx: &mut Context<Self>,
    );

    pub fn set_bindings(
        &mut self,
        bindings: TerminalBindings,
        cx: &mut Context<Self>,
    );
}
```

`set_config` вычисляет diff и делает не больше одного `cx.notify()`: theme и
appearance требуют repaint, font — повторного измерения/shape и resize,
bindings — только замены lookup table, behavior может сбросить несовместимое
hover/drag state. Convenience methods используют тот же private diff path.

Host меняет настройки нативно через entity update:

```rust,ignore
self.terminal.update(cx, |terminal, cx| {
    terminal.set_theme(new_theme, cx);
});
```

Если приложение хранит глобальные настройки в GPUI `Global` или отдельной
settings entity, именно host подписывается/наблюдает за ними и передаёт новый
валидный config в terminal. Виджет не зависит от конкретного app settings
store.

### Контракт сменяемого backend

Backend boundary работает на уровне готового terminal runtime, а не GPUI
элементов. Минимальный object-safe контракт:

```rust,ignore
pub trait TerminalBackend: Send + 'static {
    fn start(
        self: Box<Self>,
        initial_size: TerminalSize,
    ) -> Result<BackendSession, BackendError>;
}

pub struct BackendSession { /* private fields */ }

impl BackendSession {
    pub fn new(
        handle: TerminalHandle,
        events: TerminalEvents,
        run: impl FnOnce() -> Result<(), BackendError> + Send + 'static,
    ) -> Self;
}
```

Связанные lifecycle types остаются небольшими и явными:

```rust,ignore
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendGeneration(u64);

#[derive(Clone, Debug)]
pub enum BackendState {
    Starting,
    Running,
    Stopping,
    Exited(ExitStatus),
    Failed(Arc<BackendError>),
}
```

У `BackendGeneration` нужен accessor, но внутреннее число не выставляется как
public field. Generation идентифицирует конкретный запуск, а не terminal
widget или бизнес-сущность host-приложения.

Планируемые реальные реализации:

- `LocalBackend` строит local PTY через `otty-libterm`;
- `SshBackend` строит SSH session через `otty-libterm`;
- внешний backend может реализовать `TerminalBackend` и вернуть
  `BackendSession`, собранную через generic `otty_libterm::TerminalBuilder`
  со своей `Session + Pollable`, parser или surface.

`BackendError` должен уметь оборачивать `otty_libterm::Error` и внешний
`Error + Send + Sync`, чтобы custom implementation не преобразовывала ошибку
в строку. `BackendSession::new` документирует ownership: handle/events
передаются widget, а one-shot `run` closure выполняется ровно один раз на
именованном background thread.

Таким образом frontend не знает enum `Local/Ssh`, а host не обязан менять тип
`Entity<Terminal>` при смене транспорта. При этом custom backend сохраняет
единый `TerminalRequest`/core `TerminalEvent` contract и не дублирует
эмуляцию терминала.

Runtime replacement:

```rust,ignore
impl Terminal {
    pub fn replace_backend(
        &mut self,
        backend: impl TerminalBackend,
        cx: &mut Context<Self>,
    ) -> BackendGeneration;

    pub fn backend_generation(&self) -> BackendGeneration;
    pub fn backend_state(&self) -> &BackendState;

    pub fn shutdown_backend(&mut self, cx: &mut Context<Self>);
}
```

`replace_backend` имеет одну предсказуемую семантику, без speculative policy
enum:

1. увеличивает generation и переводит widget в `Starting`;
2. прекращает принимать input старым handle и отправляет ему shutdown;
3. очищает backend-scoped state: snapshot, title, selection, hover, IME и
   block selection;
4. сохраняет frontend config, focus handle, bounds и host subscription;
5. запускает новый backend вне UI thread;
6. применяет только события с текущим generation;
7. публикует изменения lifecycle через `TerminalEvent`.

Старый runtime завершается в background/reaper path. Замена никогда не ждёт
`JoinHandle` на UI thread. Если новый backend не стартовал, entity остаётся
живой в `Failed` и допускает следующую замену.

При первом создании до layout backend получает безопасный начальный
`TerminalSize` (обычно 80×24), после первого prepaint — фактический resize.
При replacement используется последний effective size текущего widget, а не
размер всего окна.

### Управляющие операции и GPUI actions

Программное управление выполняется именованными методами entity, а не общим
command enum:

```rust,ignore
impl Terminal {
    pub fn write_text(
        &mut self,
        text: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError>;

    pub fn write_bytes(
        &mut self,
        bytes: &[u8],
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError>;

    pub fn copy_selection(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError>;

    pub fn paste(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError>;

    pub fn clear_selection(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError>;

    pub fn select_block(
        &mut self,
        id: &BlockId,
        cx: &mut Context<Self>,
    ) -> bool;

    pub fn scroll_to_block(
        &mut self,
        id: &BlockId,
        cx: &mut Context<Self>,
    ) -> bool;

    pub fn copy_block(
        &mut self,
        id: &BlockId,
        part: BlockTextPart,
        cx: &mut Context<Self>,
    ) -> Result<(), OperationError>;
}
```

Методы, которые могут встретить bounded-channel backpressure, не теряют
данные: они помещают запрос в ограниченную frontend queue и инициируют flush.
Immediate validation/queue failures возвращаются как typed `OperationError`;
ошибка уже запущенного runtime дополнительно отражается в `BackendState` и
emitted lifecycle signal. Silent drop запрещён.

Для keyboard menus и host keymaps crate объявляет typed GPUI actions минимум
для `Copy`, `Paste`, `SelectAll`, `ClearSelection`, `ScrollPageUp`,
`ScrollPageDown`, `ScrollToTop` и `ScrollToBottom`. `Terminal::render`
регистрирует их через `.on_action(cx.listener(...))`. Host может использовать
обычный GPUI `KeyBinding`/`dispatch_action`, не вызывая специальный terminal
dispatcher. Динамические block operations остаются методами, потому что несут
`BlockId`.

```rust,ignore
actions!(
    otty_terminal,
    [
        Copy,
        Paste,
        SelectAll,
        ClearSelection,
        ScrollPageUp,
        ScrollPageDown,
        ScrollToTop,
        ScrollToBottom,
    ]
);
```

Namespace принадлежит crate, чтобы host keymap не конфликтовал с глобальными
actions другого editor/terminal widget.

### Сигналы от widget

В GPUI нативный сигнал — тип, для которого view реализует
`EventEmitter<TerminalEvent>`. Entity вызывает `cx.emit(event)`, а host
подписывается через `cx.subscribe` или `cx.subscribe_in`:

```rust,ignore
pub enum TerminalEvent {
    BackendStateChanged {
        generation: BackendGeneration,
        state: BackendState,
    },
    TitleChanged(Option<SharedString>),
    Bell,
    OpenLinkRequested {
        uri: SharedString,
    },
    SelectionChanged {
        has_selection: bool,
    },
    BlockSelectionChanged {
        block_id: Option<BlockId>,
    },
    Copied {
        source: CopySource,
    },
    ContextMenuRequested {
        position: Point<Pixels>,
        target: HitTarget,
    },
}
```

Точный список корректируется parity tests, но разделение фиксированное:

- lifecycle: starting/running/stopping/exited/failed;
- metadata: title reset выражается как `TitleChanged(None)`;
- user intent: bell, open-link и context-menu request;
- coarse interaction: selection/block selection/copy completion.

`Frame`, cursor blink tick, resize, every keystroke, mouse move и scroll delta
наружу не emit-ятся. Иначе host окажется частью внутреннего reducer и API снова
станет Iced-подобным. Focus event тоже не дублируется: host использует
`Focusable`, `FocusHandle` и стандартные `cx.on_focus`/`cx.on_blur`.

Политики определяют, что widget делает сам:

- link policy: открыть через `cx.open_url`, только emit-ить request или сделать
  оба действия;
- bell policy: system bell, только emit или ignore;
- context-menu policy: emit host request либо выключить;
- copy policy: native GPUI clipboard с последующим `Copied` signal.

По умолчанию link/context menu лучше передавать host через event: встраиваемый
widget не должен решать routing приложения. Clipboard и обычные terminal
keybindings остаются внутри widget как platform services GPUI.

### Чтение состояния

Host читает дешёвое текущее состояние через `Entity::read(cx)`, без request /
response messages:

```rust,ignore
impl Terminal {
    pub fn config(&self) -> &TerminalConfig;
    pub fn backend_state(&self) -> &BackendState;
    pub fn title(&self) -> Option<&str>;
    pub fn snapshot_arc(&self) -> SnapshotArc;
    pub fn blocks(&self) -> &[BlockSnapshot];
    pub fn block_text(&self, id: &BlockId) -> Option<String>;
    pub fn has_selection(&self) -> bool;
}
```

Large frame/block data не включаются в каждый emitted event. Signal сообщает
о semantic change, после чего заинтересованный host при необходимости читает
актуальное состояние. Возвращаемые borrowed views не должны переживать GPUI
update; для передачи между tasks используется `SnapshotArc`.

Все новые public items имеют короткие doc comments и компилируемые examples,
где это практично. Public fields не используются. Имена и точные GPUI
signatures подтверждаются compile spike на закреплённой версии GPUI.

### Чего в public API быть не должно

- типов `iced::*` и конвертеров, делающих Iced основным контрактом;
- `Message`, `Subscription`, `Command` или `handle(Event)` в смысле Iced;
- публичных `Redraw`, `ContentSync`, `Write`, `Resize`, `Scroll` событий;
- обязательного caller-provided widget ID;
- callback fields внутри `TerminalConfig` вместо GPUI `EventEmitter`;
- generic `Terminal<B>`: он мешает заменить backend без замены типа entity;
- глобального singleton backend/settings store;
- renderer/backend manager/factory поверх уже достаточного
  `TerminalBackend`.

## Вид и поведение именно как widget

`Terminal::render` должен возвращать самостоятельный framed element, который:

- занимает размер, выделенный parent, а не управляет окном;
- не закрывает окно при child exit;
- имеет собственный stable element id и focus handle;
- рисует terminal background внутри своих bounds;
- clip-ит grid, selection, cursor и overlay по rounded bounds;
- поддерживает настраиваемые padding, border, corner radius и focused border;
- фокусируется по primary click;
- меняет cursor style на I-beam, pointer над link/action и terminal-requested
  icon там, где это поддержано;
- корректно работает рядом с button, form, menu и другим terminal;
- не перехватывает keyboard input, когда focus находится у соседнего widget;
- не требует от host отдельного frame subscription или redraw forwarding.

Default appearance должен быть спокойным и пригодным для embedding:

- 8–10 px inner padding;
- 8 px corner radius;
- 1 px neutral border;
- более заметный border только в focused state;
- terminal palette управляет внутренним background, а не theme всего окна.

Это оформляется `TerminalAppearance` внутри `TerminalConfig`, без
универсальной style abstraction.

## Предлагаемая структура файлов

```text
otty-ui/terminal-gpui/
├── Cargo.toml
├── README.md
├── examples/
│   ├── backend_switching.rs
│   ├── bindings.rs
│   ├── blocks_overlay.rs
│   ├── fonts.rs
│   ├── full_screen.rs
│   ├── split_view.rs
│   └── themes.rs
└── src/
    ├── lib.rs
    ├── actions.rs
    ├── appearance.rs
    ├── backend/
    │   ├── local.rs
    │   ├── mod.rs
    │   ├── slot.rs
    │   └── ssh.rs
    ├── bindings.rs
    ├── behavior.rs
    ├── block_controls.rs
    ├── block_layout.rs
    ├── config.rs
    ├── error.rs
    ├── event.rs
    ├── font.rs
    ├── input.rs
    ├── render_runs.rs
    ├── terminal.rs
    ├── terminal_element.rs
    ├── text_layout.rs
    └── theme.rs
```

Ответственность файлов:

- `backend/mod.rs` — public `TerminalBackend`, `BackendSession`, generation и
  state types;
- `backend/local.rs` и `backend/ssh.rs` — две реальные built-in реализации;
- `backend/slot.rs` — private start/replace/shutdown и stale-generation guard;
- `terminal.rs` — public entity state и методы без backend construction;
- `event.rs` — только host-facing `TerminalEvent` и payload types;
- `actions.rs` — public GPUI actions и их namespace;
- `config.rs` — validated aggregate `TerminalConfig` и diff;
- `appearance.rs`, `behavior.rs`, `font.rs`, `theme.rs` — отдельные части
  frontend config;
- `terminal_element.rs` — GPUI layout/prepaint/paint и registration listeners;
- `input.rs` — pointer/keyboard/IME state transitions в конкретные requests;
- `bindings.rs` — binding types, lookup и default maps;
- `render_runs.rs` — cell-to-run grouping без GPUI paint calls;
- `text_layout.rs` — преобразование runs в `TextRun`/`ShapedLine`;
- `theme.rs` — palette validation и ANSI-to-GPUI color mapping;
- `block_layout.rs` и `block_controls.rs` — GPUI geometry blocks/overlay;
- `error.rs` — явные construction/runtime/config errors.

Если `terminal_element.rs` начнёт совмещать независимые input registration и
painting responsibilities, listeners нужно вынести в `input.rs`, а не
оставлять один трудно читаемый файл. Тесно связанный layout state не дробить
только ради количества строк.

## Потоки событий

### Backend frame

```text
PTY readable
  -> otty-libterm parses output
  -> core TerminalEvent::Frame { Arc<SnapshotOwned> }
  -> GPUI event task awaits TerminalEvents::recv_async()
  -> WeakEntity<Terminal>::update with BackendGeneration
  -> discard if generation is stale
  -> replace latest Arc
  -> cx.notify()
  -> next prepaint borrows SnapshotView
```

### Host signal

```text
core event or user intent
  -> update Entity<Terminal> state
  -> cx.emit(TerminalEvent)
  -> host cx.subscribe/cx.subscribe_in callback
  -> optional Entity::read for current large state
```

`cx.notify()` и `cx.emit(...)` выполняют разные задачи и не заменяют друг
друга: первое инвалидирует/оповещает observers, второе доставляет typed
semantic event подписчикам.

### Keyboard/IME

```text
focused Terminal
  -> GPUI action?                yes -> typed widget operation
  -> terminal binding?           yes -> encoded control/escape bytes
  -> IME marked text?            yes -> local visual composition only
  -> IME committed/printable     yes -> UTF-8 bytes exactly once
  -> TerminalRequest::WriteBytes
```

Нужно отдельно тестировать отсутствие двойной отправки printable text через
`on_key_down` и `InputHandler`.

### Pointer

```text
mouse position
  -> widget bounds / hitbox
  -> local pixel position
  -> grid point using cell metrics + display_offset
  -> terminal mouse mode?
       yes -> encode SGR/UTF-8/legacy report
       no  -> link/block hit-test or selection start/update
```

## Матрица функционального parity

| Возможность | Цель | Проверка |
|---|---|---|
| Native `Entity<Terminal>` embedding | P0 | `.child(entity)` без window ownership |
| `EventEmitter<TerminalEvent>` signals | P0 | `subscribe_in` получает typed lifecycle/user events |
| Runtime config replacement | P0 | theme/font/appearance/behavior/bindings diff tests |
| Local interactive shell | P0 | запуск default shell, ввод, exit |
| SSH password/key/passphrase/cancel | P0 | существующие builders + manual target |
| Custom `TerminalBackend` | P0 | test backend через `otty-libterm::TerminalBuilder` |
| Backend replacement | P0 release blocker | stale events отброшены, UI thread не заблокирован |
| Backend failure/retry | P0 | `Failed` entity принимает следующую замену |
| Frame/title/reset/exit | P0 | event reducer tests и example title |
| Bell/runtime error | P1 | GPUI event/system bell smoke test |
| ANSI 16/256/truecolor | P0 | palette unit tests + replay screenshot |
| Bold/italic/dim/inverse | P0 | render-run tests |
| Underline/strikethrough | P1 | style flag fixtures |
| Wide/combining/emoji/fallback | P0 | shape/grid tests |
| Hebrew niqqud и complex scripts | P0 | перенесённые regression fixtures |
| Cursor visible/hidden | P0 | layout tests |
| Block/bar/underline cursor | P1 | visual + geometry tests |
| Cursor blink/terminal control | P1 | deterministic timer/state tests |
| Resize/HiDPI | P0 | rows/cols tests и native resize smoke |
| Resize coalescing | P0 | не больше одного effective resize/frame |
| Scrollback | P0 | line/pixel scroll tests |
| Alternate scroll | P0 | cursor sequence tests |
| Simple/semantic/line selection | P0 | table-driven input tests |
| Block selection type | P1 | modifier/command test |
| SGR/UTF-8/legacy mouse | P0 | byte-exact tests |
| Mouse motion | P0 | drag/motion tests |
| Copy/paste | P0 | GPUI clipboard tests |
| Bracketed paste | P1 | mode-aware byte fixture |
| IME composition | P1 release blocker | marked/commit/cancel tests + OS smoke |
| Hyperlink hover/open | P0 | hit-test и `open_url` callback test |
| Custom bindings | P0 | replace/include/exclude mode tests |
| Typed GPUI actions | P0 | key bindings/dispatch вызывают те же operations |
| Runtime font/theme changes | P0 | entity update + reshape/repaint tests |
| Blocks metadata/text | P0 | snapshot tests |
| Block select/copy/scroll | P0 | command reducer tests |
| External block overlay | P0 | `blocks_overlay` example |
| Multiple instances/focus | P0 | `split_view` + GPUI focus test |
| Clean shutdown | P0 release blocker | release/drop integration test |
| Bounded memory under output | P1 release blocker | stress/replay benchmark |

## Пошаговый план реализации

Каждый этап выполняется test-first:

1. Сначала failing test или characterization fixture.
2. Затем минимальная реализация.
3. Затем targeted tests и обязательные workspace checks.
4. Только после exit criteria начинается следующий этап.

### Этап 0. Dependency и baseline spike

Оценка: 1–2 дня после согласования dependency.

#### До изменения зависимостей

- [ ] Запросить разрешение на `gpui = "=0.2.2"` с описанием из dependency
  section выше.
- [ ] Отдельно согласовать возможные pinned Git sources в `deny.toml`.
- [ ] Подтвердить target platforms: Linux X11, Linux Wayland и macOS.
- [ ] Подтвердить, что Windows/WASM не являются release blockers.
- [ ] Решить, поддерживаются ли Iced и GPUI frontend долго или Iced только
  временный reference. Не создавать общий frontend abstraction до решения.

#### Baseline

- [ ] Запустить test/lint/coverage baseline до добавления GPUI.
- [ ] Зафиксировать уже существующие failures отдельно от новой работы.
- [ ] Сохранить reference screenshots Iced examples.
- [ ] Подготовить deterministic terminal replay fixtures: colors, styles,
  Unicode, links, selection, alt screen, mouse modes и blocks.
- [ ] Измерить Iced baseline: first frame, idle CPU/RSS, resize frame time,
  большой output и четыре terminals.

#### Compile spike

- [ ] Добавить минимальный временный branch/commit с exact GPUI dependency.
- [ ] Проверить `cargo check` на Linux и macOS target/CI.
- [ ] Проверить `cargo deny check` и transitive Git sources.
- [ ] Проверить простой `Entity<ProbeView>` и custom `Element`.
- [ ] Подтвердить `cx.new`, `Entity::update`, `Entity::read`, `Focusable` и
  `.child(Entity<ProbeView>)` на опубликованной версии.
- [ ] Подтвердить `EventEmitter`, `cx.emit`, `subscribe_in` и отмену callback
  при drop сохранённого `Subscription`.
- [ ] Подтвердить typed GPUI action, `.on_action(cx.listener(...))` и host
  `dispatch_action`.
- [ ] Проверить `WindowTextSystem::shape_line` с `force_width` на ASCII,
  wide char, combining mark и fallback font.
- [ ] Проверить `InputHandler` commit/marked text API.
- [ ] Проверить, что две entities независимо получают focus и events.
- [ ] Проверить background start, foreground `WeakEntity::update` и release
  task без удержания entity через сильную ссылку.
- [ ] Удалить spike-код либо превратить его в первый test-first commit.

#### Exit criteria

- Dependency и необходимые source exceptions согласованы.
- GPUI компилируется с workspace toolchain.
- Известен точный application bootstrap опубликованной версии.
- Text grid и IME APIs признаны достаточными либо зафиксирован blocker.

### Этап 1. Native API, config, replaceable backend и entity lifecycle

Оценка: 5–8 дней.

#### Тесты до реализации

- [ ] Tests на validation font size, line height, padding, behavior values и
  palette colors.
- [ ] Tests на `TerminalConfig` diff: theme/appearance repaint, font reflow,
  bindings lookup replacement и один aggregate invalidation.
- [ ] Tests на local/SSH backend getters/builders без запуска UI.
- [ ] Tests на `TerminalBackend` → `BackendSession` boundary через test
  adapter, построенный на реальных core channels/fixtures, без ручного mock.
- [ ] Tests lifecycle state machine:
  `Starting -> Running -> Stopping -> Exited` и `Starting -> Failed`.
- [ ] Tests на mapping core `TerminalEvent` в internal state и emitted
  `TerminalEvent` frontend crate.
- [ ] GPUI test: сохранённый `Subscription` получает title/bell/lifecycle, а
  после drop callback больше не вызывается.
- [ ] Tests: новый frame заменяет `Arc`; large frame не копируется в host
  event.
- [ ] Tests: `replace_backend` увеличивает generation, отбрасывает поздний
  frame/exit старого backend и принимает frame нового.
- [ ] Tests: replacement очищает backend-scoped state, но сохраняет config и
  `FocusHandle`.
- [ ] Tests: failed backend можно заменить рабочим без пересоздания entity.
- [ ] Tests: повторный `shutdown_backend()` безопасен.
- [ ] Integration test: local shell создаётся, выдаёт frame и завершается.
- [ ] Integration test: release entity отправляет shutdown и runtime выходит.

#### Реализация

- [ ] Создать `otty-ui/terminal-gpui/Cargo.toml` с package
  `otty-ui-term-gpui` и workspace edition/rust-version.
- [ ] Добавить минимальные dependencies: согласованный GPUI,
  `otty-libterm`, workspace `thiserror` и `log`.
- [ ] Реализовать validated `TerminalConfig`, отдельные theme/font/
  appearance/behavior/bindings types и private diff.
- [ ] Реализовать object-safe `TerminalBackend`, `BackendSession`,
  `BackendGeneration`, `BackendState` и typed errors.
- [ ] Реализовать `LocalBackend` и `SshBackend` поверх `otty-libterm`.
- [ ] Реализовать private `BackendSlot` для start/replace/shutdown и
  generation guard; не добавлять второй manager/factory layer.
- [ ] Вызывать `TerminalBackend::start` на background executor, не на UI
  thread.
- [ ] Запускать blocking runtime на именованном thread.
- [ ] Хранить request proxy/handle и `TerminalEvents` без Tokio proxy channel.
- [ ] Реализовать `Terminal::new(config, backend, cx) -> Self`, `Render`,
  `Focusable` и `EventEmitter<TerminalEvent>`.
- [ ] Реализовать event task через `WeakEntity` и `recv_async()`.
- [ ] Перед применением каждого core event сравнивать backend generation.
- [ ] На `Frame` менять только latest `SnapshotArc` и вызывать `cx.notify()`;
  frame не emit-ить host.
- [ ] На title/exit/bell/error и coarse user intent вызывать `cx.emit(...)`.
- [ ] Реализовать config getters, `set_config` и точечные `set_*` через один
  diff path.
- [ ] Реализовать `replace_backend` и `shutdown_backend` без ожидания thread
  join на UI thread.
- [ ] Реализовать idempotent release hook и non-blocking drop fallback.
- [ ] Construction/runtime/config errors возвращать typed, без `unwrap()`.
- [ ] Добавить doc comments новым public items.

#### Exit criteria

- Crate собирается без Iced, Tokio и `open`.
- Host создаёт widget через `cx.new`, вставляет `.child(entity)` и меняет
  config через `Entity::update`.
- Entity получает frames и host получает только typed semantic events.
- Local, SSH и custom backend используют один public boundary.
- Backend можно заменить без пересоздания entity; stale events доказанно
  игнорируются.
- Drop/release не оставляет orphan runtime в integration test.
- Existing `otty-ui-term` не изменён и продолжает собираться.

### Этап 2. Theme, font metrics, bindings и render model

Оценка: 3–4 дня.

#### Тесты до реализации

- [ ] Перенести palette tests для standard, indexed 16–255 и truecolor.
- [ ] Invalid hex должен возвращать typed error, а не panic.
- [ ] Перенести bindings tests: add, replace, include/exclude `SurfaceMode`.
- [ ] Table-driven tests GPUI keystroke/modifiers → binding input.
- [ ] Перенести render-run tests для spaces, colors, bold/italic/dim/inverse.
- [ ] Перенести wide spacer, combining, Thai/Lao/Arabic/Hebrew fixtures.
- [ ] Tests на stable byte ranges нескольких `TextRun` color/style spans.
- [ ] Tests на block rectangles и action-button geometry в GPUI coordinates.

#### Реализация

- [ ] Сохранить `ColorPalette` по смыслу, но преобразовывать в `gpui::Hsla`
  один раз при construction/theme change.
- [ ] Не парсить hex и не строить 256-color map в каждом frame.
- [ ] Заменить `iced::Font` на GPUI font family/weight/style/fallback settings.
- [ ] Измерять cell width через GPUI text system и glyph `m`.
- [ ] Хранить font size и line-height отдельно от platform scale factor.
- [ ] Реализовать GPUI-specific `InputKind` и `Modifiers` adapter.
- [ ] Перенести default bindings без второго escape encoder.
- [ ] Создать shape-ready runs из `SnapshotCell`, сохраняя grid columns.
- [ ] Добавить underline/strikethrough flags как P1 style spans.
- [ ] Перенести block geometry на `Bounds<Pixels>`/`Point<Pixels>`.

#### Exit criteria

- Business-significant helpers тестируются без real window.
- Все current Unicode/render-run regressions перенесены.
- Theme/font/bindings не зависят от Iced.
- Нет parsing panic и unnecessary per-frame allocation очевидного уровня.

### Этап 3. Custom `TerminalElement` и первый интерактивный frame

Оценка: 4–7 дней.

#### Тесты до реализации

- [ ] Tests bounds → rows/columns/cell size, включая слишком малый widget.
- [ ] Tests device-pixel snapping на scale factors 1.0, 1.25, 1.5 и 2.0.
- [ ] Tests paint-plan order без real GPU.
- [ ] Tests cursor geometry для hidden/block/bar/underline.
- [ ] Tests selection/background batches и clipping.
- [ ] Tests shaping result занимает ожидаемое число grid columns.
- [ ] Tests theme/font changes invalidate нужный layout state.

#### Реализация

- [ ] `Terminal::render` создаёт outer framed `div` и private
  `TerminalElement`.
- [ ] `request_layout` запрашивает доступный parent size, не размер окна.
- [ ] `prepaint` создаёт hitbox и вычисляет inner bounds после padding/border.
- [ ] Из font metrics и bounds вычислять `TerminalSize`.
- [ ] Посылать resize только при изменении effective rows/cols/cell pixels.
- [ ] Coalesce resize до одного effective request за frame; не переносить
  Iced timer механически, если GPUI frame boundary уже решает задачу.
- [ ] Borrow latest snapshot и построить owned `PrepaintState`:
  backgrounds, selections, block visuals, cursor, shaped lines и hit targets.
- [ ] Использовать GPUI `TextRun` для color/style spans.
- [ ] Проверить semantics `shape_line(..., force_width)` и не предполагать,
  что он идентичен Cosmic Text `set_monospace_width`.
- [ ] Для complex cluster сохранять terminal cell count отдельно от UTF-8
  bytes/glyph count.
- [ ] Paint order:
  1. widget и terminal background;
  2. non-default cell backgrounds;
  3. block/selection highlights;
  4. cursor background/shape;
  5. shaped glyphs;
  6. underline/strikethrough/hyperlink decorations;
  7. block dividers/actions и focus border.
- [ ] Использовать content mask/clip для всех terminal pixels.
- [ ] Первую корректную версию делать full redraw; damage/caches добавлять
  только после profiling.

#### Exit criteria

- `full_screen` показывает интерактивный local shell.
- Colors и Unicode fixtures визуально соответствуют Iced reference.
- Resize не смещает text/cursor/selection.
- Виджет корректно занимает вложенные bounds и не управляет window.

### Этап 4. Keyboard, clipboard, IME и pointer interaction

Оценка: 4–6 дней.

#### Тесты до реализации

- [ ] Портировать input tests single/double/triple click и drag selection.
- [ ] Byte-exact tests всех default control/function/navigation keys.
- [ ] Tests application cursor mode и modifier combinations Linux/macOS.
- [ ] Tests copy и paste через GPUI test clipboard.
- [ ] Tests typed GPUI actions и прямые entity methods приводят к одному
  operation path.
- [ ] Tests bracketed paste open/close sequences по active mode.
- [ ] Tests marked text: update, replace, cancel и commit ровно один раз.
- [ ] Tests IME candidate bounds следуют terminal cursor.
- [ ] Tests line/pixel scroll accumulation.
- [ ] Tests SGR, UTF-8, normal mouse press/release/motion/wheel bytes.
- [ ] Tests modifier-click link и cursor style.
- [ ] Tests link policies: emit-only, open-and-emit и disabled.
- [ ] Tests bell/context-menu policies и соответствующие emitted events.
- [ ] Tests соседний focused control не передаёт input terminal.

#### Реализация

- [ ] Отслеживать focus через `FocusHandle`, а не boolean-only state.
- [ ] Объявить typed GPUI actions и зарегистрировать их через
  `.on_action(cx.listener(...))`; не добавлять terminal command dispatcher.
- [ ] Направить actions и public methods в одни private operations.
- [ ] Raw key events переводить через существующую binding table.
- [ ] Printable/IME commit кодировать UTF-8 один раз.
- [ ] Реализовать `InputHandler` и marked-text visual state.
- [ ] Рисовать marked text у cursor и возвращать корректные candidate bounds.
- [ ] Clipboard использовать только через GPUI App APIs.
- [ ] Link/bell/context-menu поведение выполнять по `TerminalBehavior`:
  `cx.open_url`, `cx.emit`, оба действия или ignore согласно конкретной
  policy.
- [ ] Перенести point → grid mapping с display offset и wide cell handling.
- [ ] В terminal mouse mode не начинать local selection без нужного override.
- [ ] Поддержать hover links/blocks без `cx.notify()` на неизменившемся state.
- [ ] После обычного input прокручивать к bottom в соответствии с текущим
  поведением, кроме режимов, где это нарушает terminal semantics.

#### Exit criteria

- Shell пригоден для интерактивной работы.
- Проверены `vim`, `less`, `top` или аналоги в alternate screen.
- Copy/paste, selection, links и mouse applications работают.
- IME smoke test выполнен на Linux и macOS.

### Этап 5. Blocks, внешний overlay и widget polish

Оценка: 3–4 дня.

#### Тесты до реализации

- [ ] Перенести tests `snapshot_arc`, blocks и block text/prompt text.
- [ ] Tests select/clear/copy/copy-content/copy-prompt/copy-command.
- [ ] Tests `ScrollTo` для block выше и ниже viewport.
- [ ] Tests `PrimaryClick` и `CommandOnly`.
- [ ] Tests alt screen запрещает block UI commands.
- [ ] Tests external overlay не дублирует internal highlight/actions.
- [ ] Tests focus border и clip geometry.

#### Реализация

- [ ] Сохранить `BlockUiMode` по смыслу, но не переносить общий
  `BlockCommand` dispatcher.
- [ ] Выполнять select/clear/copy/scroll через именованные entity methods и
  `BlockTextPart` только там, где payload действительно динамический.
- [ ] Эмитить `BlockSelectionChanged` и `Copied` host-у.
- [ ] Internal mode рисует highlight/dividers и согласованный action chrome.
- [ ] External mode публикует snapshot и geometry helpers для host overlay.
- [ ] Реализовать stacked absolute overlay в GPUI example.
- [ ] Добавить focused/hovered visual states без влияния на cell geometry.
- [ ] Добавить минимальную accessibility semantics после проверки доступных
  AccessKit roles GPUI 0.2.2.

#### Exit criteria

- `blocks_overlay` повторяет текущий сценарий selection/copy/log.
- Overlay остаётся синхронизирован с scroll и resize.
- Terminal выглядит как самостоятельный widget в любом parent container.

### Этап 6. Полный набор примеров и документация

Оценка: 2–3 дня.

Создать те же файлы и сценарии:

- [ ] `backend_switching.rs` — дополнительный пример смены backend в той же
  entity, lifecycle signals и retry после controlled failure;
- [ ] `full_screen.rs` — минимальный local shell, title и exit event;
- [ ] `split_view.rs` — несколько independent entities, split/close/focus;
- [ ] `themes.rs` — runtime palette switching;
- [ ] `fonts.rs` — embedded fonts, family и size switching;
- [ ] `bindings.rs` — custom binding replace/include/exclude;
- [ ] `blocks_overlay.rs` — external overlay и block command;
- [ ] `README.md` — install, `cx.new`/`.child`, config updates, backend
  replacement, `EventEmitter` subscriptions, actions, examples, platforms и
  limitations;
- [ ] rustdoc example — создание entity и embedding без владения window;
- [ ] сравнить список примеров автоматически через review/checklist.

Требования к примерам:

- каждый example сам владеет только host window и layout;
- terminal widget не вызывает app quit сам;
- child exit закрывает только выбранный terminal, если host так решил;
- split example не хранит ephemeral element вместо `Entity<Terminal>`;
- host хранит `Subscription` и не использует detached callback без объяснения
  lifetime;
- runtime theme/font/backend changes выполняются через `Entity::update`;
- никакого `unwrap()` в library production code; в example initialization
  разрешён только `expect()` с контекстом согласно правилам проекта.

#### Exit criteria

- Все шесть parity examples и дополнительный `backend_switching` собираются
  `--all-targets`.
- README quick start соответствует реальному API.
- Два terminals одновременно принимают output, focus переключается корректно.

### Этап 7. Backpressure, performance и platform stabilization

Оценка: 4–6 дней плюс platform-specific fixes.

#### Backpressure

Сейчас default event/request channels `otty-libterm` unbounded. Простое
перекладывание каждого frame в GPUI entity может дать растущую очередь при
очень быстром output. Установка bounded channel без изменений core тоже
опасна: текущий `flush_event_queue()` возвращает `EventChannelFull`.

Поэтому перед release:

- [ ] Написать failing stress test: producer frames быстрее UI consumer.
- [ ] В `otty-libterm` добавить конкретную coalescing policy для replaceable
  `Frame`, не теряя `ChildExit`, title, bell и error events.
- [ ] При full channel не завершать runtime только из-за промежуточного frame.
- [ ] Сохранять последний frame и порядок non-frame events.
- [ ] Не добавлять общий event-bus abstraction.
- [ ] Проверить bounded memory и eventual latest-frame delivery.

Это business-significant изменение core, поэтому тесты обязательны до
реализации.

#### Performance

- [ ] Сравнить first frame, idle CPU/RSS и input latency с Iced baseline.
- [ ] Replay большого `cargo build` output.
- [ ] Scroll длинной history.
- [ ] Continuous resize одного и четырёх terminals.
- [ ] Theme/font change нескольких terminals.
- [ ] Проверить allocations в render-run building и shaping.
- [ ] Использовать `SnapshotDamage` только если profile показывает выгоду.
- [ ] Добавить shape cache только с ключом content/font/cell metrics; position
  не должна сама инвалидировать shaping.
- [ ] Проверить, что closed terminals освобождают snapshots/tasks/threads.

#### Platform matrix

- [ ] Linux X11: focus, clipboard, IME, mouse, URL, fonts, resize.
- [ ] Linux Wayland: те же сценарии.
- [ ] macOS Intel или CI target: build и basic smoke.
- [ ] macOS Apple Silicon: full smoke, Metal, IME и Command shortcuts.
- [ ] SSH timeout/cancel на поддерживаемой платформе.

#### Exit criteria

- Memory queue не растёт без границ.
- Нет orphan threads/zombie child processes.
- p95 frame/input показатели не хуже согласованного baseline margin.
- Все target platform smoke tests подписаны.

## Стратегия тестирования

### Unit tests

Тестировать только business-significant behavior:

- bindings и input encoding;
- `TerminalConfig` validation/diff и behavior policies;
- palette resolution и validation;
- point/grid, resize и block geometry;
- render-run grouping и style resolution;
- Unicode/grid constraints;
- block commands;
- backend generation/replacement, event reducer, lifecycle transitions и
  shutdown idempotency;
- frontend semantic events отдельно от `cx.notify` invalidation;
- backpressure/coalescing policy.

Не добавлять tests для тривиального GPUI application bootstrap, logging или
Cargo wiring.

### Integration tests

- local PTY → event → entity snapshot;
- entity input → `TerminalRequest` → shell output;
- entity release → runtime shutdown;
- backend replacement → stale old events ignored → new frame accepted;
- failed backend → same entity retries successfully;
- host subscription receives typed events and stops after drop;
- two-terminal focus/lifecycle;
- existing replay fixtures через новый render plan.

SSH credentials не должны попадать в repository fixtures. SSH auth/cancel
проверять либо fake local SSH endpoint существующими test tools, либо ручным
secure smoke test без сохранения secrets.

### GPUI tests

Использовать `#[gpui::test]` только для поведения, зависящего от framework:

- focus routing;
- clipboard;
- key/action dispatch;
- IME handler contract;
- entity event emission/release;
- two widgets in one window.

### Visual/manual tests

- reference screenshots всех examples;
- ANSI/Unicode replay comparison;
- cursor shape/blink;
- selection/hyperlink/block hover;
- native font fallback и HiDPI;
- `vim`, `less`, `top`, mouse-enabled TUI;
- IME на Linux и macOS.

Pixel-perfect snapshot не должен быть единственным тестом текста: rasterization
различается по platform. Основные invariants проверять через render plan,
grid positions, styles и glyph bounds с допустимым tolerance.

## Обязательные проверки

После каждого этапа запускать targeted tests нового crate. Перед завершением
каждого PR и всего виджета запускать требования workspace:

```bash
cargo +nightly fmt -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features --fail-under-lines 80
```

Дополнительно:

```bash
cargo tree -p otty-ui-term-gpui
cargo test -p otty-ui-term-gpui --all-features
cargo run -p otty-ui-term-gpui --example full_screen
cargo run -p otty-ui-term-gpui --example split_view
cargo run -p otty-ui-term-gpui --example blocks_overlay
cargo run -p otty-ui-term-gpui --example backend_switching
```

Coverage сравнивать с baseline и не компенсировать падение тестами
infrastructure/bootstrap кода.

## Основные риски и меры

### GPUI pre-1.0 и расхождение published/main

Риск: breaking API и разный bootstrap при одинаково выглядящем номере версии.

Меры:

- exact crates.io pin;
- ссылки в коде/документации на docs `0.2.2`, не `latest`;
- отдельный upgrade PR;
- не смешивать GPUI upgrade с terminal features;
- повторный compile/text/input spike перед каждым upgrade.

### Grid и shaping

Риск: shaped advances, ligatures, fallback glyphs и BiDi не совпадут с
фиксированной terminal grid.

Меры:

- отключить discretionary ligatures по умолчанию;
- хранить cell columns независимо от glyph count;
- wide/combining/RTL fixtures до renderer implementation;
- device-pixel snapping;
- full redraw correctness до cache optimization.

### IME и raw input

Риск: committed character отправится дважды или composition попадёт в PTY
раньше commit.

Меры:

- разделить actions, control keys, raw printable и marked text;
- один commit path;
- deterministic GPUI tests;
- native smoke на X11/Wayland/macOS.

### Runtime lifecycle

Риск: entity закрыта, а blocking runtime/thread/child process остаются.

Меры:

- weak entity event task;
- idempotent shutdown;
- `on_release` плюс Drop fallback;
- join/reaper вне UI thread;
- integration test на многократное open/close.

### Backend replacement races

Риск: старый backend присылает frame/exit после запуска нового и портит state
той же entity либо два runtime одновременно принимают input.

Меры:

- монотонный `BackendGeneration` на каждом start;
- generation check до любого state mutation или host event;
- сначала отключить старый request handle, затем начать новый start;
- один active `BackendSlot` и отдельный список только завершающихся reapers;
- deterministic tests с задержанным frame/exit старого backend;
- `Failed` не уничтожает entity и не блокирует retry.

### Frame backpressure

Риск: unbounded `Arc<SnapshotOwned>` queue растёт при большом output.

Меры:

- latest-frame coalescing в core;
- critical non-frame events не теряются;
- bounded stress test;
- memory measurement при длительном output.

### Dependency policy и system libraries

Риск: `cargo deny`, Git sources или Linux packages ломают CI/release.

Меры:

- dependency approval до edit;
- exact source allowlist только после отдельного согласования;
- X11/Wayland CI jobs;
- package build spike до feature implementation;
- документировать native packages в README.

## Milestones

### M1 — Feasibility proven

- dependency согласована;
- GPUI builds на Linux/macOS;
- custom element shapes grid-safe text;
- entity получает frame;
- `EventEmitter`/stored `Subscription` доставляют typed signal;
- local и custom backend собираются через единый `TerminalBackend` contract.

### M2 — P0 interactive terminal

- local/SSH lifecycle и runtime replacement с generation guard;
- rendering, resize, keyboard, selection, scroll, mouse и clipboard;
- `full_screen` работает.

### M3 — Embeddable widget parity

- framed/focus behavior;
- multiple terminals;
- theme/font/bindings;
- runtime config API, typed actions и host signals;
- blocks/external overlay;
- все шесть parity examples и `backend_switching`.

### M4 — Production-ready

- IME, cursor shapes/blink и bracketed paste;
- bounded backpressure;
- performance/platform matrix;
- docs, lint, deny, tests и coverage green.

## Definition of Done

- [ ] Добавлен `otty-ui/terminal-gpui` с package
  `otty-ui-term-gpui`.
- [ ] Crate не зависит от Iced.
- [ ] Public API не содержит `iced::*`, Iced `Message`/`Subscription`,
  обязательный numeric widget ID или внешний frame reducer.
- [ ] Widget встраивается как `Entity<Terminal>` и не владеет host window.
- [ ] Host создаёт entity через `cx.new`, рендерит через `.child` и управляет
  через `Entity::update`/GPUI actions.
- [ ] Semantic signals реализованы через `EventEmitter<TerminalEvent>`;
  lifetime `Subscription` документирован и протестирован.
- [ ] `TerminalConfig` отделён от backend и поддерживает runtime theme, font,
  appearance, behavior и bindings replacement.
- [ ] Local и SSH sessions используют `otty-libterm`, без дублирования VTE.
- [ ] Custom backend может реализовать документированный `TerminalBackend`.
- [ ] Backend заменяется в той же entity; generation guard отбрасывает stale
  events, а failed start допускает retry.
- [ ] Нет общего `execute(Command)`; public methods и typed GPUI actions имеют
  единый operation path.
- [ ] P0 и обязательные P1 parity items выполнены.
- [ ] Все шесть существующих examples имеют GPUI аналоги.
- [ ] Дополнительный `backend_switching` демонстрирует replacement и signals.
- [ ] Несколько widgets независимо работают и получают focus.
- [ ] Нет `unwrap()` в production code.
- [ ] Новые public items документированы, public fields отсутствуют.
- [ ] Не добавлены speculative traits/managers/factories.
- [ ] Business-significant code реализован test-first.
- [ ] Если появились mocks, они сделаны через согласованный `mockall`.
- [ ] Runtime shutdown не оставляет threads/child processes.
- [ ] Frame queue имеет доказанную bounded/coalescing policy.
- [ ] Linux X11, Linux Wayland и macOS smoke tests пройдены.
- [ ] `fmt`, Clippy, deny, workspace tests и coverage проходят.
- [ ] Iced reference не удалён до отдельного решения о cutover.

## Использованные первичные источники

- [GPUI 0.2.2 documentation](https://docs.rs/gpui/0.2.2/gpui/)
- [Опубликованный GPUI 0.2.2 `hello_world` с `Application::new()`](https://docs.rs/crate/gpui/0.2.2/source/examples/hello_world.rs)
- [Опубликованный GPUI 0.2.2 input/IME example](https://docs.rs/crate/gpui/0.2.2/source/examples/input.rs)
- [GPUI `Element` API](https://docs.rs/gpui/0.2.2/gpui/trait.Element.html)
- [GPUI `Entity` API](https://docs.rs/gpui/0.2.2/gpui/struct.Entity.html)
- [GPUI `Render` API](https://docs.rs/gpui/0.2.2/gpui/trait.Render.html)
- [GPUI `Focusable` API](https://docs.rs/gpui/0.2.2/gpui/trait.Focusable.html)
- [GPUI `EventEmitter` API](https://docs.rs/gpui/0.2.2/gpui/trait.EventEmitter.html)
- [GPUI `Context` notify/emit/subscribe/spawn API](https://docs.rs/gpui/0.2.2/gpui/struct.Context.html)
- [GPUI `Subscription` lifetime API](https://docs.rs/gpui/0.2.2/gpui/struct.Subscription.html)
- [GPUI typed `Action` API](https://docs.rs/gpui/0.2.2/gpui/trait.Action.html)
- [GPUI `KeyBinding` API](https://docs.rs/gpui/0.2.2/gpui/struct.KeyBinding.html)
- [GPUI `ShapedLine` API](https://docs.rs/gpui/0.2.2/gpui/struct.ShapedLine.html)
- [GPUI crate README](https://github.com/zed-industries/zed/blob/30730a305ae235f3be44643d5895e142048ef701/crates/gpui/README.md)
- [GPUI custom element implementation guide/source](https://github.com/zed-industries/zed/blob/30730a305ae235f3be44643d5895e142048ef701/crates/gpui/src/element.rs)
- [GPUI input example](https://github.com/zed-industries/zed/blob/30730a305ae235f3be44643d5895e142048ef701/crates/gpui/examples/input.rs)
- [Zed production `TerminalElement`](https://github.com/zed-industries/zed/blob/30730a305ae235f3be44643d5895e142048ef701/crates/terminal_view/src/terminal_element.rs)
- [Zed production `TerminalView`](https://github.com/zed-industries/zed/blob/30730a305ae235f3be44643d5895e142048ef701/crates/terminal_view/src/terminal_view.rs)
