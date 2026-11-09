# Виджеты

Это часть описания [архитектуры `app`](README.md). Здесь записано, из каких файлов состоит виджет, что можно писать в каждом файле и как большой виджет делится на дочерние.

*Виджет* это папка в `app/src/widgets/`, а не `iced::Widget`: одна часть интерфейса со своим состоянием и логикой. `X` обозначает имя виджета в CamelCase, `x` имя его папки. *View* это функция, которая рисует один кусок интерфейса: панель, меню или диалог. *View model* это данные, которые показывает view.

## Содержание

- [Что виджет может импортировать](#что-виджет-может-импортировать)
- [Файлы виджета](#файлы-виджета)
  - [Что видно снаружи](#что-видно-снаружи)
- [Правила для редьюсера и view](#правила-для-редьюсера-и-view)
- [Пример виджета](#пример-виджета)
  - [Имена](#имена)
  - [`widgets/notes/event.rs`](#widgetsnoteseventrs)
  - [`widgets/notes/state.rs`](#widgetsnotesstaters)
  - [`widgets/notes/reducer.rs`](#widgetsnotesreducerrs)
  - [`widgets/notes/model.rs`](#widgetsnotesmodelrs)
  - [`widgets/notes/view/`](#widgetsnotesview)
  - [`widgets/notes/storage.rs`](#widgetsnotesstoragers)
  - [`widgets/notes/mod.rs`](#widgetsnotesmodrs)
  - [`router/notes.rs`](#routernotesrs)
  - [Подключение](#подключение)
  - [Если нужны данные другого виджета](#если-нужны-данные-другого-виджета)
- [Большие виджеты](#большие-виджеты)
  - [Файлы дочернего виджета](#файлы-дочернего-виджета)
  - [Видимость дочернего виджета](#видимость-дочернего-виджета)
  - [События дочернего виджета](#события-дочернего-виджета)
  - [Редьюсер дочернего виджета](#редьюсер-дочернего-виджета)
  - [Данные родителя](#данные-родителя)
  - [Пример: пользователь нажимает Run](#пример-пользователь-нажимает-run)
  - [Кто меняет дочерний виджет](#кто-меняет-дочерний-виджет)
  - [Подписка дочернего виджета](#подписка-дочернего-виджета)
  - [Границы дочернего виджета](#границы-дочернего-виджета)
  - [View model и view дочернего виджета](#view-model-и-view-дочернего-виджета)

## Что виджет может импортировать

- Свои файлы.
- [Домен](domain.md), [общий код](shared.md) и крейты.
- Модули ввода-вывода `services.rs` и `shell_integration.rs`, но только из своих `services.rs`, `storage.rs` и `subscription.rs`.

Другие виджеты, [приложение](app.md), [роутинг](routing.md) и [экраны](screens.md) виджет не импортирует никогда. Тип, нужный двум виджетам, лежит в [домене](domain.md). Как связаны все слои, показывает таблица [«Кто что импортирует»](README.md#кто-что-импортирует). Дочерний виджет импортирует ещё меньше, см. [«Границы дочернего виджета»](#границы-дочернего-виджета).

## Файлы виджета

Эти файлы есть у каждого виджета:

| Файл | Что в нём | Что запрещено |
| --- | --- | --- |
| `mod.rs` | объявления модулей, `XWidget` и реэкспорты из закрытых модулей | логика сверх передачи вызова дальше |
| `event.rs` | `XIntent`, `XEffect`, `XEvent` | всё остальное |
| `state.rs` | `XState` и правила, которые держат его корректным | `Task`, `Element`, файлы, процессы |
| `reducer.rs` | `reduce` и при необходимости `XCtx` | запуск фоновой работы |
| `model.rs` или `model/` | view model, построенные из состояния | изменение состояния |
| `view/` | по файлу на каждую view, обычно `view(props) -> Element<'_, XIntent>` | импорт `state` |

Ещё у каждого виджета есть маршрут `router/x.rs` в роутинге, см. [«Маршруты»](routing.md#маршруты).

Когда `model.rs` становится большим, вместо него заводят папку `model/`: `model/mod.rs` и по файлу на каждую view, с именем файла этой view. Например, view model из `view/main_area.rs` лежат в `model/main_area.rs`. В остальном файлы `model/` подчиняются тем же правилам, что и `model.rs`.
- Файлы внутри `model/` закрыты (`mod main_area;`), а `model/mod.rs` реэкспортирует их view model через `pub(crate) use`. Поэтому путь не меняется: view по-прежнему импортирует `super::super::model::MainAreaViewModel`.
- `from_state` в таком файле объявлен `pub(in super::super)`, то есть виден во всём виджете. С `pub(super)` его видел бы только `model/`, и `XWidget` не смог бы его вызвать. `pub(crate)` не подходит: `from_state` получает состояние виджета, а оно видно только внутри виджета.
- Помощники, которые нужны тестам нескольких файлов `model/`, лежат в `model/mod.rs`, в `#[cfg(test)] mod test_support`. Так у `containers`: `model/mod.rs`, `engine_tree.rs`, `inspector.rs`, `main_area.rs`, `no_engine.rs` и `status_bar.rs`.
- Если одной view model нужна другая, файл импортирует соседний: `use super::no_engine::NoEngineViewModel;`.

Эти файлы виджет добавляет, только когда они нужны:

| Файл | Что в нём |
| --- | --- |
| `types.rs` | типы данных и константы виджета, в том числе типы, которые лежат в его интентах, и данные, которые виджет отдаёт наружу, например `LaunchSummary` |
| `errors.rs` | тип ошибки виджета |
| `services.rs` | обращения во внешний мир |
| `storage.rs` | сохранённые данные виджета |
| `subscription.rs` | долгоживущие фоновые потоки |
| `<topic>.rs` | чистая логика с именем по смыслу, например `sort.rs`; без `Task`, `Element` и I/O |
| `child/` | дочерние виджеты большого виджета, по папке на каждый, см. [«Большие виджеты»](#большие-виджеты) |

Как выглядит каждый файл, показывает [«Пример виджета»](#пример-виджета).

### Что видно снаружи

Каждый модуль виджета либо открыт (`pub(crate) mod`), либо закрыт (`mod`). Открыт только тот модуль, которым пользуется код вне виджета.

| Модуль | Видимость | Почему |
| --- | --- | --- |
| `event`, `types` | открыт | это контракт виджета: с ними другие маршруты отправляют ему интенты. Например, `router/containers.rs` отправляет `TabsIntent::OpenCommandTab` с `TabMachine::ThisComputer` |
| `view` | открыт | его вызывают `view.rs` и файлы режимов в `screens/` |
| `storage`, `services` | открыт, но пользуется им только маршрут `router/x.rs`, а `storage` ещё и `XWidget::load` один раз при старте | ввод-вывод выполняет маршрут. Rust не умеет открыть модуль для одного файла, а в тесте архитектуры сейчас нет правил, поэтому это проверяет ревью |
| `child` и `child/<child>/` | открыт | маршрут, `screens/containers.rs` и тесты пишут пути к `event`, `types` и `view` дочернего виджета, например `containers::child::run::event::RunIntent`. Внутри дочернего виджета действует эта же таблица, только `storage`, `services` и `child` он не открывает |
| `state`, `reducer`, `model`, `<topic>.rs` и все остальные | закрыт | ими пользуется только сам виджет |

Внутри открытого модуля `pub(crate)` получает только то, чем пользуются снаружи. Например, в `storage.rs` это функции, которые вызывает маршрут, а вспомогательные функции остаются простыми `fn`.

`mod.rs` реэкспортирует через `pub(crate) use` только то, что лежит в закрытом модуле и нужно снаружи. Например:
- `XCtx` из `reducer`, потому что его собирает маршрут;
- тип view model из `model`, но только если код снаружи пишет имя этого типа, например в сигнатуре функции. Чтобы передать `app.widgets.x.vm()` в `<View>Props`, реэкспорт не нужен.

У дочернего виджета вместо этого `pub(in super::super) use`, см. [«Видимость дочернего виджета»](#видимость-дочернего-виджета).

Что лежит в открытом модуле, не реэкспортируется. Поэтому у каждого типа один путь, и этот путь показывает файл: `crate::widgets::tabs::event::TabsIntent`, `crate::widgets::tabs::types::TabMachine`. Сам `XWidget` лежит в `mod.rs`, и его путь `crate::widgets::tabs::TabsWidget`.

## Правила для редьюсера и view

- Редьюсер может менять состояние, возвращать эффекты и операции над элементами интерфейса, например фокус. Ему нельзя использовать `Task::perform`, `Task::future`, `Task::run`, `Task::stream`, файлы и процессы. Благодаря этому эффекты остаются обычными данными, которые тесты могут прочитать.
- Редьюсер, которому нужны данные другого виджета, получает их в `XCtx`. Этот контекст собирает маршрут, см. [«Если нужны данные другого виджета»](#если-нужны-данные-другого-виджета).
- View получает свою view model, тему и простые значения в структуре `<View>Props` из того же файла. Если view размещает корневой экран и ей нужно отправить сообщение другого виджета, она обобщена по типу сообщения и получает его параметром, как статус-бар Containers: `view<'a, Message>(..., on_sessions: Message, ...) -> Element<'a, Message>`.
- Тип элемента везде в `app/src` пишется коротко: `Element<'a, X>`. Приложение рисует темой и рендерером, которые iced берёт по умолчанию, поэтому `Theme` и `iced::Renderer` в этом типе не пишут.

## Пример виджета

Виджет `notes` показывает заметку из файла `~/.config/otty/note.txt` и кнопку «Обновить». Любой новый виджет устроен так же: те же файлы и те же имена, меняется только содержимое. Маршрут `router/notes.rs` и подключение относятся к другим слоям, [роутингу](routing.md), [приложению](app.md) и [экранам](screens.md), но генератор создаёт их вместе с виджетом, поэтому они показаны здесь.

Путь нажатия: view отправляет `NotesIntent::ReloadPressed`, редьюсер возвращает `NotesEffect::LoadRequested`, маршрут читает файл в фоне и отправляет `NotesIntent::Loaded` с текстом, редьюсер кладёт текст в состояние, и view его показывает.

### Имена

`X` это имя виджета в CamelCase, `x` это имя его папки. `<View>` и `<view>` это имя одной view в CamelCase и в snake_case, по имени её файла: view из файла `view/notes_panel.rs` получает `NotesPanelProps`, а её view model называется `NotesPanelViewModel`. Единственная view model виджета может называться `NotesViewModel`, как в примере.

| Имя | Файл | Что это | В примере |
| --- | --- | --- | --- |
| `XWidget` | `mod.rs` | виджет целиком: хранит состояние и даёт другим слоям читать и менять его | `NotesWidget` |
| `XState` | `state.rs` | данные виджета и правила, которые держат их корректными | `NotesState` |
| `XIntent`, `XEffect`, `XEvent` | `event.rs` | см. [«События»](README.md#события) | `NotesIntent`, `NotesEffect`, `NotesEvent` |
| `reduce` | `reducer.rs` | применяет интент к состоянию и возвращает эффекты | `reducer::reduce` |
| `XCtx` | `reducer.rs` | данные других виджетов для редьюсера, только если они нужны | `NotesCtx` |
| `XViewModel` | `model.rs` или `model/<view>.rs` | то, что показывает view, только для чтения. View model называется по файлу своей view: `<View>ViewModel`, например `MainAreaViewModel` для `view/main_area.rs`. Если view model у виджета или дочернего виджета одна, она может называться `XViewModel`, как `TabsViewModel`. View model строки или пункта, которые рисует одна view, называется по этой view: `<View><Item>ViewModel`, например `EngineTreeRowViewModel` | `NotesViewModel` |
| `<View>Props` | `view/<view>.rs` | всё, что получает одна view: view model, тема, простые значения | `NotesPanelProps` |
| `route` | `router/x.rs` | маршрут: передаёт интенты в `reduce` и выполняет эффекты | `router/notes.rs` |

### `widgets/notes/event.rs`

```rust
/// What the notes widget is asked to do.
#[derive(Debug, Clone)]
pub(crate) enum NotesIntent {
    /// The Reload button was pressed.
    ReloadPressed,
    /// The note was read; carries its text.
    Loaded(String),
    /// Reading the note failed; carries the reason.
    LoadFailed(String),
}

/// What the notes reducer asks its route to do.
#[derive(Debug, Clone)]
pub(crate) enum NotesEffect {
    /// Read the note file; the result comes back as an intent.
    LoadRequested,
}

/// A notes event as the app loop carries it.
#[derive(Debug, Clone)]
pub(crate) enum NotesEvent {
    /// An intent for the reducer.
    Intent(NotesIntent),
    /// An effect for the route.
    Effect(NotesEffect),
}
```

### `widgets/notes/state.rs`

Поля приватные. Правило «не читать файл, пока идёт чтение» лежит здесь, а не в редьюсере.

```rust
/// The note text and whether it is being read.
#[derive(Debug, Default)]
pub(super) struct NotesState {
    text: String,
    is_loading: bool,
}

impl NotesState {
    /// Return the note text.
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    /// Return whether the note is being read.
    pub(super) fn is_loading(&self) -> bool {
        self.is_loading
    }

    /// Start a read; returns `false` when one is already running.
    pub(super) fn begin_loading(&mut self) -> bool {
        if self.is_loading {
            return false;
        }

        self.is_loading = true;
        true
    }

    /// Replace the text with the one just read.
    pub(super) fn finish_loading(&mut self, text: String) {
        self.text = text;
        self.is_loading = false;
    }

    /// Keep the old text after a failed read.
    pub(super) fn stop_loading(&mut self) {
        self.is_loading = false;
    }
}
```

### `widgets/notes/reducer.rs`

Первая строка запрещает фоновую работу. Эффект возвращается как `Task::done(NotesEvent::Effect(..))`, поэтому тест может его прочитать.

```rust
#![forbid(clippy::disallowed_methods)]

use iced::Task;

use super::event::{NotesEffect, NotesEvent, NotesIntent};
use super::state::NotesState;

/// Apply a notes intent to the state and return the effects it asks for.
pub(super) fn reduce(
    state: &mut NotesState,
    intent: NotesIntent,
) -> Task<NotesEvent> {
    match intent {
        NotesIntent::ReloadPressed => {
            if !state.begin_loading() {
                return Task::none();
            }

            Task::done(NotesEvent::Effect(NotesEffect::LoadRequested))
        },
        NotesIntent::Loaded(text) => {
            state.finish_loading(text);
            Task::none()
        },
        NotesIntent::LoadFailed(message) => {
            state.stop_loading();
            log::warn!("notes read failed: {message}");
            Task::none()
        },
    }
}
```

### `widgets/notes/model.rs`

```rust
use super::state::NotesState;

/// What the notes panel shows.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NotesViewModel<'a> {
    pub(super) text: &'a str,
    pub(super) is_loading: bool,
}

impl<'a> NotesViewModel<'a> {
    /// Build the view model from the widget state.
    pub(super) fn from_state(state: &'a NotesState) -> Self {
        Self {
            text: state.text(),
            is_loading: state.is_loading(),
        }
    }
}
```

### `widgets/notes/view/`

`view/mod.rs` только объявляет файлы: `pub(crate) mod notes_panel;`. View импортирует `model` и `event`, но не `state`.

```rust
// widgets/notes/view/notes_panel.rs
use iced::Element;
use iced::widget::{column, text};
use otty_ui_kit::components::button::{Button, ButtonKind};

use super::super::event::NotesIntent;
use super::super::model::NotesViewModel;
use crate::i18n::{self, Key};
use crate::theme::ThemeProps;

/// Props for the notes panel.
pub(crate) struct NotesPanelProps<'a> {
    pub(crate) vm: NotesViewModel<'a>,
    pub(crate) theme: ThemeProps<'a>,
}

/// Render the note with a Reload button that waits while a read runs.
pub(crate) fn view(props: NotesPanelProps<'_>) -> Element<'_, NotesIntent> {
    let palette = props.theme.theme.ui();

    let mut reload = Button::new(i18n::t(Key::ButtonReload), palette)
        .kind(ButtonKind::Ghost);
    if !props.vm.is_loading {
        reload = reload.on_press(NotesIntent::ReloadPressed);
    }

    column![text(props.vm.text), reload].into()
}
```

### `widgets/notes/storage.rs`

Этот файл есть, потому что виджет читает файл. Его вызывает только маршрут `router/notes.rs`.

```rust
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

/// Read the saved note; a missing file means an empty note.
pub(crate) fn read_note() -> std::io::Result<String> {
    match fs::read_to_string(note_path()) {
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(String::new()),
        result => result,
    }
}

fn note_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();

    PathBuf::from(home).join(".config/otty/note.txt")
}
```

### `widgets/notes/mod.rs`

Открыты `event`, `view` и `storage`, остальные модули закрыты. Разрешение на запрещённые вызовы стоит на объявлении `storage`. `NotesViewModel` не реэкспортируется: `view.rs` передаёт `vm()` в `NotesPanelProps` и не пишет имя типа.

```rust
pub(crate) mod event;
mod model;
mod reducer;
mod state;
#[expect(clippy::disallowed_methods, reason = "this module reads the note")]
pub(crate) mod storage;
pub(crate) mod view;

use event::{NotesEvent, NotesIntent};
use iced::Task;
use model::NotesViewModel;
use state::NotesState;

/// Notes widget: shows the saved note and reads it again on request.
pub(crate) struct NotesWidget {
    state: NotesState,
}

impl NotesWidget {
    /// Create the widget with an empty note.
    pub(crate) fn new() -> Self {
        Self {
            state: NotesState::default(),
        }
    }

    /// Apply an intent to the state and return the effects it asks for.
    pub(crate) fn reduce(&mut self, intent: NotesIntent) -> Task<NotesEvent> {
        reducer::reduce(&mut self.state, intent)
    }

    /// Return what the notes panel shows.
    pub(crate) fn vm(&self) -> NotesViewModel<'_> {
        NotesViewModel::from_state(&self.state)
    }
}
```

Методы `XWidget`:
- `new()` создаёт виджет. Если начальное состояние читается из `storage`, вместо него пишут `load()`.
- `reduce` передаёт интент в `reducer::reduce` и ничего больше не делает.
- `vm()` строит view model через `from_state`. Если view несколько, у каждой свой метод: `<view>_vm()`, например `main_area_vm()` и `status_bar_vm()` у `containers`.
- Остальные методы только читают состояние, например `is_menu_open()`. Их используют маршруты, входной фильтр, `view.rs` и файлы режимов. Если метод отдаёт данные, которые не рисует ни одна view виджета, их тип лежит в `types.rs` и не называется view model. Например, `QuickLaunchWidget::launches()` и `top_level_launches()` возвращают `LaunchSummary`: по первому списку палитра команд строит свой, а по второму `screens/terminal.rs` строит меню New session.

### `router/notes.rs`

Маршрут передаёт интенты редьюсеру и выполняет эффекты. Результат фоновой работы возвращается интентом своего же виджета. Правила для маршрутов, в том числе имена параметров, описаны в разделе [«Маршруты»](routing.md#маршруты).

```rust
use iced::Task;

use super::AppEvent;
use crate::app::App;
use crate::widgets::notes::event::{NotesEffect, NotesEvent, NotesIntent};
use crate::widgets::notes::storage::read_note;

/// Route a notes event: reduce its intents and run its effects.
pub(crate) fn route(app: &mut App, event: NotesEvent) -> Task<AppEvent> {
    match event {
        NotesEvent::Intent(intent) => {
            app.widgets.notes.reduce(intent).map(AppEvent::Notes)
        },
        NotesEvent::Effect(effect) => handle_effect(effect),
    }
}

fn handle_effect(effect: NotesEffect) -> Task<AppEvent> {
    match effect {
        NotesEffect::LoadRequested => load(),
    }
}

/// Read the note in the background.
fn load() -> Task<AppEvent> {
    Task::perform(
        async { read_note().map_err(|err| format!("{err}")) },
        load_finished,
    )
}

fn load_finished(result: Result<String, String>) -> AppEvent {
    let intent = match result {
        Ok(text) => NotesIntent::Loaded(text),
        Err(message) => NotesIntent::LoadFailed(message),
    };

    AppEvent::Notes(NotesEvent::Intent(intent))
}
```

### Подключение

Генератор сам вписывает виджет в `widgets/mod.rs`, `router/mod.rs` и `app.rs`. Показать его в `view.rs` или в файле режима в `screens/` нужно вручную. В `guards.rs` ничего не добавляется: нажатие «Обновить» при открытом меню ведёт себя как обычный клик.

```rust
// widgets/mod.rs
pub(crate) mod notes;

pub(crate) struct Widgets {
    // ...
    pub(crate) notes: notes::NotesWidget,
}

// router/mod.rs
pub(crate) mod notes;

use crate::widgets::notes::event::NotesEvent;

pub(crate) enum AppEvent {
    // ...
    Notes(NotesEvent),
}

// router/mod.rs, в route
AppEvent::Notes(event) => notes::route(app, event),

// app.rs
use crate::widgets::notes::NotesWidget;

// app.rs, в App::new
let widgets = Widgets {
    // ...
    notes: NotesWidget::new(),
};

// view.rs или файл режима в screens/
use crate::widgets::notes::event::NotesEvent;
use crate::widgets::notes::view::notes_panel;

// там же, где виджет виден
notes_panel::view(notes_panel::NotesPanelProps {
    vm: app.widgets.notes.vm(),
    theme: theme_props,
})
.map(|intent| AppEvent::Notes(NotesEvent::Intent(intent)))
```

### Если нужны данные другого виджета

Допустим, редьюсеру `notes` нужен id активной вкладки. Тогда:
- в `reducer.rs` появляется `NotesCtx`, а `mod.rs` его реэкспортирует: `pub(crate) use reducer::NotesCtx;`. Модуль `reducer` закрыт, поэтому без реэкспорта маршрут не сможет собрать контекст;
- `NotesWidget::reduce` и `reducer::reduce` получают последним параметром `ctx: &NotesCtx`;
- маршрут собирает контекст из других виджетов перед вызовом `reduce`.

Поля контекста `pub(crate)`: это простой набор параметров без своих правил. Параметр `<'a>` нужен, только если поля ссылаются на чужие данные.

```rust
// widgets/notes/reducer.rs
/// Data from other widgets that the notes reducer reads.
pub(crate) struct NotesCtx {
    pub(crate) active_tab_id: Option<u64>,
}

// router/notes.rs
use crate::widgets::notes::NotesCtx;

// router/notes.rs, в route
NotesEvent::Intent(intent) => {
    let ctx = NotesCtx {
        active_tab_id: app.widgets.tabs.active_tab_id(),
    };

    app.widgets.notes.reduce(intent, &ctx).map(AppEvent::Notes)
},
```

## Большие виджеты

Если у куска виджета есть своё состояние и свои интенты, он становится *дочерним виджетом*: папкой `x/child/<child>/`. `<child>` и `<Child>` это имя дочернего виджета в snake_case и в CamelCase. Виджет, в котором он лежит, называется *родителем*. Одно лишь число строк не повод для разделения. В этом документе слова «дочерний виджет» значат только это.

Все дочерние виджеты лежат в папке `child/` родителя, а свои папки родителя `model/` и `view/` лежат рядом с ней. Так по дереву сразу видно, что относится к самому виджету, а что к дочерним. Например, у `containers` четыре дочерних виджета:

```text
app/src/widgets/containers/
  child/
    mod.rs       только объявляет дочерние виджеты
    actions/     меню строки, подтверждение и Prune
    create/      диалоги Pull image, Create volume и Create network
    inspector/   вкладки правой панели
    run/         форма Run container
  model/
  view/
  event.rs  state.rs  reducer.rs  ...
```

Дочерний виджет это всегда папка прямо внутри `child/`. Другие папки виджета, даже с `mod.rs`, дочерними виджетами не считаются.

Ниже примеры взяты из `run`.

### Файлы дочернего виджета

У дочернего виджета те же файлы, что у виджета: `mod.rs`, `event.rs`, `state.rs`, `reducer.rs`, `model.rs` или папка `model/` и `view/`. Ещё он может добавить `types.rs`, `subscription.rs` и файлы по темам, например `child/run/checks.rs`. У дочернего виджета нет `XWidget`, своего маршрута, `<Child>Event` и своих дочерних виджетов.

### Видимость дочернего виджета

- Родитель объявляет `pub(crate) mod child;`. `child/mod.rs` объявляет каждый дочерний виджет строкой `pub(crate) mod <child>;` и больше ничего не содержит:

```rust
// widgets/containers/child/mod.rs
pub(crate) mod actions;
pub(crate) mod create;
pub(crate) mod inspector;
pub(crate) mod run;
```

- Внутри дочернего виджета действует та же таблица видимости, что у виджета. Например, у `run` открыты `event`, `types` и `view`, а остальные модули закрыты. Отличие одно: дочерний виджет не открывает `storage` и `services`, потому что у него нет своего маршрута, а ввод-вывод выполняет маршрут родителя.
- `mod.rs` дочернего виджета объявляет модули и через `pub(in super::super) use` реэкспортирует родителю то, чем родитель пользуется: состояние, `reduce`, Ctx, итог и view model. Из `child/run/` первый `super` ведёт в `child`, а второй в родителя `containers`. Если родителю нужна функция из подписки или из файла по теме, её реэкспортируют так же.

```rust
// widgets/containers/child/run/mod.rs
pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod checks;
mod model;
mod reducer;
mod state;

pub(in super::super) use model::{RunFormViewModel, RunPanelViewModel};
pub(in super::super) use reducer::{RunCtx, RunEngine, RunOutcome, reduce};
pub(in super::super) use state::RunState;
```

- Rust не даёт реэкспортировать элемент шире, чем он объявлен. Поэтому внутри дочернего виджета `reduce`, `from_state` и типы, которые получает родитель, объявлены `pub(crate)`, а не `pub(super)`. С `pub(super)` их видел бы только сам дочерний виджет, и `pub(in super::super) use` в его `mod.rs` не скомпилировался бы. Дальше родителя они всё равно не видны, потому что их модули закрыты.
- Методы состояния дочернего виджета, которые его меняют, объявлены `pub(super)` и видны только внутри дочернего виджета. Геттеры объявлены `pub(crate)`: через них родитель читает состояние дочернего виджета.
- Закрытые типы дочернего виджета, например `RunForm` из `child/run/state.rs`, нельзя назвать снаружи дочернего виджета, даже в тестах родителя. Такой тест читает дочерний виджет через геттеры, не называя этих типов, или проверяет его view model, например `FormDialogViewModel::from_state(state.create())`.

### События дочернего виджета

Интент родителя оборачивает интент дочернего виджета, а эффект родителя оборачивает эффект дочернего виджета: `ContainersIntent::Run(RunIntent)` и `ContainersEffect::Run(RunEffect)`. Поэтому `AppEvent` и маршрут видят только родителя. Маршрут остаётся одним файлом, и эффекты каждого дочернего виджета в нём разбирает своя приватная функция. Результат фоновой работы маршрут возвращает интентом дочернего виджета внутри интента родителя, например `ContainersIntent::Run(RunIntent::Finished { .. })`.

### Редьюсер дочернего виджета

Сверху `reducer.rs` дочернего виджета стоит тот же `#![forbid(clippy::disallowed_methods)]`. `reduce` дочернего виджета получает состояние, интент и, если нужно, Ctx. Он возвращает эффекты и, если нужно, *итог* (`<Child>Outcome`):

```rust
// widgets/containers/child/run/reducer.rs
pub(crate) fn reduce(
    state: &mut RunState,
    intent: RunIntent,
    ctx: &RunCtx<'_>,
) -> (Task<RunEffect>, Option<RunOutcome>)
```

- Эффект это просьба к внешнему миру, например `RunEffect::RunContainer`: создать и запустить контейнер.
- Итог это сообщение родителю. Например, `RunOutcome::Started { engine, id }` значит, что контейнер запущен и форма закрылась. Родитель обрабатывает итог в том же обновлении.

### Данные родителя

Всё, что дочерний виджет читает у родителя, лежит в его Ctx. Ctx дочернего виджета объявлен в его `reducer.rs`. Например, `RunCtx` несёт найденные движки с их сетями, включён ли SELinux и домашнюю папку. Тип, который нужен двум дочерним виджетам, лежит в `types.rs` родителя. Например, `OpenList`, открытый список с выбранной строкой, входит и в `ActionsCtx`, и в `InspectorCtx`.

Состояние дочернего виджета лежит полем в состоянии родителя. Rust разрешает взять это поле для изменения и в то же время читать другие поля только там, где эти поля видны, то есть в `state.rs` родителя. Поэтому дочерний виджет выдают методы состояния родителя:

| Метод в `state.rs` родителя | Кто его вызывает | Что он возвращает |
| --- | --- | --- |
| `<child>_and_ctx(&mut self)` | `reducer.rs` родителя, если у дочернего виджета есть Ctx | дочерний виджет для изменения и его Ctx, например `run_and_ctx() -> (&mut RunState, RunCtx<'_>)` |
| `<child>_mut(&mut self)` | `reducer.rs` родителя, если у дочернего виджета нет Ctx | дочерний виджет для изменения, например `create_mut()` |
| `<child>(&self)` и `<child>_ctx(&self)` | view model и код, который только читает дочерний виджет | дочерний виджет и его Ctx для чтения, например `run()` и `run_ctx()` |

Для каждого дочернего виджета в `reducer.rs` родителя есть три функции: `reduce_<child>` передаёт ему интент, `on_<child>_outcome` обрабатывает его итог, а `wrap_<child>_effect` оборачивает его эффект в событие родителя. Эффект дочернего виджета выполняет функция `<child>_effect` в маршруте родителя, поэтому имена не совпадают. Родитель берёт дочерние виджеты через `use super::child::{actions, create, inspector, run};`.

```rust
// widgets/containers/reducer.rs
fn reduce_run(
    state: &mut ContainersState,
    intent: RunIntent,
) -> Task<ContainersEvent> {
    let (run, ctx) = state.run_and_ctx();
    let (effects, outcome) = run::reduce(run, intent, &ctx);
    let after = outcome
        .map_or_else(Task::none, |outcome| on_run_outcome(state, outcome));

    Task::batch([effects.map(wrap_run_effect), after])
}
```

### Пример: пользователь нажимает Run

Форма Run container открыта, и образ уже есть на компьютере.

| Проход | Событие | Кто обрабатывает | Что происходит |
| --- | --- | --- | --- |
| 1 | `RunIntent::Submitted` от `child/run/view/run_form.rs`; `screens/containers.rs` оборачивает его в `ContainersIntent::Run` | `reduce_run` в `widgets/containers/reducer.rs`, затем `run::reduce` | форма проверяет поля и ждёт запуска; дочерний виджет возвращает `RunEffect::RunContainer`, а родитель оборачивает его в `ContainersEffect::Run` |
| 2 | `ContainersEffect::Run(RunEffect::RunContainer { .. })` | `router/containers.rs` | запускает контейнер в фоне через `Task::perform` |
| 3 | `ContainersIntent::Run(RunIntent::Finished { .. })` с id нового контейнера | `reduce_run`, затем `run::reduce` | форма закрывается, дочерний виджет возвращает итог `RunOutcome::Started { engine, id }`. В том же обновлении `on_run_outcome` выделяет новый контейнер в таблице и возвращает `ContainersEffect::LoadLists` |
| 4 | `ContainersEffect::LoadLists(..)` | `router/containers.rs` | загружает списки движка в фоне |
| 5 | `ContainersIntent::ListsLoaded { .. }` | `widgets/containers/reducer.rs` | новый контейнер появляется в таблице уже выделенным |

### Кто меняет дочерний виджет

Родитель меняет дочерний виджет только через его интенты и его `reduce`, в том числе для своих команд. Например:
- когда пользователь открывает другой список, родитель отправляет форме `RunIntent::Close`, а не очищает её состояние сам;
- после каждого своего интента родитель отправляет инспектору `InspectorIntent::FollowSelection`, и инспектор сам решает, чьи логи читать и что загрузить.

`reduce` дочернего виджета вызывает только `reducer.rs` родителя.

### Подписка дочернего виджета

Если дочернему виджету нужен долгоживущий фоновый поток, он лежит в его `subscription.rs` и отдаёт интенты дочернего виджета. Родитель переводит этот поток в свои интенты в своём `subscription.rs`. Например, `inspector::followed_logs` возвращает `Subscription<InspectorIntent>`, а родитель переводит его в `ContainersIntent::Inspector`.

### Границы дочернего виджета

Дочерний виджет импортирует свои файлы, `types`, `errors` и файлы по темам родителя, а также домен и общий код. Он никогда не импортирует состояние, редьюсер, события, view model, подписку, view, `services` и `storage` родителя, то, что объявлено в `mod.rs` родителя, и соседние дочерние виджеты. До соседа дочерний виджет доходит через итог: родитель превращает его в интент соседа. Например, Run на строке образа даёт итог `ActionsOutcome::RunRequested`, и родитель отправляет форме `RunIntent::Open`.

### View model и view дочернего виджета

- View model дочернего виджета строится в его `model.rs` через `from_state(state, ctx)`, а у дочернего виджета без Ctx через `from_state(state)`. Её строит метод `XWidget::<view>_vm()`, например `ContainersWidget::run_form_vm()` вызывает `RunFormViewModel::from_state(self.state.run(), &self.state.run_ctx())`. Если view model дочернего виджета входит в view model родителя, её строит `model.rs` родителя.
- View дочернего виджета возвращает `Element<'_, RunIntent>`. Тот, кто её показывает, переводит элемент в интент родителя: `.map(ContainersIntent::Run)`. Экран, который собирает несколько дочерних виджетов, это view родителя: она переводит элемент каждого дочернего виджета в интент родителя.
