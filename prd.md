# PRD: Рефакторинг архитектуры `otty` к Clean Architecture

## 1. Контекст и проблема

`otty` уже использует feature-oriented подход (`event/state/model/feature`), но в текущем состоянии есть смешение слоев:

- `features/sidebar` зависит от UI-типов (`ui::widgets::*`), что связывает доменный слой с presentation.
- `ui/components/resize_grips.rs` зависит от `crate::app::Event`.
- Оркестрация сценариев распределена между `app/update` и крупными `feature.rs` (особенно `quick_launch`), что снижает читаемость.
- В domain-state присутствуют типы `iced`/`otty_ui_term`, затрудняя изоляцию бизнес-логики.

Это увеличивает стоимость изменений, усложняет тестирование и мешает масштабированию функционала.

## 2. Цель

Сделать архитектуру читаемой, расширяемой и предсказуемой за счет явного разделения на слои:

- `presentation`
- `application`
- `domain`
- `infrastructure`

и однонаправленных зависимостей от внешнего к внутреннему слою.

## 3. Non-goals

- Полный редизайн UI.
- Изменение UX/поведения существующих фич.
- Переписывание crates `otty-*` (низкоуровневый terminal stack).

## 4. Требования

### 4.1 Функциональные

1. Поведение приложения до и после рефакторинга должно быть эквивалентно.
2. Существующие пользовательские сценарии (tabs, terminal, quick launch, explorer, settings) должны работать без регрессий.
3. Все кросс-feature эффекты должны проходить через application orchestration.

### 4.2 Архитектурные

1. `domain` не импортирует `ui`, `app`, `iced::widget::*`, `Task`.
2. `presentation` не содержит бизнес-правил и не выполняет I/O.
3. `infrastructure` реализует порты (`trait`) для I/O и runtime интеграций.
4. UI-события мапятся в application-команды, а не напрямую в domain-объекты других слоев.

### 4.3 Качественные

1. Снижение среднего размера `feature.rs` за счет выделения use-case модулей.
2. Повышение тестопригодности: unit-тесты domain/use-case без UI runtime.
3. Явная схема зависимостей, проверяемая линтами/тестами.

## 5. Целевая архитектура

## 5.1 Слои

1. `presentation` (`ui`, `view`, `subscription`)
- Рендеринг.
- Mapping локальных UI events -> application commands.
- Без доменной мутации и без I/O.

2. `application` (`app/update`, use-cases, coordinators)
- Оркестрация фич.
- Выполнение сценариев и сбор эффектов.
- Конвертация `Effect` -> `iced::Task`.

3. `domain` (`features/*/model`, `features/*/state`, domain events/rules)
- Чистые инварианты, состояние, правила.
- Без `Task`, без прямых вызовов UI/FS/PTY.

4. `infrastructure` (`features/*/services`, `features/*/storage`, runtime adapters)
- Файловая система, env, PTY/SSH, внешние процессы.
- Реализации портов для application/domain.

## 5.2 Правило зависимостей

Допустимое направление:

`presentation -> application -> domain <- infrastructure`

Запрещено:

- `domain -> presentation`
- `domain -> infrastructure` (напрямую, без портов)
- `components -> app::Event`

## 5.3 Референс-структура каталогов (целевая)

```text
otty/src/
  app/
    mod.rs
    event.rs
    update.rs
    commands.rs
    effects.rs
  presentation/
    view.rs
    subscription.rs
    ui/
      components/
      widgets/
  domain/
    sidebar/
      model.rs
      state.rs
      event.rs
      rules.rs
    quick_launch/
      model.rs
      state.rs
      event.rs
      rules.rs
  application/
    sidebar/
      reducer.rs
      mapper.rs
    quick_launch/
      reducer.rs
      use_cases/
        drag_drop.rs
        inline_edit.rs
        launch.rs
        persistence.rs
  infrastructure/
    quick_launch/
      storage.rs
      runtime.rs
    settings/
      storage.rs
```

Примечание: допускается эволюционный путь без мгновенного физического переноса всех папок, но логические границы слоев должны быть введены сразу.

## 6. Ключевые изменения по модулям

1. `sidebar`
- Убрать зависимость `features/sidebar` от `ui::widgets::sidebar_*`.
- Ввести доменный `SidebarEvent` (без UI-типов).
- Mapping `SidebarMenuEvent`/`SidebarWorkspaceEvent` выполнять в application/presentation.

2. `resize_grips`
- Вернуть локальный `ResizeGripEvent`.
- Преобразование в `App::Event::ResizeWindow` делать в `view`.

3. `quick_launch`
- Разбить `feature.rs` на use-cases:
  - `interaction`
  - `drag_drop`
  - `inline_edit`
  - `launch`
  - `persistence`
- Ввести унифицированный `Effect` для domain/application, `Task` собирать только в application.

4. `terminal` и `quick_launch state`
- Поэтапно удалить `iced`- и widget-specific типы из domain state.
- UI/runtime идентификаторы хранить в application/runtime adapters.

## 7. Примеры (до/после)

## 7.1 Пример: событие sidebar

До:

```rust
pub(crate) enum SidebarEvent {
    Menu(sidebar_menu::SidebarMenuEvent),
    Workspace(sidebar_workspace::SidebarWorkspaceEvent),
    ToggleVisibility,
}
```

После:

```rust
pub(crate) enum SidebarEvent {
    SelectTerminal,
    SelectExplorer,
    OpenSettings,
    ToggleWorkspace,
    ToggleVisibility,
    OpenAddMenu,
    DismissAddMenu,
    AddMenuCreateTab,
    AddMenuCreateCommand,
    AddMenuCreateFolder,
}
```

Mapping (application/presentation):

```rust
fn map_sidebar_menu_event(event: SidebarMenuEvent) -> SidebarEvent {
    match event {
        SidebarMenuEvent::SelectItem(SidebarMenuItem::Terminal) => SidebarEvent::SelectTerminal,
        SidebarMenuEvent::SelectItem(SidebarMenuItem::Explorer) => SidebarEvent::SelectExplorer,
        SidebarMenuEvent::OpenSettings => SidebarEvent::OpenSettings,
        SidebarMenuEvent::ToggleWorkspace => SidebarEvent::ToggleWorkspace,
        SidebarMenuEvent::Resized(_) => SidebarEvent::ToggleWorkspace, // пример, реальный mapping зависит от политики resize
    }
}
```

## 7.2 Пример: `resize_grips`

До:

```rust
use crate::app::Event;
pub(crate) fn view() -> Element<'static, Event, Theme, iced::Renderer> { ... }
```

После:

```rust
pub(crate) enum ResizeGripEvent {
    Resize(iced::window::Direction),
}

pub(crate) fn view() -> Element<'static, ResizeGripEvent, Theme, iced::Renderer> { ... }
```

Mapping в `view.rs`:

```rust
resize_grips::view().map(|event| match event {
    ResizeGripEvent::Resize(direction) => Event::ResizeWindow(direction),
})
```

## 7.3 Пример: единый `Effect`

```rust
pub(crate) enum Effect {
    None,
    OpenTab { kind: TabKind },
    SyncExplorer,
    PersistQuickLaunches,
    FocusWidget { id: String },
}
```

`application` преобразует `Effect` в `Task<AppEvent>`, а `domain` возвращает только `Effect`.

## 8. План внедрения

1. Phase 1: Декуплинг UI типов
- `sidebar event` отделяется от widget events.
- `resize_grips` отвязывается от `App::Event`.

2. Phase 2: Выделение application use-cases
- Разбиение `quick_launch/feature.rs`.
- Явный слой `application` для orchestration policy.

3. Phase 3: Effect pipeline
- Внедрение `Effect`.
- Централизация `Task` сборки в application.

4. Phase 4: Очистка domain/runtime boundaries
- Вынос runtime-специфичных данных из domain state.
- Замена прямых вызовов infrastructure на порты.

5. Phase 5: Hardening
- Архитектурные тесты на недопустимые зависимости.
- Документация и migration notes для команды.

## 9. Критерии приемки

1. `sidebar` и `resize_grips` не зависят от `crate::app::Event` и UI-типов между слоями.
2. `quick_launch/feature.rs` разбит на подмодули use-cases, размер основного reducer-файла существенно уменьшен.
3. Domain-слой не использует `Task` и не вызывает I/O напрямую.
4. Все проверки проекта проходят:
- `cargo +nightly fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo deny check`
- `cargo test --workspace --all-features`
- `cargo llvm-cov --workspace --all-features --fail-under-lines 80`

## 10. Риски и смягчение

1. Риск: регрессии поведения из-за перепривязки событий.
- Смягчение: snapshot-тесты event mapping + интеграционные сценарии tab/quick-launch.

2. Риск: затяжной рефакторинг без value delivery.
- Смягчение: поэтапные PR с обратной совместимостью и feature flags (при необходимости).

3. Риск: дублирование типов в переходный период.
- Смягчение: временные adapters с четким сроком удаления и TODO с owner/date.

## 11. Метрики успеха

1. Средний размер `feature.rs` у top-3 сложных фич уменьшен минимум на 40%.
2. Количество прямых импортов UI-типов в `features/*` = 0 (кроме специально оговоренных transitional adapters).
3. Доля pure unit-тестов в domain/application слоях увеличена.
4. Время онбординга нового разработчика на изменение фичи сокращено (оценка команды после релиза фазы 2).

