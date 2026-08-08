# VS Code 1.128-1.132 Modern UI 源码调研

## 1. 核心结论

VS Code 最近几个版本的 Modern UI 不是一次单纯的视觉换肤。从 1.128 到
1.132，它逐步形成了四层协同方案：

1. 统一的实验开关与配置迁移；
2. 可复用的颜色与尺寸 token；
3. 为浮动卡片真实预留空间的布局计算；
4. 在不替换既有 Workbench Part 的前提下重塑视觉的样式模块。

对 OTTY 最重要的结论是：圆角只是最表层的结果。真正使这套设计可用的，是
状态、测量、绘制和测试之间的严格一致。VS Code 在演进过程中发生的主要回归，
几乎都来自其中一层试图间接推断另一层状态，或者布局尺寸与最终绘制不一致。

OTTY 已经实现了部分外观：4/6/8 px 分级圆角、透明标签栏、跟随主题的颜色混合、
带边框的编辑器主表面，以及独立的侧边栏和活动栏颜色。但它还没有建立对应的
设计系统、布局几何、模式边界和状态覆盖。因此当前实现是“借鉴了 VS Code 的
视觉语言”，还不能称为“对齐 VS Code Modern UI 的实现方案”。

建议 OTTY 后续沿以下方向演进：

- 将终端 ANSI 调色板与语义化 UI 颜色分离；
- 建立强类型的尺寸、间距、圆角、描边和排版 token；
- 在渲染前将卡片 inset 作为业务显著的布局数据完成计算；
- 让 Modern UI 状态显式存在，不从渲染后的组件树反推；
- 只有在状态模型和视觉测试矩阵同时就绪时，才扩展标签页能力。

## 2. 范围与方法

本文基于官方 [`microsoft/vscode`](https://github.com/microsoft/vscode)
仓库的以下稳定标签：

| 版本 | 标签提交 | 标签日期 |
| --- | --- | --- |
| 1.128.0 | [`fc3def6774c`](https://github.com/microsoft/vscode/tree/fc3def6774c76082adf699d366f31a557ce5573f) | 2026-07-07 |
| 1.129.0 | [`125df4672b8`](https://github.com/microsoft/vscode/tree/125df4672b8a6a34975303c6b0baa124e560a4f7) | 2026-07-15 |
| 1.130.0 | [`1b6a188127e`](https://github.com/microsoft/vscode/tree/1b6a188127eeaf9194f945eb6eb89a657e93c54c) | 2026-07-22 |
| 1.131.0 | [`3a03d6f72d6`](https://github.com/microsoft/vscode/tree/3a03d6f72d628a7741c29f456b4ddbb5ae68502c) | 2026-07-28 |
| 1.132.0 | [`df53daabb18`](https://github.com/microsoft/vscode/tree/df53daabb18cd157bdb08c7f01c34df936cf12f4) | 2026-08-04 |

调研使用完整本地克隆，通过标签间 diff、提交祖先关系、实现文件和测试代码建立
结论。Release notes 只适合说明用户看到什么，不作为本文判断“内部如何实现”的
依据。

本文中的 Modern UI 特指：

- `workbench.experimental.modernUI`；
- `styleOverrides` Workbench Contribution；
- 浮动的 sidebar、panel 和 editor surface；
- 现代标签页样式；
- 作为设计与布局试验场的 Agents window。

1.132.0 之后的主干变化会单独标记，不能视为 1.132.0 稳定版已经发布的行为。

## 3. 五个版本的演进

### 3.1 版本矩阵

| 版本 | 源码里程碑 | 架构变化 |
| --- | --- | --- |
| 1.128 | 统一实验首次进入本调研范围的稳定标签 | 原本独立的 `floatingPanels` 和 `styleOverrides` 由 `workbench.experimental.modernUI` 统一控制；Workbench 使用统一的 `style-override` 展示 class；浮动几何已经进入布局代码。 |
| 1.129 | 设计系统和组件覆盖扩大 | 字体阶梯被规范化；size registry 获得完整的排版词汇；通知、对话框、终端等 override 扩大；首帧提前注入样式 class，避免第一次测量不一致。 |
| 1.130 | 浮动卡片几何趋于完整 | edge ownership、状态栏和活动栏间距、webview clipping、编辑器边框、活动栏尺寸、hover 状态以及 compact/pinned tab 缺陷被作为相互影响的布局状态处理。 |
| 1.131 | 语义 surface 与选择器成本显式化 | 通用 `surface.*` 主题色替代 Agents 专属 surface 色；重复选择器被合并；active pane identity 写入 `dataset`，减少结构选择器开销。 |
| 1.132 | 标签页按完整状态系统重做 | active/hover 背景、compact 尺寸、wrap、pinned、dirty、selected、multi-selected、拖拽指示、截断、action 位置和高对比度被集中重构；根节点 `:has()` 在严重性能回归后被移除。 |

### 3.2 统一开关，而不是允许任意组合

关键起点是提交
[`bf46ff6aa608`](https://github.com/microsoft/vscode/commit/bf46ff6aa6087791540c8224ce056d1bb245ab13)
“Centralize Modern UI toggle”。此前 floating panels 和单独的 style override 模块
可以独立开启；该提交将两个重叠实验合并为一个布尔配置：

```text
workbench.experimental.modernUI
```

它同时注册配置迁移：删除旧的 `workbench.experimental.floatingPanels` 和
`workbench.experimental.styleOverrides`，只要任一旧实验开启，且新配置尚未被用户
显式设置，就启用新配置。

这不是简单的设置项改名。一个同时改变测量和绘制的设计模式，如果允许用户自由
组合局部模块，会快速放大状态空间：某个 padding 模块可能要求新的 header 高度，
而对应布局模块却没有启用。VS Code 最终选择“一组模块作为一个产品实验发布”。

在 1.132.0 中，该设置仍为实验项，默认 `false`，并带有
`experiment: { mode: 'auto' }`，允许实验基础设施控制开启范围。对应源码见
[`workbench.contribution.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/workbench.contribution.ts)
和
[`layoutService.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/services/layout/browser/layoutService.ts)。

### 3.3 首帧必须使用最终尺寸

在 1.132.0 中，`style-override` 运行时由 `StyleOverridesContribution` 管理，但该
Contribution 在 `LifecyclePhase.Restored` 才启动。部分 Workbench Part 会在布局时
读取这个 class，以决定 title 或 tab 高度。如果完全等待 Contribution，第一次布局
会先使用经典 UI 尺寸，再切换到 Modern UI 尺寸。

提交
[`658b4cee0e3b`](https://github.com/microsoft/vscode/commit/658b4cee0e3ba4e0bb415931086dcfa928b4898f)
因此让 `Layout.getLayoutClasses()` 在初始容器创建时也注入 `style-override`。这里
故意存在“首帧预置”和“运行时所有权”两条路径，目的是保证第一次测量和第一次绘制
已经一致。

这个原则不依赖 Electron。在 Iced 这类 retained-mode UI 中，只要样式模式会改变
intrinsic size，就必须在第一次 layout 之前选定，而不能把它当成首次绘制后的装饰更新。

## 4. 实现架构

### 4.1 Style Overrides 是 Contribution，不是替换 Workbench

1.132.0 的
[`StyleOverridesContribution`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/contrib/styleOverrides/browser/styleOverrides.contribution.ts)
导入 14 个 CSS 模块：

- activity bar；
- command center；
- editor border；
- font ramp；
- keyboard-only focus；
- notifications/dialogs；
- padding；
- pane headers；
- rounded corners；
- sash handles；
- shadows；
- status bar；
- tabs；
- title bar。

模块目录仍保留 15 个条目，其中 `scrollShadows` 是历史目录项，但它的独立样式文件
已经在 1.130 前后移除，不能把目录项数量误认为实际 CSS 文件数量。

所有规则默认不生效，只有容器祖先存在 `style-override` 时才激活。Contribution：

- 给当前所有 Workbench container 应用 class；
- 监听后续创建的 auxiliary window container；
- 配置变化时更新全部容器；
- dispose 时恢复全局 metric 并清理 class；
- 仅在 layout-affecting 模块启用状态发生变化时触发额外 relayout。

这保留了既有 Workbench Part 层级、action、快捷键和状态逻辑，只替换展示层和少量
明确的尺寸策略。同时它不是用户可注入任意 CSS 的机制：样式模块全部是随产品发布的
固定代码，用户只能开关整个实验。

### 4.2 绘制 token 与布局 metric 是两种契约

VS Code 的 size registry 早于本轮实验，但 1.128-1.132 将它真正用于 Modern UI。
1.132.0 的
[`baseSizes.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/platform/theme/common/sizes/baseSizes.ts)
定义了：

- heading：26、18、13 px；
- body：13、11 px；
- label：12、11、10 px；
- regular/semibold 字重：400、600；
- icon：16、12 px；
- radius：2、4、6、8、12、9999 px；
- stroke：1 px；
- 0-40 px 的 2 px 基准 spacing ramp。

size registry 将 `cornerRadius.large` 等标识符转为
`--vscode-cornerRadius-large` CSS 变量，解析不同主题的默认值，并贡献配置 schema。
组件消费语义 token，而不是继续增加局部魔法数字。

但 CSS token 不天然适合作为布局 metric。稳定版 1.132.0 的 pane header 仍从
computed style 读取 CSS 变量，并在值变化后补一次 relayout。紧随 1.132.0 之后的
提交
[`44bc3fd7813`](https://github.com/microsoft/vscode/commit/44bc3fd7813c15611acc93e5c399d71b84544bfd)
移除了布局热路径上的 computed-style 读取，改用 `setGlobalPaneHeaderSize`，同时将
Contribution 提前到 `WorkbenchPhase.BlockRestore`。

这个后续修正反向证明了合理边界：

- color、radius、spacing、typography 和纯绘制效果可以通过 theme/style token 流动；
- minimum size、split geometry 等参与测量的值应保存在代码中，并通过显式 relayout 更新。

OTTY 不需要复制 CSS registry，但应复制这种契约区分。

### 4.3 Semantic surface color

提交
[`07745f1d764f`](https://github.com/microsoft/vscode/commit/07745f1d764f390795e99d0d4e224918d149a49d)
增加三个通用主题色：

```text
surface.background
surface.foreground
surface.border
```

此前 floating layout 复用了 Agents window 专属颜色。新命名描述的是业务语义：
“带边框的容器表面/卡片”。默认 dark surface 继承 sidebar background，light surface
继承 editor background，普通主题的 border 使用 15% 前景透明度，高对比度主题则使用
contrast border。

这不是把所有组件都涂成一个颜色。panel 仍可使用 `panel.background`，auxiliary bar
仍可使用 `sideBar.background`，但 framed surface 可以共享 foreground 和 border 语义。
可复用视觉原语不应继续依赖它最初诞生的产品功能名称。

### 4.4 浮动卡片在 CSS 和 TypeScript 中各实现一次

视觉 margin 与 border 位于
[`floatingPanels.css`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/media/floatingPanels.css)，
但同一组几何也必须由 TypeScript 为内容真实预留。

1.132.0 稳定版常量为：

```text
FLOATING_PANEL_MARGIN       = 4 px
FLOATING_PANEL_INNER_MARGIN = 2 px
border                      = 每侧 1 px
```

前导 4 px 与尾随 2 px 共同形成相邻卡片间 6 px 的 gap。处于窗口最外侧的卡片使用
8 px outer gutter。对应 Part 的 layout 会从 content size 中减去 margin 和 border。

`AbstractPaneCompositePart.layout()` 的过程是：

1. 保存 Grid 提供的原始 dimension；
2. 计算 floating inset；
3. 从 width/height 中减去 inset；
4. 写入 outer-edge class；
5. 将缩减后的 content dimension 交给基础布局。

保存原始 dimension 很关键。由 header、footer 等内部变化触发的 relayout 必须再次使用
原始尺寸，否则会在已经缩减的尺寸上重复减 inset，产生累积收缩。

`EditorPart.layout()` 对主编辑器做同样的空间预留，并额外减去 1 px frame border。
`ActivitybarPart` 在 Modern UI 下同时改变 intrinsic width 与 action height，并将 gutter
计入 minimum/maximum width。Status bar 的 padding 也进入测量，而不是只存在于 CSS。

最终契约是：

```text
Grid 分配尺寸
  - 卡片 margin
  - 卡片 border
  = 内容布局尺寸
```

如果只画 CSS，子内容仍会认为自己拥有被 margin/border 占用的像素，结果是 clipping、
sash 偏移、终端列数错误，或者 relayout 时持续变小。

### 4.5 Edge ownership 是布局业务逻辑

outer gutter 不能由单个组件本地决定，它依赖：

- primary sidebar 在左还是右；
- activity bar 是否可见；
- primary/auxiliary sidebar 是否可见；
- editor 是否可见；
- panel 的位置与 alignment；
- horizontal panel 是否延伸到 sidebar 下方；
- status bar 是否可见。

VS Code 在 `getFloatingOuterEdgeOwners()` 中重建横向 Part 顺序，跳过隐藏 Part，再从
窗口两侧向内寻找第一个可见 Part。Activity bar 可以占据窗口边缘，但它不是 card，
因此不会产生 card owner。Horizontal panel 另行计算，因为同一个 panel 可以同时到达
左右两个窗口边缘。

相关逻辑集中在
[`layoutService.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/services/layout/browser/layoutService.ts)：

- `getFloatingOuterEdgeOwners`；
- `getFloatingOuterGutterEdges`；
- `getFloatingSidebarSiblingToEditorStatus`。

这些函数虽然输出像素相关状态，却属于业务显著逻辑：它们编码了所有受支持 Workbench
排列中“哪个 Part 拥有哪个边界”，因此 VS Code 用纯状态组合测试覆盖它们。

### 4.6 根状态显式镜像，不从 DOM 结构推断

Modern UI 初期使用 CSS `:has()`，让根节点或兄弟节点根据 compact activity bar、隐藏
Part 等后代结构改变样式。提交
[`f57741ddcd28`](https://github.com/microsoft/vscode/commit/f57741ddcd28b480a74120a8b0e3cd41a890bbfe)
记录了实际代价：只要 live DOM 中存在 root-anchored `:has()`，Blink 会在文档任意位置
发生 DOM mutation 时重新评估它。大型 chat working set 中，一次强制样式重算约为
218 ms，拖动 sash 接近冻结。

修复方式是将应用层已经知道的状态镜像到 Workbench root：

- `Layout` 写入 `noactivitybar`；
- `ActivitybarPart` 写入 `activitybar-compact`；
- floating panel owner 写入 outer-edge 状态；
- active pane composite id 写入 `data-active-composite`。

CSS 只消费稳定 class/data attribute，不扫描渲染树重新发现 TypeScript 已经拥有的状态。

对 OTTY 的直接启示是：render style 应由 state/view model 直接派生，而不是检查生成后的
widget tree。

### 4.7 Webview 揭示了 frame 与 content 的区别

VS Code webview iframe 挂载在 Workbench root，再覆盖到所属 card 上，不能自然继承
editor container 的 clipping。Modern UI 最终给直接包裹并裁剪 iframe 的
`.webview-overlay-content` 设置圆角。只给 overlay root 设置圆角无效，因为其
fixed-position child 会逃离该 clip。

这是 DOM 特有约束，OTTY 不应照搬。可迁移原则是：在真正的 compositing boundary
应用 clip。对 OTTY 来说，terminal/editor 内容应由同一个负责绘制 card radius 的
renderer node 裁剪，而不是只给更外层容器添加圆角样式。

## 5. Modern tabs 是状态系统，不是 pill 样式

### 5.1 几何与颜色

1.132.0 的 Modern UI multi-tab 使用：

- 普通 tab 高 24 px，容器上下各 4 px，总 title area 32 px；
- compact tab 高 20 px，加相同 padding，总 title area 28 px；
- 4 px radius；
- 每个 tab 后 4 px spacing；
- compact pinned tab 宽 28 px；
- inactive 背景透明；
- active/hover 背景由主题前景色混合生成。

Dark theme 的 active/hover 分别使用 22%/8% 前景混合，light theme 使用 16%/6%。
tab action 的变体会将前景色与 editor background 混合，而不是使用一个固定 alpha，
以保证按钮在实际 surface 上仍然可读。

代码和 CSS 共同持有这些测量约束。`EditorTabsControl` 在 style override 激活时返回
不同 title height；`MultiEditorTabsControl` 在计算 pinned tab 滚动位置时使用
28 px compact width 和额外 4 px spacing。

### 5.2 覆盖状态

1.132 的标签重构提交
[`82705b922d89`](https://github.com/microsoft/vscode/commit/82705b922d896afda0b07ddb23f77ff9a1bd6d40)
不只处理 active/inactive。对应 fixture matrix 包括：

- multiple、single、hidden tab mode；
- pinned tab 位于主行或独立行；
- compact、shrink、fixed、fit sizing；
- wrapped tabs；
- close/unpin action 位于左右侧或隐藏；
- dirty tab 与 modified border；
- tab index；
- 短、长、重名、溢出 label；
- icon 与 decoration 组合；
- active/inactive editor group；
- multi-selection；
- drag insertion target；
- high-contrast theme；
- editor action 位置与 title scrollbar mode。

此前一系列 bug 说明了这张矩阵为何必要：

- 多个 compact pinned tab 会重叠；
- 隐藏 close button 后 padding 错误；
- shrink sizing 产生错误 hover shadow；
- multi-selection 视觉反馈丢失；
- drag target 被绘制两次；
- 状态切换时短暂出现双边框。

OTTY 当前 tab view 只有 active boolean、title、close action、固定宽度和字符截断。
如果只复制 pill 外观，而不定义状态契约，真正困难的交互仍然没有方案。

### 5.3 1.132 之后的方向

1.132.0 之后的提交
[`a73436f3b28`](https://github.com/microsoft/vscode/commit/a73436f3b28d7fb507a52fb17eb372e07a371f0d)
将 modern tab 从整个 `style-override` 中分离到专用 `modern-ui-tabs` class。这样普通
Workbench 和 Agents window 可以共享同一份 tab 样式，而不必同时开启所有 Modern UI
模块。

这对 OTTY 是合理目标：tab presentation 可以作为可复用组件样式，整个 Modern UI
layout 则是另一层应用模式。但必须明确，这不是 1.132.0 稳定标签已经发布的行为。

## 6. Agents single-pane 架构

Modern UI 配置描述明确指出其设计对齐 Agents window，因此 Agents 的实现是理解方案的
必要部分。它在两层使用不同扩展方式。

### 6.1 Workbench shell selection

[`createSessionsWorkbench`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/sessions/browser/workbenchFactory.ts)
只在 dock-detail setting 开启且不是 phone viewport 时选择 `SinglePaneWorkbench`。
该决定在构造时固定，切换设置需要 reload。

`SinglePaneWorkbench` 继承 Agents `Workbench`，但修改 serialized grid topology：
auxiliary bar 不再拥有独立 grid column，而是 dock 到 editor node 内。editor content 或
detail content 任意一个可见，editor node 就保持可见。持久化 width 时，只有 detail
实际存在才从 editor node width 中减去 docked detail width，避免每次 reload 继续缩小。

`SinglePaneMainEditorPart` 复用标准 editor part 和 editor group，但额外拥有 docked
auxiliary bar，强制 multiple tabs，提供专用 menu id，并监听 group header 高度变化。

### 6.2 Behavior controller selection

会话行为层的
[`SessionsLayoutContribution`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/sessions/contrib/layout/browser/sessions.layout.contribution.ts)
在三种 sibling controller 中选择一个：

- `SinglePaneLayoutController`；
- `MobileLayoutController`；
- classic desktop `LayoutController`。

single-pane 与 desktop controller 都继承 `BaseLayoutController`。Single-pane 行为通过
策略对象组合：detail visibility、detail content mapping、managed tabs、editor collapse、
responsive sidebar、new-session rule 和 quick-chat hide。共享 context 暴露 restore 状态，
避免把 session restore 引起的变化误判成用户操作。

因此物理布局壳层使用继承改变 Grid，行为编排层使用 sibling controller 与 composition
隔离策略。简单说“它采用继承”或“它采用组合”都会遗漏这一层次区分。

### 6.3 对 OTTY 的可迁移部分

OTTY 当前没有 Agents session domain，也没有多个竞争的 Workbench topology，不应为了
形式对齐而增加 single-pane controller。可迁移原则更有限：

- 在第一次测量前选择 topology；
- 持久化 content width，而不是混合 content 与 docked panel 的总宽度；
- restore layout state 时暂时抑制 visibility reaction；
- 将物理布局所有权与 feature-specific visibility policy 分离。

## 7. 性能与失败复盘

### 7.1 结构选择器

root-anchored `:has()` 是最强警告：一个方便的 CSS 关系让编辑器日常 inline style 写入
触发全 Workbench 样式重算。最终方案是显式 state class 和 data attribute。

OTTY 中对应的反模式，是每次 draw/layout 都遍历大型 widget tree 来推断 hover、
visibility 或 layout mode。正确做法是只存一次状态，再沿 view model 向下传递。

### 7.2 布局热路径上的 computed style

稳定版 1.132 的 pane header 从 computed style 读取 CSS variable，并按 layout pass 缓存。
紧随其后的主干提交改用代码 metric。这说明即使“统一 token”在概念上更漂亮，也不值得
在布局热路径引入 style/layout 同步开销。

OTTY 中所有参与 `PaneGrid`、terminal resize 或 minimum-size 计算的 metric 都应是 Rust
值。renderer 消费这些值，而不是再从 renderer 反向查询。

### 7.3 Override 层的选择器与状态膨胀

提交
[`07745f1d764f`](https://github.com/microsoft/vscode/commit/07745f1d764f390795e99d0d4e224918d149a49d)
合并重复选择器；提交
[`45fb8c7c3e37`](https://github.com/microsoft/vscode/commit/45fb8c7c3e372442884d38dc0a18b0ae9c446018)
将 active composite identity 写入 pane root，避免反复结构匹配。这表明使用晚期 override
修补大量既有组件，会迅速累积性能和维护成本。

OTTY 拥有类型化组件构造器，不需要复制这个成本。应优先使用共享 style function 与
显式 view-model field，而不是让每个组件各自解释一个宽泛的 late override object。

## 8. VS Code 的测试策略

### 8.1 纯布局组合测试

`layoutService.test.ts` 枚举 visible parts、sidebar side、panel position、panel alignment
和 Modern UI 激活状态，验证 edge owner 与精确 margin。此类测试便宜，却能捕获最危险的
回归：代码预留几何与 CSS 绘制几何不一致。

### 8.2 组件 metric 测试

Activity bar 测试验证普通/compact width、floating gutter、content height，以及运行时
切换 Modern UI 后的变化。当前主干的 Contribution 测试还覆盖初始激活、auxiliary
container、metric reset 和模式切换后只触发一次 relayout。

注意：后一组 Contribution 测试属于 1.132 之后主干补强，不应误写成 1.132.0 已有测试。

### 8.3 Visual fixture matrix

editor tab fixture 会分别在 Modern UI on/off 下渲染 normal、light、high contrast、各种
配置组合和边界状态。它不是一张总览截图；fixture 列表本身就是受支持状态契约。

对 OTTY，应先给业务显著的几何写 Rust unit test，再增加小型视觉状态矩阵。逐一测试
每个纯样式函数的价值低于测试 layout ownership 与用户可见状态。

## 9. OTTY 差距分析

### 9.1 已经实现的部分

当前代码已经包含多项明确的 VS Code-inspired 设计：

- [`layout.rs`](../otty/src/layout.rs) 定义 4、6、8 px radius；
- [`theme.rs`](../otty/src/theme.rs) 实现跟随主题的前景/背景混色；
- [`theme.rs`](../otty/src/theme.rs) 增加 sidebar、activity bar、accent 颜色；
- [`view.rs`](../otty/src/view.rs) 将主编辑器绘制为带边框的圆角 surface；
- [`tab_bar.rs`](../otty/src/widgets/tabs/view/tab_bar.rs) 使用透明 inactive tab 和混色
  active pill；
- 应用使用系统窗口 decoration，不再让内部自绘 title bar 占据内容布局。

这些变化已经对齐部分视觉词汇，但还没有对齐下面的结构机制。

### 9.2 终端色与 UI 色仍在同一个模型

`ColorPalette` 同时包含 ANSI black/red/green/yellow/blue/magenta/cyan 槽位和语义 UI
字段，Settings 又以一个位置数组暴露全部颜色。UI 组件仍使用 `dim_black`、`dim_white`
和 `red` 等终端来源颜色表达 chrome 状态。

VS Code 的 `surface.*` 演进说明了更合理边界：UI role 应描述业务含义，不应继承某个
终端色名称或最早使用它的功能名称。

建议至少建立以下 UI role：

```text
shell.background
surface.background
surface.foreground
surface.border
control.hoverBackground
control.activeBackground
control.foreground
control.mutedForeground
focus.border
danger.foreground
accent.background
```

Terminal ANSI colors 应保留为独立 terminal palette。一个 theme 可以同时构造两套
palette，但普通 widget 不应知道 ANSI 槽位名称。

### 9.3 Layout token 不完整且分散

OTTY 有 control size 和三个 radius，但没有统一 spacing、stroke、typography、icon 和
surface metric 词汇。组件局部常量混合了语义尺寸和实现细节。

下一层 token 应是具体 Rust value，而不是 trait 或通用设计系统框架。当前只有一个真实
实现，一个不可变的 `UiMetrics` 或 `UiTokens` 就足够；只收纳已经被多个组件使用，或
Modern UI layout 当前确实需要的值。

layout-affecting value 必须在构造 Iced view tree 前提供。OTTY 没有理由重现 VS Code 的
class toggling 或 computed-style bridge。

### 9.4 Card geometry 被绘制，但没有成为布局模型

OTTY 的主内容 card 已经绘制 border/radius，sidebar 与 content 仍通过固定
`PaneGrid` spacing 和 separator 连接。`pane_grid_size()` 会减 tab bar 和 sidebar menu
width，但不知道 card outer gutter、border 或 adjacency。

这对终端是 correctness 问题。1 px 或 4 px 的布局不一致可能改变终端行列数，触发重复
PTY resize，或让内容进入 border 区域。

布局层应从显式状态计算 `SurfaceInsets`：

- Modern UI 是否启用；
- sidebar 是否隐藏；
- workspace panel 是否打开；
- content 是否到达左右窗口边缘；
- tab bar 是否存在；
- 当前平台/窗口 decoration 是否暴露窗口边缘。

同一个结果同时用于 content size 和 padding/border/clip 绘制。不能在 `view.rs` 与
`layout.rs` 中各自维护一套相似算术。

### 9.5 缺少明确的 Modern UI 模式边界

当前 Modern UI 变化无条件生效，没有 setting、migration、startup selection 或 classic
fallback。只有在产品已经决定 Modern UI 是唯一长期设计，并且旧样式路径会被完整删除时，
这种做法才合理。

如果 OTTY 需要分阶段发布，应只引入一个 boolean setting，并在第一次布局前选定。不要
暴露可任意组合的 style module 列表，VS Code 已经用实践证明该模式会让测量与视觉状态
难以推理。

如果产品决定只保留 Modern UI，则不要为了形式复制一个永远不会关闭的 speculative
toggle；但仍应保留 token 和 geometry 边界，以保证内部一致和可测试。

### 9.6 Tab state contract 不完整

OTTY tab 当前支持 activate/close，宽度固定 235 px，并用最多 20 字符的尾部保留规则
截断。它不包含 pinned、dirty、multi-selection、wrap、compact、drag insertion、
inactive group 或 action placement。

OTTY 不应为了 parity 一次性实现 VS Code 全部状态。应先决定哪些是真实产品需求。每增加
一个状态，都要同时定义：

- layout measurement；
- foreground/background/border；
- action visibility；
- keyboard/pointer behavior；
- overflow behavior；
- 对应测试 fixture。

固定宽度和按字符数截断最终应改为根据 available width 测量并由 renderer clip。字符数
不能预测 shaped glyph width，尤其无法正确处理 CJK、组合字符和不同字体宽度。

### 9.7 缺少视觉回归矩阵

OTTY 已有 reducer/theme unit test，但没有覆盖 root layout 与 tab bar 的 component fixture
或 screenshot matrix。视觉状态组合因此没有保护。

第一版可采用以下最小矩阵：

- dark/light theme；
- sidebar hidden/open；
- workspace panel hidden/open；
- single tab/overflowing tabs；
- active/inactive tab；
- 长 Latin label 与宽字符 label；
- sidebar collapse threshold 两侧的窗口宽度；
- 如果测试工具可控，再增加 high-DPI scale。

## 10. 建议的 OTTY 实施顺序

以下是后续方向，不属于本次调研文档实施范围。每一阶段都必须遵循仓库规则：先写业务
显著测试，再写实现。

### Phase 1：语义 token

1. 先补 theme parsing/defaulting 与 semantic UI color derivation 测试。
2. 在保持现有 settings 兼容的前提下，拆分 terminal ANSI colors 与 semantic UI colors。
3. 引入具体 token value，承载共享 spacing、radius、stroke、typography、icon 和 control
   metric。
4. 将 Workbench chrome 对 ANSI 色的直接依赖替换为 semantic UI role。

从 VS Code 直接采用：语义 role 和 paint/layout 分离。

按 Iced 改写：直接使用 Rust value，不建立生成 CSS variable 的 registry。

不采用：运行时插件式 token registry 或 per-component override module。

### Phase 2：可测量的 surface layout

1. 先为现有 sidebar visibility 状态编写 surface ownership/inset 表驱动测试。
2. 增加纯 geometry function，返回 content inset 与 outer-edge 状态。
3. 在绘制 surface 前，将结果接入 `screen_size`、`pane_grid_size`、sidebar sizing 和
   terminal workspace sizing。
4. renderer 使用同一份 geometry 完成 padding、border、clip 和 radius。
5. 验证 sidebar/workspace visibility 切换只产生一次一致的 terminal resize，不发生
   递增式缩小。

从 VS Code 直接采用：显式 ownership、保留 raw dimension、测量与绘制完全一致。

按 Iced 改写：用类型化 state 和 layout function 表达，不使用 DOM class。

不直接采用：VS Code 的非对称 margin 数值。OTTY 是更密集的 terminal workspace，应在
实际视觉与终端尺寸测试后选择自己的 spacing。

### Phase 3：标签状态与视觉 fixture

1. 定义 OTTY 真正需要的 tab state，优先 active、hover、dirty、overflow、drag target
   和 close-action visibility。
2. 修改 view 前先增加 reducer/model 测试。
3. 将固定字符截断替换为按 available width 测量。
4. 为约定状态增加 component fixture 或 deterministic screenshot。
5. compact/pinned/wrapped tab 在当前工作流没有需求时保持后置。

从 VS Code 直接采用：状态矩阵思维与主题感知的 active/hover 混色。

按 Iced 改写：每个 style 直接由 tab state 和 semantic token 决定。

不采用：OTTY domain 并不存在的数百条 CSS 状态组合。

## 11. 实施前检查清单

每次继续修改 OTTY Modern UI 前，应回答：

1. 这个值是 paint token 还是 layout metric？
2. layout 是否预留了所有最终会被 margin/border 绘制占用的像素？
3. 状态是显式拥有，还是从渲染树反推？
4. 颜色是 semantic UI role，还是 ANSI/theme 实现细节？
5. 新功能是否引入真实产品状态，并有相应测试？
6. 第一次 layout 是否已经使用最终 metric？
7. relayout 是否可能重复减 inset？
8. 隐藏某个 Part 是否会改变窗口 edge owner？
9. high contrast、light、long label 和 overflow 是否仍可读？
10. 新抽象是否解决当前的第二实现、基础设施边界、真实重复或测试困难？

## 12. 关键源码索引

### 1.132.0 稳定快照

- [`styleOverrides.contribution.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/contrib/styleOverrides/browser/styleOverrides.contribution.ts)
- [`floatingPanels.css`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/media/floatingPanels.css)
- [`layoutService.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/services/layout/browser/layoutService.ts)
- [`layout.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/layout.ts)
- [`paneCompositePart.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/parts/paneCompositePart.ts)
- [`editorPart.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/parts/editor/editorPart.ts)
- [`activitybarPart.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/parts/activitybar/activitybarPart.ts)
- [`tabs.css`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/contrib/styleOverrides/browser/media/tabs.css)
- [`editorTabsControl.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/parts/editor/editorTabsControl.ts)
- [`multiEditorTabsControl.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/browser/parts/editor/multiEditorTabsControl.ts)
- [`baseSizes.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/platform/theme/common/sizes/baseSizes.ts)
- [`editorTabBar.fixture.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/workbench/test/browser/componentFixtures/editor/editorTabBar.fixture.ts)
- [`singlePaneWorkbench.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/sessions/browser/singlePaneWorkbench.ts)
- [`singlePaneLayoutController.ts`](https://github.com/microsoft/vscode/blob/df53daabb18cd157bdb08c7f01c34df936cf12f4/src/vs/sessions/contrib/layout/browser/singlePaneLayoutController.ts)

### 关键提交

- [`bf46ff6aa608`](https://github.com/microsoft/vscode/commit/bf46ff6aa6087791540c8224ce056d1bb245ab13)：统一 Modern UI 开关并迁移旧设置。
- [`658b4cee0e3b`](https://github.com/microsoft/vscode/commit/658b4cee0e3ba4e0bb415931086dcfa928b4898f)：在第一次 layout 前预置样式 class。
- [`dd766a81feda`](https://github.com/microsoft/vscode/commit/dd766a81feda3bb44ff977b4ff512f5f6ab29895)：规范 font-size/font-weight token。
- [`8338938a170f`](https://github.com/microsoft/vscode/commit/8338938a170fcba1799b7aa8135100047261e618)：重构 floating card geometry。
- [`07745f1d764f`](https://github.com/microsoft/vscode/commit/07745f1d764f390795e99d0d4e224918d149a49d)：统一 semantic surface token 与选择器。
- [`45fb8c7c3e37`](https://github.com/microsoft/vscode/commit/45fb8c7c3e372442884d38dc0a18b0ae9c446018)：修复 Modern UI 选择器性能回归。
- [`f57741ddcd28`](https://github.com/microsoft/vscode/commit/f57741ddcd28b480a74120a8b0e3cd41a890bbfe)：移除 root-anchored `:has()` 状态推断。
- [`82705b922d89`](https://github.com/microsoft/vscode/commit/82705b922d896afda0b07ddb23f77ff9a1bd6d40)：重做 modern tabs 并集中 bug bash。
- [`44bc3fd7813`](https://github.com/microsoft/vscode/commit/44bc3fd7813c15611acc93e5c399d71b84544bfd)：1.132 后的 startup/resize metric 优化。
- [`a73436f3b28`](https://github.com/microsoft/vscode/commit/a73436f3b28d7fb507a52fb17eb372e07a371f0d)：1.132 后拆分共享 modern tab 激活 class。
