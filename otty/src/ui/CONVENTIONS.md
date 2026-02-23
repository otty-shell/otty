# UI Package Conventions

## 1. Scope

This document defines only top-level rules for `otty/src/ui` and links to layer-level conventions.

- `ui/` is an application-level presentation module.
- Detailed rules MUST be defined and maintained at layer level.
- New UI code MUST target `strict` profile in the relevant layer convention.

## 2. UI Layer Map

Detailed conventions are split by the two UI layers:

- [`components` layer conventions](./components/CONVENTIONS.md)
- [`widgets` layer conventions](./widgets/CONVENTIONS.md)

Any nested folders inside these layers MUST follow the same layer convention unless an explicit exception is documented.
