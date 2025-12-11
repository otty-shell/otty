# macOS упаковка `otty` в `.dmg`

Этот документ описывает, как:

- собирать бинарник `otty` для macOS (Intel и Apple Silicon);
- упаковывать его в `.app`‑bundle;
- собирать `.dmg` для обеих архитектур в GitHub Actions.

Документ оформлен как план работ с примерами кода, чтобы можно было по шагам довести проект до готовых macOS‑пакетов.

---

## 1. Цели и ограничения

- Поддерживаемые архитектуры:
  - `x86_64-apple-darwin` (Intel/macOS 13 runner);
  - `aarch64-apple-darwin` (Apple Silicon/macOS 14 runner).
- Собираем один бинарник `otty` на архитектуру.
- Укладываем бинарник в минимальный `.app`‑bundle.
- Упаковываем `.app` в `.dmg` в CI — на GitHub Actions в отдельном job, рядом с уже существующим `linux-packages`.
- Коды подписей/нотаризации **пока не настраиваем** (можно добавить отдельной задачей).

---

## 2. Структура для macOS‑пакетов

Нужно договориться о структуре для macOS‑артефактов (по аналогии с `packages/linux`):

```text
packages/
  linux/
    otty.desktop
  macos/
    Info.plist
    logo-small.icns    # иконка приложения
    make-app-bundle.sh
    make-dmg.sh
```

### Задачи

1. Создать директорию `packages/macos`.
2. Добавить минимальный `Info.plist`, описывающий приложение `otty`.
3. Добавить скрипт `make-app-bundle.sh`, который формирует `.app` из собранного бинарника.
4. Добавить скрипт `make-dmg.sh`, который упаковывает `.app` в `.dmg`.

---

## 3. Сборка бинарника `otty` для macOS в CI

### 3.1. Подготовка toolchain

На macOS‑runner’e GitHub Actions (и на любых других машинах, где будет запускаться CI‑скрипт) должны быть доступны таргеты:

```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

Если сборка ведётся прямо с соответствующей архитектуры (например, arm64‑Mac собирает `aarch64-apple-darwin`), можно добавить только нужный таргет.

### 3.2. Команды сборки

Сборка `otty` для каждой архитектуры:

```bash
# Intel
cargo build --release -p otty --target x86_64-apple-darwin

# Apple Silicon
cargo build --release -p otty --target aarch64-apple-darwin
```

Бинарники будут лежать в:

- `target/x86_64-apple-darwin/release/otty`
- `target/aarch64-apple-darwin/release/otty`

---

## 4. Создание `.app`‑bundle

### 4.1. Минимальный `Info.plist`

Файл: `packages/macos/Info.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>otty</string>
    <key>CFBundleDisplayName</key>
    <string>otty</string>
    <key>CFBundleIdentifier</key>
    <string>sh.otty.app</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>otty</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
```

На следующем шаге можно заменить версии на считываемые из `Cargo.toml` (через скрипт), но для первого прототипа фиксированные значения подходят.

### 4.2. Скрипт сборки `.app`

Файл: `packages/macos/make-app-bundle.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-aarch64-apple-darwin}" # или x86_64-apple-darwin
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${PROJECT_ROOT}/target/${ARCH}/release"
APP_NAME="otty"
APP_BUNDLE_DIR="${PROJECT_ROOT}/dist/mac/${ARCH}/${APP_NAME}.app"

echo "Building app bundle for ${ARCH}"

mkdir -p "${APP_BUNDLE_DIR}/Contents/MacOS"
mkdir -p "${APP_BUNDLE_DIR}/Contents/Resources"

cp "${TARGET_DIR}/${APP_NAME}" "${APP_BUNDLE_DIR}/Contents/MacOS/${APP_NAME}"
cp "${PROJECT_ROOT}/packages/macos/Info.plist" "${APP_BUNDLE_DIR}/Contents/Info.plist"

# Иконку можно добавить позже:
# cp "${PROJECT_ROOT}/packages/macos/icon.icns" "${APP_BUNDLE_DIR}/Contents/Resources/icon.icns"

echo "Created app bundle at: ${APP_BUNDLE_DIR}"
```

---

## 5. Создание `.dmg` в CI

Используем стандартный `hdiutil`.

Файл: `packages/macos/make-dmg.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-aarch64-apple-darwin}" # или x86_64-apple-darwin
APP_NAME="otty"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_BUNDLE_DIR="${PROJECT_ROOT}/dist/mac/${ARCH}/${APP_NAME}.app"
DMG_DIR="${PROJECT_ROOT}/dist/mac/${ARCH}"

mkdir -p "${DMG_DIR}"

DMG_NAME="${APP_NAME}-${ARCH}.dmg"
DMG_PATH="${DMG_DIR}/${DMG_NAME}"

echo "Creating DMG: ${DMG_PATH}"

hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${APP_BUNDLE_DIR}" \
  -format UDZO \
  -ov \
  "${DMG_PATH}"

echo "DMG created at: ${DMG_PATH}"
```

## 6. Интеграция в GitHub Actions

Сейчас в `.github/workflows/build-packages.yml` есть `linux-packages`. Нам нужно добавить туда новый job `macos-packages`, который:

- собирает `otty` для двух архитектур;
- формирует `.app` и `.dmg` с помощью наших скриптов;
- загружает `.dmg` как артефакты.

### 6.1. Пример job для macOS

Фрагмент для добавления в `.github/workflows/build-packages.yml`:

```yaml
  macos-packages:
    name: Build DMG (macOS)
    runs-on: ${{ matrix.runner }}

    strategy:
      matrix:
        arch: [x86_64-apple-darwin, aarch64-apple-darwin]
        include:
          - arch: x86_64-apple-darwin
            runner: macos-13
          - arch: aarch64-apple-darwin
            runner: macos-14

    steps:
      - name: Checkout sources
        uses: actions/checkout@v4

      - name: Set up Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@stable
        with:
          targets: ${{ matrix.arch }}

      - name: Build release binary
        run: cargo build --release -p otty --target ${{ matrix.arch }}

      - name: Make app bundle
        run: bash packages/macos/make-app-bundle.sh ${{ matrix.arch }}

      - name: Make DMG
        run: bash packages/macos/make-dmg.sh ${{ matrix.arch }}

      - name: Upload DMG artifact
        uses: actions/upload-artifact@v4
        with:
          name: otty-dmg-${{ matrix.arch }}
          path: dist/mac/${{ matrix.arch }}/*.dmg
```

### 6.2. Порядок действий при интеграции

1. Добавить каталог и скрипты в `packages/macos` (см. разделы выше).
2. Убедиться, что скрипты делают `mkdir -p` для всех промежуточных директорий (`dist/mac/...`).
3. Убедиться, что скрипты корректно выполняются в CI (например, через временный тестовый workflow или отдельный запуск `workflow_dispatch`).
4. После успешной проверки — добавить/обновить job `macos-packages` в `.github/workflows/build-packages.yml` (см. пример выше).
5. Запустить workflow вручную (`workflow_dispatch`) и проверить, что `.dmg` появляются в artifacts.

---

## 7. Возможные дальнейшие улучшения

- Добавить `.icns`‑иконку и прописать её в `Info.plist`.
- Автоматически подставлять версию из `Cargo.toml` в имя `.dmg` (например, `otty-0.1.0-aarch64.dmg`).
- Добавить подпись и notarization для дистрибуции через Gatekeeper.
- Собирать универсальный бинарник (fat binary) с помощью `lipo`, если это будет оправдано.

---

## 8. Чек‑лист по задаче

- [ ] Создан `packages/macos/Info.plist`.
- [ ] Добавлен скрипт `packages/macos/make-app-bundle.sh` и он успешно выполняется в CI.
- [ ] Добавлен скрипт `packages/macos/make-dmg.sh` и он успешно выполняется в CI.
- [ ] В `.github/workflows/build-packages.yml` добавлен job `macos-packages`.
- [ ] GitHub Actions собирает `.dmg` для `x86_64-apple-darwin` и `aarch64-apple-darwin`.
- [ ] Проверено, что скачанный `.dmg` монтируется и `otty.app` запускается.
