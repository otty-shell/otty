# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0]

### Added

- Improvements of text rendering (bold, italic, underlined, etc).
- Batch background rendering
- Shell arguments (thanks [@fdavid-spk](https://github.com/fdavid-spk))
- CHANGELOG.md
- `msgcat --color=test` [results](./docs/colortest)

### Changed

- (**breaking changes**) `iced_term::Event::BackendCommandReceived` -> `iced_term::Event::BackendCall`
- (**breaking changes**) `iced_term::Subscription` -> `Terminal's` method `subscription`
- (**breaking changes**) `iced_term::Command::ProcessBackendCommand` -> `iced_term::Command::ProxyToBackend`
- Upgrade `alacritty_terminal` to `0.25.0`
- Upgrade `tokio` to `1.47.1`
- Upgrade `anyhow` to `1.0.99`
- Upgrade `open` to `5.3.2`

### Fixed

- Initial font loading
- Showing cursor only if terminal mode is `SHOW_CURSOR`
- Examples using the new widget api
- Typos in README.md (thanks [@kxxt](https://github.com/kxxt))

### Removed

- (**breaking changes**) `iced_term::Subscription`

[unreleased]: https://github.com/Harzu/iced_term/compare/0.6.0...HEAD
[0.6.0]: https://github.com/Harzu/iced_term/compare/0.5.1...0.6.0
