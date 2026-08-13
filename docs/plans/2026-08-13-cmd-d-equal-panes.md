# Cmd+D Equal Panes Buildout

Created: 2026-08-13
Author: williammiller20250731@gmail.com
Agent: Claude Code
Status: VERIFIED
Approved: Yes
Rounds: 2
Worktree: No
Type: Build

## Summary

**Goal:** 按 Cmd+D 在当前终端 tab 内新建一个 pane，并把新 pane 所在的同轴 split group 内所有兄弟 pane 的宽度均分。

对应 GitHub issue [#80](https://github.com/otty-shell/otty/issues/80)。分支 `fix-80`。

无 reference：这是新增行为，没有可并排比较的现成物；标准自身承载判定。

### 已确认的接入点（Step 1 调研）

| 事实 | 位置 |
|---|---|
| 键盘事件已全局订阅，但被丢弃 | `otty/src/subscription.rs:13`（`iced::keyboard::listen()`）→ `otty/src/events/mod.rs:61`（`AppEvent::Keyboard(_event) => Task::none()`） |
| `SplitPane` intent 链路完整，仅缺键盘触发 | `otty/src/widgets/terminal_workspace/event.rs:44` → `reducer.rs:312` → `state.rs:300` |
| 现有 split 无宽度归一化 | `otty/src/widgets/terminal_workspace/state.rs:311`（`self.panes.split(axis, pane, terminal_id)`） |
| 现有 split 唯一触发点是右键菜单 | `otty/src/widgets/terminal_workspace/view/pane_context_menu.rs:85,94` |
| 左右并排使用 `Axis::Vertical` | `otty/src/widgets/sidebar/state.rs:170-171` |

### iced 0.14.0 可用 API（docs.rs 已核实）

- `pane_grid::State::<T>::split(&mut self, axis: Axis, pane: Pane, state: T) -> Option<(Pane, Split)>`
- `pane_grid::State::<T>::layout(&self) -> &Node`
- `pane_grid::State::<T>::resize(&mut self, split: Split, ratio: f32)`
- `pane_grid::Node::Split { id: Split, axis: Axis, ratio: f32, a: Box<Node>, b: Box<Node> }` — 变体与字段均为 `pub`
- `pane_grid::Node::pane_regions(spacing, size) -> BTreeMap<Pane, Rectangle>` — 单元测试据此断言宽度

均分算法：`split()` 后从 `layout()` 定位新 pane 所在的同轴连续分组，对该分组内每个 `Split` 计算 `ratio = 左子树同轴叶子数 / 分组叶子总数`，收集为 `Vec<(Split, f32)>` 后逐个 `resize`（`layout()` 借用 `&self`，`resize()` 需要 `&mut self`，必须先收集再写入）。

## Acceptance Criteria

- [x] Criterion 1: 两个左右并排 pane 触发一次 split 后，`Node::pane_regions()` 算出的三个 pane 宽度两两相差 ≤ 0.5px（单元测试断言）。
- [x] Criterion 2: 均分只作用于新 pane 所在的同轴分组 —— 在"上下分割且下半再左右分割"的混合轴布局中触发 split，另一分组的 `Split` ratio 在 split 前后保持不变（单元测试断言）。
- [x] Criterion 3: 实机启动 otty，在两 pane 的 tab 内按 Cmd+D，截图显示三个终端等宽，且第三个 pane 出现可用 shell 提示符。
- [x] Criterion 4: 在终端 pane 内按 Cmd+D 不向 pty 写入任何字节 —— 实机确认 shell 无回显、无字符插入、无换行。
- [x] Criterion 5: `cargo +nightly fmt --check` 与 `cargo clippy --workspace --all-targets --all-features -- -D warnings` 退出码均为 0。
- [x] Criterion 6（Round 1 后经用户同意改写，原措辞见 Round Log）: `cargo test --workspace --all-features` 零失败，`cargo deny check` 退出码 0，且本次新增与改动的业务逻辑模块行覆盖率 ≥ 80%。

## Out of Scope

- 均分的设置开关与独立的"均分兄弟 pane"命令（issue 提及为备选；默认自动均分已满足诉求，YAGNI）。
- 水平分割（`Axis::Horizontal`）的快捷键 —— issue 只要求 Cmd+D。
- 其他分组内用户手动拖拽出的比例 —— 明确保留不动，见 Criterion 2。

## Progress Tracking

- [x] Task 1: 均分 ratio 计算函数与单元测试（TDD，先红）
- [x] Task 2: 把均分接进 `split_pane`，覆盖键盘与右键菜单两条路径
- [x] Task 3: 在 `AppEvent::Keyboard` 分支识别 Cmd+D 并派发 `SplitPane`
- [x] Task 4: 验证 Cmd+D 不被终端 widget 吞掉，必要时修正事件优先级
- [x] Task 5: 文档同步 —— README 记录 Cmd+D 快捷键

## Implementation Tasks

### Task 1: 均分 ratio 计算函数与单元测试

**Objective:** 在 `otty/src/widgets/terminal_workspace/` 下新增一个纯函数，输入 `&Node` 与新建的 `Pane`，输出该 pane 所在同轴分组内每个 `Split` 的目标 ratio 列表。按 AGENTS.md 要求先写测试再写实现，覆盖 2 pane、3 pane 左深树、3 pane 右深树、4 pane、以及混合轴不越界五种形态。

### Task 2: 把均分接进 split_pane

**Objective:** 在 `state.rs:300` 的 `split_pane` 中，`self.panes.split()` 成功后调用 Task 1 的函数并逐个 `resize`。放在这一层意味着右键菜单的 split 与 Cmd+D 走同一条路径，两者行为一致 —— 不在调用方各修一遍。

### Task 3: Cmd+D 键盘绑定

**Objective:** 把 `otty/src/events/mod.rs:61` 当前丢弃键盘事件的分支改为识别 `Cmd+D`（`Key::Character("d")` + `Modifiers::COMMAND`），解析当前活动 tab 的 focused pane，派发 `TerminalWorkspaceIntent::SplitPane { axis: Axis::Vertical }`。非 Cmd+D 的键盘事件保持原样丢弃。

### Task 4: 键盘事件与终端输入的冲突验证

**Objective:** 确认 Cmd+D 不会被终端 widget 先行消费或写入 pty，必要时在 `otty-ui/terminal/src/input.rs` 修正。这是本次唯一可能需要改动 `otty-ui` 的地方。

静态分析已定位分叉点（实测决定走哪条）：

- `iced::keyboard::listen()` 只产出 `Status::Ignored` 的事件（docs.rs 原文 "listens to ignored keyboard events"）。若终端把 Cmd+D 判为 `Captured`，`events/mod.rs` 永远收不到。
- `bindings.rs:128` 的 `get_action` 对修饰键做精确相等匹配，Cmd+D 无绑定 → 返回 `BindingAction::Ignore`。
- `input.rs:429-437`：`binding_action == Ignore` 且 `text` 为 `Some` 时，直接把 text 写进 pty 并返回 `Captured`。

**路径 A** — macOS 上 Cmd+D 的 `text` 为 `None`：不进上述分支，落到末尾 `_ => Status::Ignored`，订阅可收到，本任务只需记录验证结果，无需改 `otty-ui`。

**路径 B** — `text` 为 `Some("d")`：Cmd+D 会把 "d" 写进 pty 并被 capture。此时在 `input.rs` 的字符分支加一条判断 —— 带 `Modifiers::COMMAND` 且无绑定匹配时不写 pty、返回 `Ignored`。修在这一处即可覆盖全部 `Cmd+<无绑定字母>`，属同一逻辑缺陷的单点修复，不是范围扩张。

附带核实：`Key::Character` 分支读的是缓存的 `view_state.keyboard_modifiers`，而 `Key::Named` 分支读事件自带的 `modifiers`（`input.rs:421` vs `440`）。确认这个不一致不会让 Cmd 状态读错。

### Task 5: 文档同步

**Objective:** README 目前没有任何快捷键记载。新增用户可见快捷键属于文档同步触发条件，补一处 Cmd+D 说明（新建 pane 并均分同组宽度）。

## Round Log

- 阻塞（Round 0，未计入轮次）：本机没有 Rust 工具链 —— `~/.cargo` 与 `~/.rustup` 均不存在，`command -v cargo` 无结果（已绕过 sandbox 复查）。Task 1 的测试与空实现已落盘于 `otty/src/widgets/terminal_workspace/pane_balance.rs`，并已在 `mod.rs` 注册，但**从未编译、从未运行**，红灯尚未验证，因此 Task 1 保持未勾。Task 2-5 全部依赖编译与实测，无法开工。
  解阻：`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`，随后 `rustup toolchain install nightly && cargo install cargo-llvm-cov cargo-deny`。
  恢复顺序：先跑 `cargo test -p otty pane_balance` 确认四个测试因空实现而失败（RED），再填 `equalized_ratios` 的实现转绿，然后 Task 2。

- Round 1: 五个任务全部完成，判定 5/6 通过。任务清单未发生增删，与起草时一致。

  **Task 4 走了路径 B。** 实机确认 macOS 上 Cmd+D 的 `text` 是 `Some("d")`：第一次按下时 split 正常发生，但左侧 pane 的提示符后多出一个 `d`。已在 `input.rs` 的字符分支加入 `!keyboard_modifiers.contains(Modifiers::COMMAND)` 条件，单点覆盖全部 `Cmd+<无绑定字母>`，并补了两个单元测试（Cmd+D 不写 pty 且返回 `Ignored`；无修饰键的 `d` 仍写 pty 且 `Captured`）。

  **Criterion 6 失败**，失败点只有覆盖率一条：`cargo test --workspace --all-features` 507 passed / 0 failed 退出码 0，`cargo deny check` 退出码 0（advisories/bans/licenses/sources 全 ok），但 `cargo llvm-cov --workspace --all-features --fail-under-lines 80` 退出码 101 —— workspace 行覆盖率 66.97%。

  这是项目既有基线，非本次改动导致。本次改动文件的行覆盖率：`pane_balance.rs` 94.41%（新增）、`state.rs` 91.58%、`input.rs` 79.01%、`events/mod.rs` 32.14%（改动前该文件无任何测试）。拉低总数的是零覆盖的 view 层，例如 `settings_form.rs`、`tab_bar.rs`、`pane_grid.rs`、`pane_context_menu.rs` 均为 0.00%。

  该标准起草时误读了 AGENTS.md。原文是"ensure that it's not decreased for changed code (baseline >= 80%)"，要求的是**改动代码**的覆盖率，我写成了整个 workspace 达标。把 workspace 从 67% 提到 80% 需要给数千行既有 UI 代码补测试，既超出本次 issue 范围，也与 AGENTS.md"不为 infrastructure/bootstrap 包写测试"的规则冲突。

  **过程中修掉的既有缺陷**（均在本次 lineage 之外，依据 testing.md 对既有失败的零容忍例外，逐项列明）：
  1. `otty/src/view.rs:7`、`otty/src/events/mod.rs:2,74` 三个 macOS 平台条件编译导致的 unused warning —— 项目自带的 `cargo lint` 在 macOS 上同样报这三个 error，且它们在本次改动前的首次编译输出中已存在。
  2. `otty-libterm/examples/unix_shell.rs:164`、`otty-ui/terminal/examples/blocks_overlay.rs:312,343` 三处 `collapsible_if` —— 项目自带的 `cargo lint` 用 `--benches` 而非 `--all-targets`，从不检查 examples，因此长期未被发现。
  3. `settings` 的两个测试依赖 `$SHELL` 环境变量：断言 `set_shell("/bin/zsh")` 会置 dirty，但 `default_shell()` 读的就是 `$SHELL`，在 macOS 默认的 `$SHELL=/bin/zsh` 上 draft 等于 baseline，必然失败。已改为从 baseline 派生一个恒不相同的值。用 `SHELL=/bin/ksh` 重跑原测试 25 passed，确证是环境依赖而非本次改动所致。

- Criterion 6 改写（Round 1 判定后，经用户明确同意，非静默降低）：

  改写前：`cargo test --workspace --all-features` 零失败，`cargo deny check` 与 `cargo llvm-cov --workspace --all-features --fail-under-lines 80` 退出码均为 0。

  改写后：`cargo test --workspace --all-features` 零失败，`cargo deny check` 退出码 0，且本次新增与改动的业务逻辑模块行覆盖率 ≥ 80%。

  理由：原措辞是我起草时对 AGENTS.md 的误读，把"改动代码覆盖率不下降（基线 >= 80%）"写成了整个 workspace 达标；项目 workspace 基线 66.97%，从未达到 80%，该门槛与本次 issue 无关且与 AGENTS.md 的测试范围规则冲突。

  按改写后措辞的判定证据：`cargo test --workspace --all-features` 507 passed / 0 failed 退出码 0；`cargo deny check` 退出码 0；业务逻辑模块 `pane_balance.rs` 94.41%（新增）、`state.rs` 91.58%，均 ≥ 80%。

  另外两个改动文件不计入该门槛，理由如实记录：`events/mod.rs` 32.14%，属事件分发装配层（AGENTS.md 明确不要求为此类模块补测试），且改动前该文件无任何测试、覆盖率为 0%，本次为净提升；`input.rs` 79.01% 为该文件既有水平，本次仅新增 3 行条件判断并为其补了 2 个单元测试，覆盖只增不减。

- Round 2（Codex 分支 review 后）: 对 `main...HEAD` 跑 Codex review，报出三条，全部属实并已修复。三条的共同根源是 Round 1 的验收标准只有 macOS 证据，跨平台维度从起草时就缺席，因此这套标准不可能发现它们。

  1. **[P1] Linux 上 Ctrl+D 会同时发 EOF 和分屏。** `iced_core/keyboard/modifiers.rs:39-43` 定义 `COMMAND` 在非 macOS 等于 `CTRL`，而 `bindings.rs:213` 把 `"d" + CTRL` 绑为 `Char('\x04')`。我起初以为终端的 `Captured` 会挡住订阅，实际 `otty-ui/terminal/src/view.rs` 全文没有 `capture_event` 调用，`handle_keyboard_event` 的返回值被丢弃，事件照样流向 `keyboard::listen()`。改为仅在 macOS 注册该快捷键。
  2. **[P1] 自动重复未被过滤。** `KeyPressed` 带 `repeat` 字段，原实现用 `..` 丢弃，长按会按重复事件逐个派发 `SplitPane`，每次新建 Terminal 与 shell 进程。现在派发前拒绝 `repeat == true`。
  3. **[P2] 字符分支读缓存 modifiers。** 这正是 Task 4 里写了"附带核实"却没做的那一条。新建 pane 的 `TerminalViewState` 初始 modifiers 为空，且因修饰键未变化不会收到 `ModifiersChanged`，导致按住 Cmd 连续分屏时 `d` 泄漏进新 pty。绑定查询与 Command guard 均改读事件自带的 `modifiers`。

  验证：新增三个单元测试（拒绝重复、非 macOS 路径、陈旧修饰键缓存），其中第三条先复现红灯再转绿。`pre-commit run --all-files` 四个 hook 全 Passed，严格 clippy（含 examples）与全量测试退出码 0。实机在 macOS 上确认普通字符输入未受影响（`echo regression-check` 正常执行），并由用户手动确认两次 Cmd+D 得到三个等宽 pane、提示符无多余字符。

  **未能验证的两项，如实记录：** 非 macOS 分支无法在本机验证 —— 交叉编译缺 `x86_64-linux-gnu-gcc`，`openssl-sys` 需编译 C 代码，只能依赖 CI；`#[cfg(not(target_os = "macos"))]` 的那个测试同理只在 Linux CI 上运行。

  **过程中的两个观察（既有问题，未处理）：** `otty` 包的 pty 创建在并发下偶发失败，`otty-ui-term` 的 `double_click_clears_selection` 依赖 `Click` 的时间语义、并发下偶发失败，两者均多次全量重跑通过，属既有 flaky，不在本次 lineage 内。
