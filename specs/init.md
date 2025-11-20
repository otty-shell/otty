## otty-libterm API analysis

**GOAL (what problem are we solving?):** We suspect the `otty-libterm` API is not optimal for building a terminal emulator because of:

- Excessive complexity.
- Borrowing/lifetime pain (especially around `TerminalSnapshot`).
- Needing extra glue instead of using the library’s own structs/traits.

**Outcome (what do we want?):**

1. Understand whether the current API is convenient and optimal for other developers.
    - What already works well?
    - What is weak and needs redesign?
2. Define what the ideal API should look like for building terminal emulators of any complexity.
3. Produce a step-by-step plan to move from today’s API to that ideal API, with examples that show how it will be used and what will change.

**Notes:**

- Deep conceptual rewrites of the internal libraries are undesirable if they are already correct.
- Preserve modularity: `libterm` taking traits instead of concrete impls is a big plus.
- Do not change code in this task; we need a plan. Include code examples to illustrate usage and desired changes.
- Generate this plan as the `plan.md` file.
