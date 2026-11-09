# Архитектура `app`

Здесь записано, как устроен крейт `app`: какие файлы создавать и какой код куда писать. Почему выбрана именно эта архитектура, объясняет [ADR 0001](../adr/0001-keep-widget-architecture.md). `crates/` сюда не входит: это библиотеки со своей архитектурой.

Здесь *виджет* означает папку в `app/src/widgets/`, а не `iced::Widget`. `X` обозначает имя виджета. *View* это функция, которая рисует один кусок интерфейса: панель, меню или диалог. *View model* это данные, которые показывает view.

## Слои

| Слой | Файлы | Что может импортировать |
| --- | --- | --- |
| Связка | `main.rs`, `app.rs`, `state.rs`, `update.rs`, `subscription.rs`, `guards.rs`, `view.rs`, `screens/`, `widgets/mod.rs`, `router/` | всё |
| Виджеты | `widgets/<name>/` | свои файлы, общие модули, крейты |
| Общие модули | `i18n/`, `theme.rs`, `fonts.rs`, `icons.rs`, `geometry.rs`, `layout.rs`, `shortcuts.rs`, `commands.rs`, `services.rs`, `shell_integration.rs`, `shell_session.rs`, `focus.rs`, `components/`, `domain/` | другие общие модули, крейты |

В связке нет бизнес-правил. Виджет никогда не импортирует другой виджет или связку. Тип, нужный двум виджетам, лежит в `domain/`. `testing/` и код под `#[cfg(test)]` служат только тестам, и эти правила на них не распространяются.

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

Ещё у каждого виджета есть маршрут `router/x.rs`, см. «Путь события».

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
| `<part>/` | часть большого виджета, см. «Большие виджеты» |

Как выглядит каждый файл, показывает [«Пример виджета»](#пример-виджета).

### Что видно снаружи

Каждый модуль виджета либо открыт (`pub(crate) mod`), либо закрыт (`mod`). Открыт только тот модуль, которым пользуется код вне виджета.

| Модуль | Видимость | Почему |
| --- | --- | --- |
| `event`, `types` | открыт | это контракт виджета: с ними другие маршруты отправляют ему интенты. Например, `router/containers.rs` отправляет `TabsIntent::OpenCommandTab` с `TabMachine::ThisComputer` |
| `view` | открыт | его вызывают `view.rs` и файлы режимов в `screens/` |
| `storage`, `services` | открыт, но пользуется им только маршрут `router/x.rs`, а `storage` ещё и `XWidget::load` один раз при старте | ввод-вывод выполняет маршрут. Rust не умеет открыть модуль для одного файла, поэтому это проверяет тест архитектуры |
| `<part>/` | открыт | маршрут, `screens/containers.rs` и тесты пишут пути к `event`, `types` и `view` части, например `containers::run::event::RunIntent`. Внутри части действует эта же таблица, только `storage` и `services` часть не открывает |
| `state`, `reducer`, `model`, `<topic>.rs` и все остальные | закрыт | ими пользуется только сам виджет |

Внутри открытого модуля `pub(crate)` получает только то, чем пользуются снаружи. Например, в `storage.rs` это функции, которые вызывает маршрут, а вспомогательные функции остаются простыми `fn`.

`mod.rs` реэкспортирует через `pub(crate) use` только то, что лежит в закрытом модуле и нужно снаружи. Например:
- `XCtx` из `reducer`, потому что его собирает маршрут;
- тип view model из `model`, но только если код снаружи пишет имя этого типа, например в сигнатуре функции. Чтобы передать `app.widgets.x.vm()` в `<View>Props`, реэкспорт не нужен.

У части вместо этого `pub(super) use`, см. «Большие виджеты».

Что лежит в открытом модуле, не реэкспортируется. Поэтому у каждого типа один путь, и этот путь показывает файл: `crate::widgets::tabs::event::TabsIntent`, `crate::widgets::tabs::types::TabMachine`. Сам `XWidget` лежит в `mod.rs`, и его путь `crate::widgets::tabs::TabsWidget`.

## События

- **`XIntent`** это то, о чём просят виджет: нажатие во view, результат фоновой работы, такт подписки или команда от маршрута.
- **`XEffect`** это то, о чём редьюсер просит, потому что сам сделать не может.
- **`XEvent`** объединяет их, а `AppEvent` объединяет события всех виджетов.

## Откуда приходит событие

iced доставляет приложению `Message`: событие и отметку, откуда оно пришло.

```rust
enum Message {
    Input(AppEvent),    // нажатие или ввод в интерфейсе, клавиша
    Internal(AppEvent), // эффект, результат фоновой работы, подписка
}
```

`Message` содержит `AppEvent`, а `AppEvent` не содержит `Message`, поэтому уровней всегда ровно два. Отметки ставятся только в `app.rs`:

| Откуда | Отметка |
| --- | --- |
| `App::view`: всё, что приходит из нарисованного интерфейса | `Input` |
| `App::subscription`: клавиатура | `Input` |
| `App::subscription`: остальные подписки | `Internal` |
| `App::update`: всё, что вернула обработка любого события | `Internal` |

Виджеты и маршруты не знают про `Message` и работают с `AppEvent`, поэтому отметку нельзя подделать или забыть.

## Путь события

Путь зависит от отметки:

```text
Input:     iced -> update_input    -> шорткат становится командой -> входной фильтр -> router/mod.rs -> router/x.rs
Internal:  iced -> update_internal -> router/mod.rs -> router/x.rs
```

- `update_input` сначала превращает клавишу шортката в `AppEvent::Command`, см. «Команды».
- Входной фильтр решает, можно ли обрабатывать ввод сейчас.
- `router/mod.rs` находит виджет, которому принадлежит событие.
- `router/x.rs`, маршрут виджета, смотрит на вид события:

| Вид | Что делает маршрут |
| --- | --- |
| интент | вызывает `XWidget::reduce` |
| эффект | выполняет его сам: запускает фоновую работу или отправляет сообщение другому виджету |

`reduce` меняет состояние и может вернуть эффект. Эффект не выполняется сразу: он возвращается в iced и снова проходит дорогу с начала, уже как `Internal`. Результат фоновой работы тоже приходит от iced, как `Internal`. Поэтому одно действие пользователя обычно проходит дорогу несколько раз, а фильтр решает один раз, на первом проходе.

В примерах ниже каждая строка таблицы это один такой проход. Первый идёт как `Input`, остальные как `Internal`.

### Пример: виджет делает свою работу

Пользователь раскрывает папку в Explorer.

| Проход | Событие | Кто обрабатывает | Что происходит |
| --- | --- | --- | --- |
| 1 | `ExplorerIntent::NodePressed` от `widgets/explorer/view/sidebar_tree.rs` | `widgets/explorer/reducer.rs` | папка выделена и раскрыта; редьюсер сам диск не читает и возвращает эффект |
| 2 | `ExplorerEffect::LoadFolderRequested` | `router/explorer.rs` | запускает чтение папки: `Task::perform` и `services::read_dir_nodes` |
| 3 | `ExplorerIntent::FolderLoaded`, когда чтение закончилось | `widgets/explorer/reducer.rs` | файлы попадают в состояние, и view их показывает |

### Пример: реагирует другой виджет

Пользователь переключается на режим Containers.

| Проход | Событие | Кто обрабатывает | Что происходит |
| --- | --- | --- | --- |
| 1 | `ModesIntent::Select(Mode::Containers)` от `widgets/modes/view/mode_switcher.rs` | `widgets/modes/reducer.rs` | активный режим изменён; редьюсер возвращает эффект |
| 2 | `ModesEffect::ModeChanged(Mode::Containers)` | `router/modes.rs` | отправляет `ContainersIntent::Enter` через `Task::done` |
| 3 | `ContainersIntent::Enter` | `widgets/containers/reducer.rs` | виджет начинает поиск движков и возвращает `ContainersEffect::Discover` |

`modes` и `containers` не импортируют друг друга. Их связывает только `router/modes.rs`.

### Пример: сообщить пользователю об ошибке

Запись закреплённых режимов в `modes.json` не удалась.

| Проход | Событие | Кто обрабатывает | Что происходит |
| --- | --- | --- | --- |
| 1 | `ModesEffect::SavePinned` | `router/modes.rs` | пишет файл в фоне; результат получает функция `pinned_saved` в том же маршруте |
| 2 | `NotificationsIntent::Push(text)` | `widgets/notifications/reducer.rs` | `pinned_saved` записала ошибку в лог и вызвала `notifications::push`, а та отправила этот интент через `Task::done`. Редьюсер добавляет уведомление, и вверху окна появляется тост |
| 3 | `NotificationsIntent::Tick` от подписки виджета | `widgets/notifications/reducer.rs` | такты идут, пока виден хотя бы один тост; через 5 секунд тост пропадает |

Текст собирает маршрут: `i18n::with_error` подставляет ошибку в шаблон с `{error}`, например `TplPinnedModesSaveFailed`. Время для редьюсера тоже читает маршрут: `NotificationsCtx { now }`.

Если результат приходит интентом виджета, о том, сообщать ли, решает редьюсер, и текст собирает тоже он. Например, остановка контейнера не удалась: `ActionsIntent::Finished` с ошибкой приходит в часть `actions`, она отдаёт родителю итог `ActionsOutcome::Failed(text)`, редьюсер `containers` возвращает `ContainersEffect::Failed(text)`, а `router/containers.rs` вызывает `notifications::push`. Так же сообщают:
- о неудачном Pull и о форме Run, которая при этом остаётся открытой со всеми полями;
- о запуске из Launches и о записи дерева Launches (`QuickLaunchEffect::Failed`);
- о терминале, который не запустился (`TerminalWorkspaceEffect::Failed`);
- о Settings, которые не удалось прочитать или записать или чей файл сломан (`SettingsEffect::Failed`);
- о папке в Files, которую пользователь раскрыл, а прочитать её не удалось (`ExplorerEffect::Failed`).

В Containers ошибка загрузки списка и ошибки в правой панели остаются на своём месте в окне. Ошибки проверки полей формы тоже остаются рядом с полями.

Если одна и та же ошибка повторяется сама, редьюсер сообщает только о первой. Дерево Launches пробует записаться снова каждые 200 мс, пока запись не удастся, поэтому `QuickLaunchState::fail_persist` возвращает `true` только для первой неудачи после последней удачной записи.

Маршрут Files отдаёт отдельным интентом `ExplorerIntent::FolderLoadFailed` только ошибку той загрузки, которую начало нажатие на папку. Загрузка корня вслед за папкой терминала и перечитывание папки после изменений на диске по-прежнему отдают `ExplorerIntent::LoadFailed`, и редьюсер только пишет её в лог.

Уведомление получает только ошибка операции, которую начал пользователь. Фоновые ошибки, которые повторяются сами, остаются в логе: запись shell integration, загрузка и перечитывание корня Files, наблюдение за его изменениями и записи папки, которые не удалось прочитать.

Файлы, которые читаются при старте, сообщают о себе иначе. Их читает `load()` виджета ещё до первого обновления, поэтому маршрута у этого чтения нет. Если файл Launches, Settings или закреплённых режимов не удалось прочитать или он сломан, виджет начинает со значений по умолчанию и запоминает готовый текст ошибки, а `load_problem()` отдаёт его. `App::new` после `AppEvent::IcedReady` отправляет каждый такой текст как `NotificationsIntent::Push` с отметкой `Message::Internal`. Без этого пользователь не узнал бы, что следующее изменение перезапишет его файл. Если файла нет, это первый запуск, и о нём ничего не сообщается.

Результат открытия ссылки тоже никуда не приходит, поэтому его, как `pinned_saved`, разбирает функция маршрута: `link_opened` в `router/containers.rs`. Тост появляется, если не удалось запустить программу, которая открывает ссылку в браузере. Если программа запустилась, но потом завершилась с ошибкой, это видно только в логе: ответ приходит позже, во вспомогательном потоке.

### Правила

- Редьюсер может менять состояние, возвращать эффекты и операции над элементами интерфейса, например фокус. Ему нельзя использовать `Task::perform`, `Task::future`, `Task::run`, `Task::stream`, файлы и процессы. Благодаря этому эффекты остаются обычными данными, которые тесты могут прочитать.
- `router/x.rs` единственное место, где выполняются эффекты виджета `x`. Этот файл называется маршрутом виджета.
- Маршрут выбирает, что делать, только по виду интента или эффекта, а не по состоянию виджетов. Состояние он читает только для того, чтобы собрать `XCtx` или данные, которые нужны эффекту. Решения принимает редьюсер и сообщает о них эффектами. Например, в `notes` правило «не читать файл, пока идёт чтение» проверяет редьюсер через `begin_loading`, а маршрут только выполняет `NotesEffect::LoadRequested`. Если для эффекта нет данных, маршрут ничего не отправляет: это не решение, а отсутствие данных. Например, `ExplorerIntent::SyncRoot` уходит, только если у активной вкладки есть локальная папка.
- Маршрут отправляет сообщение другому виджету через `Task::done(AppEvent::...)`. Вызывать его `reduce` или `route` нельзя. Прямые вызовы `route` разрешены только в `router/mod.rs` и `update.rs`. `router/mod.rs` передаёт событие маршруту: вернуть его в очередь значило бы зациклить. Кроме того, оба файла могут вызвать маршрут напрямую, если действие должно выполниться в том же обновлении, например отмена inline-редактирования перед обработкой нажатия.
- Редьюсер, которому нужны данные другого виджета, получает их в `XCtx`. Этот контекст собирает маршрут.
- View получает свою view model, тему и простые значения в структуре `<View>Props` из того же файла. Если view размещает корневой экран и ей нужно отправить сообщение другого виджета, она обобщена по типу сообщения и получает его параметром, как статус-бар Containers: `view<'a, Message>(..., on_sessions: Message, ...) -> Element<'a, Message>`.
- Тип элемента везде в `app/src` пишется коротко: `Element<'a, X>`. Приложение рисует темой и рендерером, которые iced берёт по умолчанию, поэтому `Theme` и `iced::Renderer` в этом типе не пишут.

## Пример виджета

Виджет `notes` показывает заметку из файла `~/.config/otty/note.txt` и кнопку «Обновить». Любой новый виджет устроен так же: те же файлы и те же имена, меняется только содержимое.

Путь нажатия: view отправляет `NotesIntent::ReloadPressed`, редьюсер возвращает `NotesEffect::LoadRequested`, маршрут читает файл в фоне и отправляет `NotesIntent::Loaded` с текстом, редьюсер кладёт текст в состояние, и view его показывает.

### Имена

`X` это имя виджета в CamelCase, `x` это имя его папки. `<View>` и `<view>` это имя одной view в CamelCase и в snake_case, по имени её файла: view из файла `view/notes_panel.rs` получает `NotesPanelProps`, а её view model называется `NotesPanelViewModel`. Единственная view model виджета может называться `NotesViewModel`, как в примере.

| Имя | Файл | Что это | В примере |
| --- | --- | --- | --- |
| `XWidget` | `mod.rs` | виджет целиком: хранит состояние и даёт другим слоям читать и менять его | `NotesWidget` |
| `XState` | `state.rs` | данные виджета и правила, которые держат их корректными | `NotesState` |
| `XIntent`, `XEffect`, `XEvent` | `event.rs` | см. «События» | `NotesIntent`, `NotesEffect`, `NotesEvent` |
| `reduce` | `reducer.rs` | применяет интент к состоянию и возвращает эффекты | `reducer::reduce` |
| `XCtx` | `reducer.rs` | данные других виджетов для редьюсера, только если они нужны | `NotesCtx` |
| `XViewModel` | `model.rs` или `model/<view>.rs` | то, что показывает view, только для чтения. View model называется по файлу своей view: `<View>ViewModel`, например `MainAreaViewModel` для `view/main_area.rs`. Если view model у виджета или части одна, она может называться `XViewModel`, как `TabsViewModel`. View model строки или пункта, которые рисует одна view, называется по этой view: `<View><Item>ViewModel`, например `EngineTreeRowViewModel` | `NotesViewModel` |
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

Маршрут передаёт интенты редьюсеру и выполняет эффекты. Результат фоновой работы возвращается интентом своего же виджета. Во всех файлах `router/` параметр типа `XIntent` называется `intent`, а параметр типа `XEffect` называется `effect`, как в `handle_effect(effect: NotesEffect)`.

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

## Команды

Одно и то же действие можно попросить несколькими способами: шорткатом, пунктом палитры команд или кнопкой шапки. Каждый способ выбирает `AppCommand` из общего модуля `commands.rs`, а выполняет команду один маршрут, `router/command.rs`. Так все способы запускают один и тот же код.

| Откуда | Как становится командой | Отметка |
| --- | --- | --- |
| клавиша, которую не взял ни один виджет | `update_input`: `shortcut_for_key_press`, затем `AppShortcut::command` | `Input` |
| привязка, о которой сообщил терминал с фокусом | `update_input`: `AppShortcut::command` | `Input` |
| пункт палитры | `PaletteTarget::Command`; `router/palette.rs` отправляет `AppEvent::Command` | `Internal` |
| кнопка шапки | эффект `chrome`; `router/chrome.rs` отправляет `AppEvent::Command` | `Internal` |

- Одно нажатие идёт только одним из двух путей. Терминал с фокусом забирает себе клавишу, которую отправил в shell или о которой сообщил как о шорткате. Поэтому `iced::keyboard::listen` такую клавишу уже не получает, и шорткат выполняется один раз. Остальные клавиши, например копирование выделения, доходят до приложения как обычно. События мыши терминал не забирает: приложение следит за указателем и над терминалом, например чтобы поставить контекстное меню.
- `AppShortcut` в `shortcuts.rs` это раскладка клавиш: у каждого шортката ровно одна команда, `AppShortcut::command`. Команда без клавиши, например «Новая папка», есть только в палитре.
- `AppCommand::is_available` решает, можно ли выполнить команду сейчас. Например, «Закрыть сессию» доступна только в режиме Terminal и при активной вкладке. Данные для этого маршрут собирает в `CommandCtx`. Палитра показывает только доступные команды, а маршрут выполняет только их, поэтому список и выполнение не расходятся.
- `router/command.rs` превращает команду в интенты виджетов, которые делают работу, например `AppCommand::CloseSession` в `TabsIntent::CloseActive`. Какую вкладку закрыть, решает редьюсер вкладок.
- Куда вернуть фокус после команды из палитры, `router/palette.rs` решает по самой команде: после переключения панелей и перехода в Terminal фокус возвращается в терминал.

## Входной фильтр

Фильтр проверяет только `Input`: что сделал пользователь, а не то, что из этого следует. Поэтому внутреннее событие он потерять не может. Код лежит в `filter_input` в `update.rs` и в `guards.rs`.

Шорткат приходит в фильтр уже командой. Поэтому фильтр проверяет его один раз и одинаково, каким бы путём ни пришла клавиша. Команда из палитры или кнопки шапки появляется позже, как `Internal`: фильтр уже проверил само нажатие.

### Оверлеи

*Оверлей* лежит поверх окна и первым получает ввод. Оверлеи бывают трёх видов:
- *меню*: список уведомлений под колокольчиком в шапке, палитра команд, меню всех режимов, меню New session и Launches, контекстное меню дерева Launches, контекстное меню панели терминала и меню строки Containers;
- *диалоги*: окно пароля SSH, Settings, диалоги создания Containers (например, Pull image) и диалог, который просит подтвердить удаление;
- *inline-редактирование*: имя новой папки или новое имя в дереве Launches.

*Тосты*, сообщения об ошибках вверху окна, не оверлеи. Esc их не закрывает, фокус они не берут, а затемнение под диалогом лежит над ними, поэтому крестик тоста нельзя нажать, пока диалог ждёт ответа. Те же ошибки хранит список уведомлений: колокольчик в шапке или пункт палитры открывает его, а значок на колокольчике считает уведомления, пришедшие после последнего открытия списка. Открытый список прячет тосты и считает всё увиденным; новая ошибка, пока он открыт, попадает в него без тоста. Крестик в строке убирает из списка только это уведомление, а кнопка с метлой очищает весь список; после обоих список остаётся открытым.

`top_overlay` в `guards.rs` говорит, какой оверлей сейчас сверху. Порядок постоянный: окно пароля SSH, Settings, список уведомлений, палитра, меню всех режимов, меню заголовков, затем оверлеи активного режима. В Containers это диалог создания, подтверждение и меню строки, в Terminal контекстное меню панели, контекстное меню Launches и inline-редактирование. Оверлеи другого режима не видны, поэтому не считаются. Settings и список уведомлений лежат поверх обоих режимов, поэтому считаются в любом. `Overlay::is_menu` и `Overlay::is_dialog` говорят, меню это или диалог.

Settings закрывается по Esc, по крестику в заголовке и по нажатию любой кнопкой мыши на затемнение вокруг окна. При закрытии правки, которые не сохранили, пропадают: черновик снова равен сохранённым настройкам. Save записывает настройки и применяет их, а Settings остаётся открытым.

Закрытие по нажатию мимо окна есть только у Settings, это исключение из правила диалогов. Окно пароля SSH и диалоги Containers на такое нажатие не отвечают и ждут ответа. Затемнение кита `Dialog` по умолчанию глотает нажатие; Settings включает закрытие через `Dialog::on_backdrop_press`. Нажатие на само окно, в том числе начатое в окне и отпущенное снаружи, Settings не закрывает. Исключение закрепляют тесты в `view.rs` и `settings_dialog.rs`.

### Порядок проверок

`filter_input` проверяет ввод по порядку:
1. Пока сверху меню или диалог, текст, который терминал отправил бы в shell, отбрасывается. Так набранные клавиши не попадут в shell под оверлеем, даже если у терминала остался фокус. Inline-редактирование этот текст пропускает: оно не лежит над терминалами.
2. Esc закрывает оверлей сверху: меню закрывается, диалог и inline-редактирование отменяются. Что отправить, знает `Overlay::escape_event`. Зажатая клавиша повторяет нажатие, но повторы не считаются, поэтому одно нажатие закрывает один оверлей. Поле ввода с фокусом забирает Esc себе и только теряет фокус, поэтому до фильтра нажатие не доходит. Поэтому поля диалогов и поле inline-редактирования отменяют их сами: через `TextField::on_escape` и `InputKeys` из кита. Поля Shell и Editor закрывают Settings так же. Палитра закрывается по Esc в своём поле так же.
3. Пока сверху диалог, он ждёт ответа, и решает `dialog_guard`. Проходят только ответы этого диалога (ввод в его поле, его кнопки, Enter и Esc), движения указателя, кнопки окна и события терминала. У Settings ответы это его поля, выбор темы и языка, Reset, Save, крестик и нажатие на затемнение вокруг окна. Шорткаты, кнопки приложения, клики мимо диалога (кроме Settings) и остальные клавиши ничего не делают. Поэтому шестерёнка, шорткат Settings и пункт палитры, пока Settings открыт, ничего не делают, и правки остаются. Диалог проверяется раньше меню. Поэтому окно пароля SSH, которое появилось над открытым меню, получает набранный текст, а меню под ним остаётся открытым до ответа.
4. Пока открыто меню всех режимов, шорткаты режимов и самого меню выполняются как обычно. Пока открыта палитра, её собственный шорткат только снова отдаёт фокус её полю: палитра остаётся открытой, и набранный текст не теряется.
5. Пока идёт inline-редактирование, ввод отменяет его, кроме печати в самом поле, движения указателя, событий терминала и клавиш, не являющихся шорткатами. Шорткат отменяет его, каким бы путём ни пришёл.
6. Пункт меню New session или Launches закрывает меню и выполняется.
7. Пока сверху меню, ввод проходит через правила меню, они ниже.

Пока открыто меню, ввод может:

| Исход | Что происходит |
| --- | --- |
| пропустить | событие обрабатывается, меню остаётся: например, пункт самого меню или движение указателя |
| отбросить | ничего не происходит: например, клавиша, которая не шорткат |
| закрыть и отбросить | меню закрывается, нажатие больше ничего не делает: обычный клик мимо меню |
| закрыть и выполнить | меню закрывается, затем выполняется действие: например, выбор режима или вкладки, любой шорткат |

Для всех меню правила одни и те же:
- клавиши никогда не попадают в терминал под меню;
- Esc закрывает меню, где бы ни был фокус;
- клик мимо меню любой кнопкой мыши закрывает его;
- шорткат закрывает меню и выполняется;
- кнопки панелей, поиск и колокольчик в шапке только закрывают меню, в том числе список уведомлений; Settings закрывает меню и открывает настройки, а кнопки окна и перетаскивание заголовка оставляют меню открытым;
- пункты самого меню выполняются: ввод в палитру и её строки, пункты меню строки Containers, метла и крестики строк списка уведомлений и так далее.

Правила лежат в `guards.rs`. Интенты виджета, которого там нет, ведут себя как обычный клик: закрывают меню и больше ничего не делают, а inline-редактирование отменяют. У некоторых виджетов поведение по умолчанию другое: например, нажатия в `quick_launch` при открытом меню игнорируются. Новый интент добавляют туда, только если ему нужно другое поведение.

### Фокус

Меню при открытии забирает клавиатуру у всех виджетов, и у терминала тоже: его редьюсер возвращает `focus::nothing()` из общего модуля `focus.rs`. Поэтому набранные клавиши не попадут в shell, а Esc дойдёт до фильтра. Без этого Esc до фильтра не дошёл бы: терминал с фокусом отправляет Esc в shell и забирает клавишу себе. Меню всех режимов сообщает об открытии эффектом `ModesEffect::MenuOpened`, а список уведомлений эффектом `NotificationsEffect::ListOpened`, и `focus::nothing()` вызывает их маршрут. Окно пароля SSH, палитра и inline-редактирование при открытии отдают фокус своему полю. Settings при открытии забирает клавиатуру у всех виджетов, как меню: его редьюсер возвращает `focus::nothing()` вместе с эффектом, который читает файл настроек. Диалоги Containers фокус не берут: терминалов в этом режиме на экране нет, а то, что набрано в поле под диалогом, отбрасывает `dialog_guard`.

По `top_overlay` маршрут `router/terminal_workspace.rs` собирает `TerminalWorkspaceCtx::is_overlay_open`. Пока открыт любой оверлей, терминал не забирает у него клавиатуру: ни по `FocusActive`, ни когда открывается новая вкладка.

Когда оверлей закрывается, его редьюсер сообщает об этом эффектом, и только если оверлей правда был открыт: `ModesEffect::MenuClosed`, `SidebarEffect::HeaderMenuClosed`, `QuickLaunchEffect::OverlayClosed`, `SettingsEffect::Closed`, `NotificationsEffect::ListClosed`, а палитра `PaletteEffect::Dismissed`. Маршрут отвечает на него `FocusActive`; маршрут меню всех режимов делает это только для `MenuClosed(Mode::Terminal)`. Контекстное меню панели при закрытии само отдаёт фокус своей панели. В Terminal клавиатуру снова получает активная панель активной вкладки. Если активная вкладка не терминал, фокус не получает никто. В Containers панели терминала нет на экране, поэтому после закрытия фокуса нет ни у кого.

## Режимы

Режим это отдельная общая задача: Terminal или Containers. Вкладки есть только у Terminal, и вкладка это только то, что запускает виджет терминала: shell, вкладка команды и редактор быстрого запуска. Остальное, например Settings, открывается окном поверх любого режима и режим не меняет.

Вкладка, которую открыли или выбрали, переключает на Terminal: `router/tabs.rs` отправляет виджету режимов `ModesIntent::TabShown`. Новая вкладка shell или команды делает это только тогда, когда её терминал запустился: `router/terminal_workspace.rs` получает `TerminalWorkspaceEffect::TabStarted`. Если терминал не запустился, вкладка закрывается, режим остаётся прежним, а пользователь видит тост. Запуск из Launches, который не удался ещё до вкладки, тоже приходит тостом и вкладку не открывает.

У каждого режима есть один файл в `screens/`: `screens/terminal.rs` и `screens/containers.rs`. В нём простые функции:
- `left_panel`: левая панель под переключателем режимов;
- `main`: основная область;
- `right_panel`: правая панель;
- `status_bar`: статус-бар;
- `overlays`: оверлеи режима. У Terminal это контекстные меню дерева Launches и панели терминала и меню New session и Launches, у Containers это меню строки и диалоги.

Трейта нет: функции выбирает `match` по `Mode` в `view.rs`. Аргументы у режимов могут быть разными, например `terminal::left_panel` и `terminal::overlays` получают ещё и `SidebarViewModel`. Файлы режимов относятся к связке, как и `view.rs`. `screens/mod.rs` объявляет их и держит то, чем пользуются и `view.rs`, и файлы режимов: `MainContent` и `ScreenId`, то есть чем заполнена основная область, и `over_main_area`. Импорты рабочего кода идут в одну сторону: `view.rs` знает про `screens/`, а `screens/` про `view.rs` не знает. Тесты экранов рисуют всё окно корневой view, а их общие помощники лежат в `testing/window.rs`.

Оверлей, который открывается из панелей одного режима, лежит в `overlays` этого режима. Например, контекстное меню дерева Launches открывается только из левой панели Terminal, а меню New session и Launches только из заголовков разделов Sessions и Launches в этой же панели.

`view.rs` это корневая view окна. Она оставляет себе:
- рамку окна: заголовок, панели и место для статус-бара;
- то, что общее у обоих режимов: меню всех режимов, палитру команд, список уведомлений, Settings, окно пароля SSH, заметку режима фокуса, тосты и полосы по краям окна, за которые меняют его размер;
- `match` по `Mode`;
- порядок слоёв.

Слои снизу вверх одни и те же в обоих режимах:
1. содержимое окна: заголовок, панели и статус-бар; заметка режима фокуса и тосты лежат над панелями;
2. оверлеи режима из `overlays`;
3. меню всех режимов;
4. палитра команд;
5. список уведомлений;
6. Settings;
7. окно пароля SSH, всегда сверху.

Над ними лежит слой с полосами для изменения размера окна. Пока открыто меню, Settings, окно пароля SSH или оверлей Containers, этот слой пуст.

Каждый слой занимает всё окно. Меню строки Containers открывается у указателя в координатах окна, поэтому `containers::overlays` кладёт его и диалоги прямо на слой. Остальные оверлеи получают координаты основной области, поэтому `view.rs` и `terminal::overlays` кладут их в основную область через `over_main_area`. Отступы сверху и снизу, на высоту заголовка и статус-бара, даёт `layout::main_area_padding`.

## Большие виджеты

Если у куска виджета есть своё состояние и свои интенты, он становится *частью*: папкой `x/<part>/`. `<part>` и `<Part>` это имя части в snake_case и в CamelCase. Одно лишь число строк не повод для разделения. В этом документе слово «часть» значит только это.

Например, у `containers` четыре части в `app/src/widgets/containers/`:
- `create`: диалоги Pull image, Create volume и Create network;
- `run`: форма Run container;
- `actions`: меню строки, подтверждение и Prune;
- `inspector`: вкладки правой панели.

Ниже примеры взяты из `run`.

**Файлы.** У части те же файлы, что у виджета: `mod.rs`, `event.rs`, `state.rs`, `reducer.rs`, `model.rs` или папка `model/` и `view/`. Ещё она может добавить `types.rs`, `subscription.rs` и файлы по темам, например `run/checks.rs`. У части нет `XWidget`, своего маршрута и `<Part>Event`.

**Что видно снаружи.**
- Родитель объявляет часть как `pub(crate) mod <part>;`. Внутри части действует та же таблица видимости, что у виджета. Например, у `run` открыты `event`, `types` и `view`, а остальные модули закрыты. Отличие одно: часть не открывает `storage` и `services`, потому что у неё нет своего маршрута, а ввод-вывод выполняет маршрут родителя.
- `mod.rs` части объявляет модули и через `pub(super) use` реэкспортирует родителю то, чем он пользуется: состояние, `reduce`, Ctx, итог и view model. Если родителю нужна функция из подписки или из файла по теме, её реэкспортируют так же.

```rust
// widgets/containers/run/mod.rs
pub(crate) mod event;
pub(crate) mod types;
pub(crate) mod view;

mod checks;
mod model;
mod reducer;
mod state;

pub(super) use model::{RunFormViewModel, RunPanelViewModel};
pub(super) use reducer::{RunCtx, RunEngine, RunOutcome, reduce};
pub(super) use state::RunState;
```

- Rust не даёт реэкспортировать элемент шире, чем он объявлен. Поэтому внутри части `reduce`, `from_state` и типы, которые получает родитель, объявлены `pub(crate)`, а не `pub(super)`. С `pub(super)` их видела бы только сама часть, и `pub(super) use` в её `mod.rs` не скомпилировался бы. Дальше родителя они всё равно не видны, потому что их модули закрыты.
- Методы состояния части, которые его меняют, объявлены `pub(super)` и видны только внутри части. Геттеры объявлены `pub(crate)`: через них родитель читает состояние части.
- Закрытые типы части, например `RunForm` из `run/state.rs`, нельзя назвать снаружи части, даже в тестах родителя. Такой тест читает часть через геттеры, не называя этих типов, или проверяет её view model, например `FormDialogViewModel::from_state(state.create())`.

**События.** Интент родителя оборачивает интент части, а эффект родителя оборачивает эффект части: `ContainersIntent::Run(RunIntent)` и `ContainersEffect::Run(RunEffect)`. Поэтому `AppEvent` и маршрут видят только родителя. Маршрут остаётся одним файлом, и эффекты каждой части в нём разбирает своя приватная функция. Результат фоновой работы маршрут возвращает интентом части внутри интента родителя, например `ContainersIntent::Run(RunIntent::Finished { .. })`.

**Редьюсер.** Сверху `reducer.rs` части стоит тот же `#![forbid(clippy::disallowed_methods)]`. `reduce` части получает состояние, интент и, если нужно, Ctx. Он возвращает эффекты и, если нужно, *итог* (`<Part>Outcome`):

```rust
// widgets/containers/run/reducer.rs
pub(crate) fn reduce(
    state: &mut RunState,
    intent: RunIntent,
    ctx: &RunCtx<'_>,
) -> (Task<RunEffect>, Option<RunOutcome>)
```

- Эффект это просьба к внешнему миру, например `RunEffect::RunContainer`: создать и запустить контейнер.
- Итог это сообщение родителю. Например, `RunOutcome::Started { engine, id }` значит, что контейнер запущен и форма закрылась. Родитель обрабатывает итог в том же обновлении.

**Данные родителя.** Всё, что часть читает у родителя, лежит в её Ctx. Ctx части объявлен в её `reducer.rs`. Например, `RunCtx` несёт найденные движки с их сетями, включён ли SELinux и домашнюю папку. Тип, который нужен двум частям, лежит в `types.rs` родителя. Например, `OpenList`, открытый список с выбранной строкой, входит и в `ActionsCtx`, и в `InspectorCtx`.

Состояние части лежит полем в состоянии родителя. Rust разрешает взять поле части для изменения и в то же время читать другие поля только там, где эти поля видны, то есть в `state.rs` родителя. Поэтому часть выдают методы состояния родителя:

| Метод в `state.rs` родителя | Кто его вызывает | Что он возвращает |
| --- | --- | --- |
| `<part>_and_ctx(&mut self)` | `reducer.rs` родителя, если у части есть Ctx | часть для изменения и её Ctx, например `run_and_ctx() -> (&mut RunState, RunCtx<'_>)` |
| `<part>_mut(&mut self)` | `reducer.rs` родителя, если у части нет Ctx | часть для изменения, например `create_mut()` |
| `<part>(&self)` и `<part>_ctx(&self)` | view model и код, который только читает часть | часть и её Ctx для чтения, например `run()` и `run_ctx()` |

Для каждой части в `reducer.rs` родителя есть три функции: `reduce_<part>` передаёт интент части, `on_<part>_outcome` обрабатывает её итог, а `wrap_<part>_effect` оборачивает эффект части в событие родителя. Эффект части выполняет функция `<part>_effect` в маршруте родителя, поэтому имена не совпадают.

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

**Пример: пользователь нажимает Run.** Форма Run container открыта, и образ уже есть на компьютере.

| Проход | Событие | Кто обрабатывает | Что происходит |
| --- | --- | --- | --- |
| 1 | `RunIntent::Submitted` от `run/view/run_form.rs`; `screens/containers.rs` оборачивает его в `ContainersIntent::Run` | `reduce_run` в `widgets/containers/reducer.rs`, затем `run::reduce` | форма проверяет поля и ждёт запуска; часть возвращает `RunEffect::RunContainer`, а родитель оборачивает его в `ContainersEffect::Run` |
| 2 | `ContainersEffect::Run(RunEffect::RunContainer { .. })` | `router/containers.rs` | запускает контейнер в фоне через `Task::perform` |
| 3 | `ContainersIntent::Run(RunIntent::Finished { .. })` с id нового контейнера | `reduce_run`, затем `run::reduce` | форма закрывается, часть возвращает итог `RunOutcome::Started { engine, id }`. В том же обновлении `on_run_outcome` выделяет новый контейнер в таблице и возвращает `ContainersEffect::LoadLists` |
| 4 | `ContainersEffect::LoadLists(..)` | `router/containers.rs` | загружает списки движка в фоне |
| 5 | `ContainersIntent::ListsLoaded { .. }` | `widgets/containers/reducer.rs` | новый контейнер появляется в таблице уже выделенным |

**Кто меняет часть.** Родитель меняет часть только через её интенты и её `reduce`, в том числе для своих команд. Например:
- когда пользователь открывает другой список, родитель отправляет форме `RunIntent::Close`, а не очищает её состояние сам;
- после каждого своего интента родитель отправляет инспектору `InspectorIntent::FollowSelection`, и инспектор сам решает, чьи логи читать и что загрузить.

`reduce` части вызывает только `reducer.rs` родителя.

**Подписка части.** Если части нужен долгоживущий фоновый поток, он лежит в её `subscription.rs` и отдаёт интенты части. Родитель переводит этот поток в свои интенты в своём `subscription.rs`. Например, `inspector::followed_logs` возвращает `Subscription<InspectorIntent>`, а родитель переводит его в `ContainersIntent::Inspector`.

**Границы.** Часть импортирует свои файлы, `types`, `errors` и файлы по темам родителя, а также общие модули. Она никогда не импортирует состояние, редьюсер, события, view model, подписку, view, `services` и `storage` родителя, то, что объявлено в его `mod.rs`, и соседние части. До соседа часть доходит через итог: родитель превращает его в интент соседа. Например, Run на строке образа даёт итог `ActionsOutcome::RunRequested`, и родитель отправляет форме `RunIntent::Open`.

**View model и view.**
- View model части строится в её `model.rs` через `from_state(state, ctx)`, а у части без Ctx через `from_state(state)`. Её строит метод `XWidget::<view>_vm()`, например `ContainersWidget::run_form_vm()` вызывает `RunFormViewModel::from_state(self.state.run(), &self.state.run_ctx())`. Если view model части входит в view model родителя, её строит `model.rs` родителя.
- View части возвращает `Element<'_, RunIntent>`. Тот, кто её показывает, переводит элемент в интент родителя: `.map(ContainersIntent::Run)`. Экран, который собирает несколько частей, это view родителя: она переводит элемент каждой части в интент родителя.

## Куда писать новый код

| Задача | Что делать |
| --- | --- |
| Новая кнопка или поле | вариант `XIntent`, ветка `reduce`, поле view model, `view/`; `guards.rs`, если при открытом меню нужно особое поведение |
| Прочитать или записать что-то вне приложения | вариант `XEffect`, `services.rs` или `storage.rs`, ветка в `router/x.rs`, вариант `XIntent` для результата, ветка `reduce` |
| Другой виджет должен отреагировать | вариант `XEffect` в этом виджете, ветка в `router/x.rs`, которая отправляет интент другого виджета |
| Нужны данные другого виджета | поле в `XCtx`, заполняется в `router/x.rs` |
| Сообщить пользователю об ошибке | шаблон `Tpl...Failed` с `{error}` в `i18n/key.rs` и файлах локалей; маршрут вызывает `notifications::push(i18n::with_error(..))` из `router/notifications.rs`. Если результат приходит интентом виджета, решает редьюсер: он возвращает эффект, а маршрут вызывает `push`. Если результат никуда не приходит, маршрут разбирает его сам, как `pinned_saved` в `router/modes.rs`. См. [«Пример: сообщить пользователю об ошибке»](#пример-сообщить-пользователю-об-ошибке) |
| Действие, которое можно вызвать шорткатом, из палитры или кнопкой шапки | вариант `AppCommand` и ветка в `router/command.rs`; правило в `AppCommand::is_available`, если команда доступна не всегда. Для шортката: вариант `AppShortcut`, клавиши в `AppShortcut::keys` и строка в `AppShortcut::command`. Для палитры: строка в `action_items` в `widgets/palette/items.rs`. Для кнопки: эффект `chrome` и строка в `router/chrome.rs` |
| Новый текст | `i18n/key.rs` и файлы локалей |
| Новый элемент интерфейса | нужен широко: `otty-ui-kit`; нескольким виджетам: `components/`; одному: его `view/` |
| Новый режим | вариант `Mode` в `domain/`, один файл в `screens/` |
| Новый виджет | запустить `cargo xtask new-widget <name>`, которая создаёт файлы как в [«Примере виджета»](#пример-виджета), и затем `cargo +nightly fmt`, потому что генератор не переносит длинные строки; потом показать виджет в `view.rs`, если он виден во всех режимах, или в файле режима в `screens/`, и при необходимости добавить подписку в `subscription.rs` |
| Новая часть большого виджета | папка `x/<part>/` с файлами части; вариант в `XIntent` и `XEffect` родителя; `<part>_and_ctx` или `<part>_mut` в `state.rs` родителя; `reduce_<part>`, `on_<part>_outcome` и `wrap_<part>_effect` в `reducer.rs` родителя; функция `<part>_effect` для эффектов части в `router/x.rs`. См. [«Большие виджеты»](#большие-виджеты) |

## Как правила проверяются

1. **Компилятор.** Приватные модули закрывают внутренности виджета. Виджеты и маршруты не видят `Message`, поэтому не могут отметить событие как ввод.
2. **Clippy.** Корневой `clippy.toml` через `disallowed-methods` запрещает `Task::perform`, `Task::future`, `Task::run`, `Task::stream`, чтение и запись файлов и запуск процессов.
   - Это правило `app`. Библиотека в `crates/`, которой такие вызовы нужны, отключает его в своём `Cargo.toml`: `disallowed_methods = "allow"` в секции `[lints.clippy]`. Это действует на весь крейт, включая его `tests/`.
   - В `app` разрешение ставится на объявление модуля: `#[expect(clippy::disallowed_methods, reason = "...")]`. Оно действует и на тесты этого модуля. Если запрещённые вызовы есть только в тестах, разрешение ставится на сам `mod tests`. Если в модуле не осталось запрещённых вызовов, сборка падает.
   - Разрешены `router/`, `services.rs`, `storage.rs`, `subscription.rs`, `shell_integration.rs` и `testing/`. В остальном коде и его тестах эти вызовы запрещены. Крейт `otty-xtask` с тестом архитектуры читает файлы и отключает запрет в своём `Cargo.toml`, как библиотеки.
   - Сверху каждого `reducer.rs` стоит `#![forbid(clippy::disallowed_methods)]`. В редьюсере запрет нельзя снять даже через `#[allow]` или `#[expect]`: такой код не скомпилируется.
   - Clippy сравнивает настоящие пути, поэтому переименование через `use` его не обманет. Проверка запускается в `cargo clippy`.
3. **Тест архитектуры.** Крейт `otty-xtask` в папке `xtask/` разбирает `app/src` библиотекой `syn`, которая превращает код на Rust в дерево: модули, функции, выражения. Код под `#[cfg(test)]` и папку `testing/` он не проверяет. Он проверяет:
   - слои и направление импортов, в том числе что `services` и `storage` использует только маршрут своего виджета;
   - обязательные файлы виджета, его маршрут, имена файлов в `model/`, файлы в `router/` и файл каждого режима в `screens/`;
   - какие модули виджета открыты и что реэкспортирует `mod.rs`;
   - что лежит в файлах: импорт `state` во view, `Task` и `Element` в `state.rs` и `<topic>.rs`, первую строку `reducer.rs`, лишние типы в `event.rs`, изменение состояния в `model.rs` и `model/`, `XCtx` вне `reducer.rs`, view model вне `model.rs` и `model/`, имя параметра `intent` у `reduce`, имена параметров `intent` и `effect` в `router/`, имя `<View>Props` в файле view, `from_state` в методах `vm()` и рендерер в типе `Element`;
   - кто что вызывает: `route` вне `router/mod.rs` и `update.rs`, `reduce` виджета вне его маршрута, потоки вне `subscription.rs`, отметки `Message` вне `app.rs` и снятие запрета clippy там, где его снимать нельзя;
   - что маршрут выбирает действие по интенту или эффекту, а не по состоянию виджетов.

   У частей больших виджетов действуют те же правила, что у виджета: обязательные файлы (без маршрута), видимость модулей (часть не открывает `storage` и `services`), реэкспорты, импорт `state` во view, `Task` и `Element` в `state.rs` и файлах по темам, имя `<View>Props` и view model только в `model.rs` или `model/`. Два правила есть только у частей: `part-imports-outside` (часть не импортирует родителя и соседей, кроме `types`, `errors` и файлов по темам родителя) и `part-reduce-outside-parent` (`reduce` части вызывает только `reducer.rs` родителя).

   У каждого правила есть короткое имя, например `view-imports-state`, и описание в одно предложение. Они лежат в `xtask/src/architecture/finding.rs`, и отчёт печатает их над местами, которые нарушают правило.
   - Тест запускается в `cargo test`, поэтому pre-commit и CI уже его покрывают. Он не пропускает новые нарушения и устаревшие записи.
   - `cargo xtask arch` печатает отчёт для чтения. В первой строке сводка, затем то, что требует действий (новые нарушения и устаревшие записи), таблица по правилам, все известные нарушения и исключения, сгруппированные по правилам, с файлом, строкой и причиной, и в конце что делать дальше. Команда завершается с кодом 1, если есть новое или устаревшее. Если тест упал, он печатает тот же отчёт, поэтому у нового нарушения есть файл, строка, описание правила и готовая строка для списка исключений.
   - Все исключения из правил этого документа записаны в тесте, в `xtask/src/architecture/allowlist.rs`, у каждого указана причина. Постоянное исключение помечено `Exception`, нарушение, которое надо исправить, помечено `KnownViolation`. Тест падает и на новом нарушении, и на записи, которая больше ничему не соответствует, поэтому список не устаревает.
   - Тест видит дерево кода, но не знает, какое имя к чему относится: он не следит за значениями между функциями. Если чтение состояния и ветвление разнести по двум функциям, чтобы обойти проверку маршрута, это заметят на ревью, а не в тесте.
4. **Генератор.** `cargo xtask new-widget <name>` создаёт обязательные файлы виджета, включая `#![forbid]` в `reducer.rs`, и его маршрут `router/<name>.rs` и подключает их в `widgets/mod.rs`, `router/mod.rs` и `app.rs`. После этого проект компилируется, но виджет ещё нигде не показан, поэтому компилятор предупреждает о неиспользуемом коде, а `cargo clippy -D warnings`, pre-commit и CI падают, пока виджет не показан и у него нет своих интентов и эффектов. Тест в `xtask/tests/new_widget.rs` создаёт виджет в копии `app/src` и проверяет её тестом архитектуры: в копии не должно быть ни новых нарушений, ни устаревших записей.
