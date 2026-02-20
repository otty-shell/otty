# Рефакторинг `otty/src`

## 1. Цель рефакторинга

Сделать кодовую базу предсказуемой и однообразной внутри каждого слоя:

1. Минимизировать дублирование.
2. Упростить чтение и поддержку.
3. Устранить архитектурные узкие места и скрытые баги.
4. Снизить лишние аллокации и линейные обходы в hot-path.
5. Ввести единые паттерны для `App -> Features -> UI`.

Ключевой принцип: если поведение похоже, оно должно быть вынесено в общее решение. Различаться должна только бизнес-логика.

---

## 2. Целевая модель слоев

### 2.1 `app` слой (оркестрация)

Назначение:

1. Роутинг событий между фичами.
2. Координация межфичевых побочных эффектов.
3. Никакой доменной логики внутри `App::update`.

Правило:

1. `App` не должен знать внутренние детали quick-commands/terminal/explorer.

### 2.2 `features` слой (домен)

Назначение:

1. Чистые редьюсеры состояния.
2. Минимум `Task`-деталей в доменных функциях.
3. Валидация и инварианты в одном месте.

Правило:

1. Одна доменная операция = один публичный entrypoint + приватные helper-ы.

### 2.3 `ui` слой (представление)

Назначение:

1. Декларативная сборка виджетов.
2. Переиспользуемые стили и layout-примитивы.
3. Никакой доменной логики кроме mapping UI-event -> feature-event.

Правило:

1. Одинаковые меню/деревья/скроллы строятся через единые утилиты.

---

## 3. Единый стиль кода по слоям

### 3.1 Сигнатуры редьюсеров

```rust
pub(crate) fn reduce(
    state: &mut State,
    deps: &FeatureDeps,
    event: Event,
) -> Task<AppEvent>
```

### 3.2 Единый layout для UI-компонентов

```rust
pub(crate) struct Props<'a> {
    pub(crate) theme: ThemeProps<'a>,
    // только данные, нужные view
}

pub(crate) fn view<'a>(props: Props<'a>) -> Element<'a, Event> {
    // layout only
}
```

### 3.3 Единые naming-правила

1. Имя события должно отражать реальный эффект (`MinimizeWindow`, а не `ToggleTray`).
2. Не использовать `_name` для активных API-элементов (`_set_current`, `_font_type`).
3. Избегать слов `by_id`, если параметр не `id` (`close_pane_by_id` -> `close_pane`).

---

## 4. Проблемы и план исправления

## R01. `panic` в рендере терминала

Категория: архитектура, надежность  
Где: `otty/src/ui/widgets/terminal/view.rs:94`

Проблема:

1. `expect("terminal missing for pane")` может уронить приложение в runtime.

Шаги исправления:

1. Убрать `expect`.
2. При рассинхроне логировать предупреждение.
3. Отрисовывать нейтральную заглушку.

Пример:

```rust
// before
let terminal_entry = terminals.get(&terminal_id).expect("terminal missing for pane");

// after
let Some(terminal_entry) = terminals.get(&terminal_id) else {
    log::warn!("pane-terminal mismatch: terminal_id={terminal_id}");
    return container(text("Terminal unavailable")).into();
};
```

---

## R02. Потеря launch-статуса после rename quick-command

Категория: архитектура, корректность  
Где: `otty/src/features/quick_commands/event.rs:508`, `otty/src/features/quick_commands/event.rs:753`

Проблема:

1. При rename обновляется дерево, но не обновляется `launching` map по пути.
2. После rename могут пропадать `Kill`/индикатор запуска.

Шаги исправления:

1. Вынести общее обновление путей запуска в отдельную функцию.
2. Вызывать ее и при `rename`, и при `move`.

Пример:

```rust
fn rename_node(state: &mut State, old_path: NodePath, new_title: String) {
    let mut new_path = old_path.clone();
    if let Some(last) = new_path.last_mut() {
        *last = new_title;
    }
    reindex_launching_paths(state, &old_path, &new_path);
}
```

---

## R03. `Arc<Mutex<Option<TerminalState>>>` в результате async-launch

Категория: архитектура  
Где: `otty/src/features/quick_commands/event.rs:934`, `otty/src/features/quick_commands/event.rs:947`

Проблема:

1. Избыточная синхронизация для single-thread UI event flow.
2. Сложно читать и поддерживать (`lock`/`take`).

Шаги исправления:

1. Возвращать из `Task::perform` DTO, а создание `TerminalState` оставлять в reducer.
2. Либо возвращать готовый `TerminalState` без `Mutex`.

Пример:

```rust
#[derive(Clone, Debug)]
pub(crate) struct LaunchPrepared {
    pub(crate) path: NodePath,
    pub(crate) launch_id: u64,
    pub(crate) tab_id: u64,
    pub(crate) terminal_id: u64,
    pub(crate) title: String,
    pub(crate) session: SessionKind,
}
```

---

## R04. Неверный парсинг editor command

Категория: архитектура, корректность  
Где: `otty/src/features/explorer/event.rs:174`

Проблема:

1. `split_whitespace` ломает кавычки и escape-последовательности.

Шаги исправления:

1. Использовать shell-aware парсер (`shell_words`).
2. При ошибке парсинга показывать user-friendly ошибку.

Пример:

```rust
fn parse_command_line(input: &str) -> Result<(String, Vec<String>), String> {
    let parts = shell_words::split(input).map_err(|err| format!("{err}"))?;
    let Some((program, args)) = parts.split_first() else {
        return Err(String::from("Editor command is empty"));
    };
    Ok((program.clone(), args.to_vec()))
}
```

---

## R05. Монолитные редьюсеры (`App::update`, `quick_commands_reducer`)

Категория: архитектура  
Где: `otty/src/app.rs:164`, `otty/src/features/quick_commands/event.rs:118`

Проблема:

1. Слишком много веток в одном месте.
2. Трудно добавлять функциональность без регрессий.

Шаги исправления:

1. Разбить на под-редьюсеры по зонам (`window`, `sidebar`, `menus`, `tabs`).
2. Ввести `FeatureDeps` и убрать прямые вызовы чужой внутренней логики.
3. Сохранить единый контракт редьюсера.

Пример:

```rust
match event {
    Event::Window(event) => window_reducer::reduce(state, event),
    Event::Sidebar(event) => sidebar_reducer::reduce(state, deps, event),
    Event::Tab(event) => tab_reducer::reduce(state, deps, event),
    Event::QuickCommands(event) => quick_commands_reducer::reduce(state, deps, event),
}
```

---

## R06. Дубли валидации/персиста quick-commands

Категория: повторы, архитектура  
Где: `otty/src/features/quick_commands/event.rs:550`, `otty/src/features/quick_commands/editor.rs:403`, `otty/src/features/quick_commands/event.rs:883`, `otty/src/features/quick_commands/editor.rs:422`

Проблема:

1. Одинаковые правила в разных модулях расходятся со временем.
2. Разные сообщения об ошибках для одного доменного ограничения.

Шаги исправления:

1. Вынести доменные операции в `features/quick_commands/domain.rs`.
2. Сделать один `save(data) -> Result<(), QuickCommandsError>`.
3. Возвращать typed ошибки (без `String` как primary error type).

Пример:

```rust
#[derive(Debug, thiserror::Error)]
pub(crate) enum QuickCommandDomainError {
    #[error("title cannot be empty")]
    EmptyTitle,
    #[error("title already exists")]
    DuplicateTitle,
}

pub(crate) fn validate_title(...) -> Result<String, QuickCommandDomainError> { ... }
```

---

## R07. Дубли создания terminal tab

Категория: повторы, архитектура  
Где: `otty/src/features/terminal/event.rs:163`, `otty/src/features/explorer/event.rs:98`, `otty/src/features/quick_commands/event.rs:257`

Проблема:

1. Похожая последовательность (id -> new terminal -> insert tab -> focus/sync) повторяется.

Шаги исправления:

1. Вынести `TabFactory`/`TabService`.
2. Использовать единый путь для создания terminal tabs.

Пример:

```rust
pub(crate) fn open_terminal_tab(
    state: &mut State,
    request: OpenTerminalRequest,
) -> Task<AppEvent> {
    // единый код создания, вставки, фокуса и sync
}
```

---

## R08. Дубли контекстных меню

Категория: повторы  
Где: `otty/src/ui/widgets/quick_commands/context_menu.rs`, `otty/src/ui/widgets/sidebar_workspace/add_menu.rs`, `otty/src/ui/widgets/terminal/pane_context_menu.rs`

Проблема:

1. Почти одинаковый код для anchor/panel/dismiss/menu-items.

Шаги исправления:

1. Вынести общий `ui/components/context_menu.rs`.
2. Передавать только `items`, `position`, `on_dismiss`.

Пример:

```rust
pub(crate) struct ContextMenuItem<Message> {
    pub(crate) label: &'static str,
    pub(crate) on_press: Message,
}

pub(crate) fn context_menu_view<'a, Message: Clone + 'a>(
    props: ContextMenuProps<'a, Message>,
) -> Element<'a, Message> { ... }
```

---

## R09. Дубли `scrollable` и `tree row` стилей

Категория: повторы  
Где: `otty/src/ui/widgets/settings.rs`, `otty/src/ui/widgets/sidebar_workspace/explorer.rs`, `otty/src/ui/widgets/quick_commands/sidebar.rs`

Проблема:

1. Один и тот же стиль скролла и выделения строки размазан по трем виджетам.

Шаги исправления:

1. Вынести в `ui/style/scroll.rs` и `ui/style/tree.rs`.
2. Использовать единые функции в каждом дереве.

Пример:

```rust
let scroll = scrollable(content).style(ui::style::scroll::thin(theme));
let row_style = ui::style::tree::row(theme, context.is_selected, context.is_hovered);
```

---

## R10. Лишние аллокации в tab bar

Категория: неэффективный код  
Где: `otty/src/state.rs:154`, `otty/src/app.rs:348`, `otty/src/ui/widgets/tab_bar.rs:27`

Проблема:

1. Клонирование заголовков табов при каждом рендере.

Шаги исправления:

1. Передавать срез ссылок вместо `Vec<(u64, String)>`.
2. `tab_button` принимать `&str`, а не `String`.

Пример:

```rust
pub(crate) struct TabSummary<'a> {
    pub(crate) id: u64,
    pub(crate) title: &'a str,
}
```

---

## R11. Линейный поиск `tab_id_by_terminal`

Категория: неэффективный код  
Где: `otty/src/features/terminal/event.rs:460`

Проблема:

1. На каждый terminal event выполняется линейный обход вкладок.

Шаги исправления:

1. Добавить индекс `terminal_to_tab: HashMap<u64, u64>` в `State`.
2. Обновлять индекс при create/split/close tab/pane.

Пример:

```rust
if let Some(&tab_id) = state.terminal_to_tab.get(&terminal_id) {
    // O(1)
}
```

---

## R12. Лишние аллокации при сортировке explorer

Категория: неэффективный код  
Где: `otty/src/features/explorer/state.rs:170`

Проблема:

1. `to_lowercase()` на каждое сравнение создает временные строки.

Шаги исправления:

1. Перейти на `cmp_ignore_ascii_case` (если достаточно ASCII).
2. При необходимости Unicode сделать precomputed key у `FileNode`.

Пример:

```rust
fn compare_names(left: &str, right: &str) -> Ordering {
    match left.as_bytes().cmp_ignore_ascii_case(right.as_bytes()) {
        Ordering::Equal => left.cmp(right),
        other => other,
    }
}
```

---

## R13. Неочевидный нейминг действий

Категория: плохой нейминг  
Где: `otty/src/ui/widgets/action_bar.rs:26`, `otty/src/app.rs:507`, `otty/src/features/terminal/event.rs:283`

Проблема:

1. `ToggleTray` фактически делает minimize.
2. `close_pane_by_id` принимает `Pane`, а не id.

Шаги исправления:

1. Переименовать `ToggleTray` -> `MinimizeWindow`.
2. Переименовать `close_pane_by_id` -> `close_pane`.
3. Массово обновить call-sites.

Пример:

```rust
pub(crate) enum Event {
    MinimizeWindow,
}
```

---

## R14. Технический долг в именах и API

Категория: плохой нейминг, архитектура  
Где: `otty/src/theme.rs:270`, `otty/src/theme.rs:287`, `otty/src/fonts.rs:12`, `otty/src/ui/widgets/tab_bar.rs:189`

Проблема:

1. `_presets`, `_set_current`, `_font_type` выглядят как временные элементы.
2. Опечатка: `ELIPSIZE`.

Шаги исправления:

1. Либо удалить, либо сделать нормальный публичный API (`presets`, `set_current`).
2. Исправить опечатку в константе.
3. Прогнать rename через workspace.

Пример:

```rust
const DEFAULT_MAX_CHAR_COUNT_BEFORE_ELLIPSIZE: usize = 20;
```

---

## 5. Структурный рефакторинг файлов

Предлагаемая целевая структура:

```text
otty/src/
  app/
    mod.rs
    reducer.rs
    subscriptions.rs
  features/
    quick_commands/
      mod.rs
      reducer.rs
      domain.rs
      commands.rs
      storage.rs
    terminal/
      mod.rs
      reducer.rs
      tab_service.rs
  ui/
    components/
      context_menu.rs
      tree_view.rs
    style/
      scroll.rs
      tree.rs
      button.rs
```

---

## 6. План внедрения (итеративно, без больших PR)

## Этап 1. Безопасность и корректность

1. R01, R02, R04.
2. Добавить тесты на rename/move/launch path.
3. Добавить тесты на parsing editor command.

## Этап 2. Дедупликация домена

1. R06, R07.
2. Вынести `TabService` и domain-валидацию quick-commands.
3. Перевести `String`-ошибки на typed errors (`thiserror`).

## Этап 3. Дедупликация UI

1. R08, R09.
2. Общие context menu и tree/scroll styles.
3. Выравнивание сигнатур `Props` и `view` между виджетами.

## Этап 4. Производительность

1. R10, R11, R12.
2. Убрать лишние clone/to_string в render path.
3. Добавить индекс `terminal_id -> tab_id`.

## Этап 5. Naming cleanup

1. R13, R14.
2. Переименования + небольшие документационные правки.

---

## 7. Базовые тесты, которые нужно добавить

1. `quick_commands_rename_updates_launching_paths`.
2. `quick_commands_move_updates_launching_paths`.
3. `quick_commands_context_menu_kill_visible_for_renamed_launching_command`.
4. `parse_editor_command_handles_quoted_args`.
5. `terminal_event_lookup_uses_index`.
6. Snapshot-тесты для общего context menu layout.

---

## 8. Definition of Done

1. Нет `expect`/`unwrap` в runtime-путях UI/features.
2. Нет дублированных реализаций меню/scroll/tree styles.
3. Для каждого слоя есть единый шаблон организации кода.
4. Уменьшено количество `clone()/to_string()` в горячих местах рендера.
5. `cargo fmt`, `cargo clippy --workspace --all-targets`, `cargo test --workspace` проходят.

