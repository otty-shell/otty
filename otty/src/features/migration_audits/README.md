# Feature Migration Audits (Strict Conventions)

Актуальный порядок выполнения (обновлен по текущему состоянию кода и остаточным зависимостям):

1. `05-tab.md`
2. `02-terminal.md`
3. `06-explorer.md`
4. `01-settings.md`
5. `04-quick-launches.md`
6. `03-quick-launches-editor.md`

Логика порядка:
- сначала фикс ownership и event boundaries в `tab`;
- затем развязываем `terminal` и циклы зависимостей;
- после этого стабилизируем `explorer`, который зависит от tab/terminal orchestration;
- затем закрываем async/effect boundary для `settings`;
- в конце доводим `quick_launches` и `quick_launches/editor`, которые завязаны на tab/terminal API.

Каждый файл аудита содержит:
- текущее состояние относительно `otty/src/features/CONVENTIONS.md`;
- остаточные несоответствия с точками в коде;
- пошаговый план миграции до 100% strict;
- критерии готовности и финальную верификацию.
