# MOUSE-NEXT Agent 上下文

系统级鼠标手势工具（Rust / Windows）。在系统层面监测鼠标按键与轨迹，识别方向并映射为键盘快捷键，控制浏览器 tab 等。

**当前进度**：输入监测模块与动作模块已实现（`cargo run` 可测试手势与快捷键输出）。egui overlay 原型已接入，用于验证透明浮窗、轨迹绘制、按钮 hover 和假功能触发。白名单、开始/结束的 Ctrl 按住/松开、真实 tab 数据尚未实现。

**当前目标**：继续验证 overlay 窗口机制。当前 egui 浮窗可展示轨迹、按钮和调试背景，但大尺寸透明窗口、焦点保持、鼠标穿透之间存在兼容性问题，需要先稳定窗口模型，再绑定真实功能。

**计划状态**：已确认方案，计划文件为 `egui 手势浮窗`。当前阶段优先保持同步线程 + `std::sync::mpsc` 的简单可靠风格；Tokio 不在本阶段引入，后续外部通信模块需要异步 I/O 时再在模块内部局部封装。

## 当前计划待办

目标：先稳定 egui overlay 的窗口行为，使手势期间能可靠显示轨迹和按钮，并且不抢焦点、不破坏原本快捷键输入；不要在本阶段扩展真实 tab 数据、浏览器扩展或复杂菜单系统。

- [x] 接入 `eframe/egui` overlay，展示右键手势轨迹和少量功能按钮。
- [x] 保持核心手势识别只有一份，Overlay 只展示事件和结果，不参与动作判断。
- [x] 右键抬起时根据当前选中按钮发送假动作。
- [ ] 继续验证大尺寸透明窗口、焦点保持和鼠标穿透的兼容方案。

## 已确认设计偏好

- 简单可靠优先：核心输入链路继续使用同步线程和 channel，不为当前浮窗提前引入 Tokio。
- Hook 低层稳定：Hook 只采集 Windows 鼠标事件、判断右键吞掉/透传、发送事件，不维护浮窗状态、不做手势识别。
- 手势识别只有一份：UI 不重新计算方向、不重新判断动作，避免显示结果和真实执行结果不同步。
- UI 可以消费原始坐标来控制打开、画线、关闭，但这只是展示生命周期，不属于动作识别逻辑。
- 方向和动作提示来自核心逻辑，后续可用于显示图标、文字、动作名。
- 当前先做假功能：按钮选中后先打印文字或发送占位执行包，后续再绑定真实浏览器/VScode/资源管理器功能。
- 后续外部数据模块独立：浏览器扩展、VSCode tab、Windows 资源管理器 tab 获取都应独立成数据服务，不直接侵入 Hook 或手势处理器。
- 文档保留高层上下文、关键取舍、项目偏好；具体实现细节优先放在代码结构和必要注释里。
- 修改应贴合当前代码风格，避免为未来可能性提前引入重架构。
- 不接受长期手写复杂 UI；未来按钮、菜单、tab 列表、排序和选择交互应尽量保留 egui 这类 UI 框架能力，自绘只适合简单轨迹层或机制验证。
- overlay 窗口机制的详细实验记录见 [overlay-window-mechanism.md](overlay-window-mechanism.md)。

## 架构

五线程单向事件流：

```
Hook 线程 ──原始事件 channel──► 处理线程 ──方向 channel──► 匹配器线程
  WH_MOUSE_LL 转发              监测器状态机              轨迹队列 + 动作匹配
  固定规则吞/透传               轨迹积累 + 补发右键              │
                                                                ▼
主线程（等待退出）◄────────────────────────────────── 执行 queue ──► 执行器线程
                                                                      SendInput
```

- Hook 回调只读 Windows 事件、转发到 channel、决定吞掉或透传；不维护状态
- 处理线程持有 `监测器`，维护跨事件状态
- 右键按下/抬起分别发送 `方向::开始` / `方向::结束`
- 匹配器在 开始~结束 会话内匹配动作；独占动作触发后会话锁定至 结束
- 执行器将匹配到的按键序列通过 `SendInput` 发送

## 浮窗计划架构

egui 浮窗不参与手势识别逻辑，只消费事件并展示状态。真实手势判断仍然只保留一份，避免 UI 展示和实际动作不同步。

```
Hook 线程
  ├── 原始鼠标事件 -> 处理线程 -> 方向事件 -> 匹配器线程 -> 执行器线程
  └── 原始鼠标事件 -> Overlay 状态 -> egui 透明浮窗

方向/动作结果也可额外发送给 Overlay，用于显示当前方向、动作名、按钮说明。
```

- `原始鼠标事件` 是小型 `Copy` 数据，Hook 可直接向处理器和 Overlay 各发送一份，暂不引入额外分发器或 Tokio。
- UI 用原始屏幕坐标画轨迹：右键按下打开并记录锚点，鼠标移动追加/更新轨迹点，右键抬起发送选中按钮对应动作并关闭。
- 轨迹绘制使用降采样：输入线程只更新轨迹状态，不每包触发 UI 刷新；egui 每帧读取状态并重绘。
- 浮窗窗口目标：透明背景、无边框、置顶、不抢焦点；后续用 Win32 扩展样式补充 `WS_EX_NOACTIVATE`、`WS_EX_TOOLWINDOW` 等行为。
- 当前按钮为假功能，先打印文字或发送占位执行包；后续再绑定真实动作。

## 关键实现原则

- 浮窗窗口：egui 负责绘制，Win32 窗口层负责透明、无边框、置顶、不抢焦点；当前优先稳定显示和焦点，穿透仍在验证。
- 坐标：事件层保留屏幕坐标；轨迹绘制保存屏幕坐标点序列；UI 层负责转换到窗口内坐标。
- 轨迹：输入事件只更新轨迹状态，不每包触发 UI 刷新；egui 每帧读取状态并重绘，使用简单降采样控制点数。
- 通道：`std::sync::mpsc` 是单消费者；当前用 Hook 向核心逻辑和 Overlay 各发送一份事件，避免引入分发器或事件总线。
- Tokio：不是当前浮窗阶段的基础依赖；后续外部通信需要异步 I/O 时，在对应模块局部封装，对主程序暴露 channel。
- 线程模型：当前多线程 channel 已经满足事件驱动需求；Hook、处理器、匹配器、执行器、UI 各自保持清晰边界。

### 预期模块结构

```
src/
├── overlay/
│   ├── mod.rs       # Overlay 模块入口，导出启动函数和事件类型
│   ├── state.rs     # visible、锚点、轨迹点、按钮、选中态、提示文本
│   ├── input.rs     # 消费原始鼠标事件，更新 Overlay 状态
│   └── window.rs    # egui/eframe 窗口、绘制、Win32 窗口样式
└── external/        # 后续外部数据服务，可选
    ├── browser.rs   # 浏览器扩展 / tab 列表
    ├── vscode.rs    # VSCode tab 列表
    └── explorer.rs  # Windows 资源管理器 tab 列表
```

- `overlay` 是当前计划要实现的模块。
- `external` 只是后续方向，不在当前浮窗阶段创建，除非开始接真实 tab 数据。
- 如果未来 `external` 内部需要 Tokio，对外仍保持 `启动_xxx服务(Sender<事件>)` 这类普通 channel 接口。

## 后续方向

- 外部数据服务：后续浏览器扩展、VSCode、Windows 资源管理器 tab 列表获取，建议独立成模块。
- 局部 Tokio：如果外部数据服务需要 WebSocket、HTTP、Native Messaging、IPC、定时任务等异步 I/O，可在对应模块内部启动 Tokio runtime；对主程序仍只暴露普通 channel 接口。
- 核心链路保持同步：Hook、手势处理、动作匹配、SendInput、Overlay 状态更新继续优先使用同步线程和 channel，保证路径简单、低延迟、容易调试。
- UI 形态可演进：当前先显示轨迹和少量按钮，后续可替换为半透明菜单面板、tab 列表选择器或类似 Alt+Tab 的切换面板。

## 命名规范

- 业务类型 / 函数 / 变量：主要用中文
- Win32 常量 / API、channel / Hook 等惯用词：保留英文
- 文件名 / 模块路径：英文（Rust 惯例）

## 文件树

```
mouse-next/
├── Agents.md              # 本文件，首轮对话上下文
├── mouse-next.md          # 需求与功能规格（权威来源，随开发更新）
├── Cargo.toml             # Rust 项目配置与依赖
├── Cargo.lock             # 依赖锁定
├── .gitignore             # 忽略 /target
└── src/
    ├── main.rs            # 程序入口
    ├── config.rs          # 方向阈值等全局配置
    ├── input/             # 输入监测模块
    │   ├── mod.rs
    │   ├── types.rs
    │   ├── hook.rs
    │   ├── processor.rs
    │   └── monitor.rs
    └── action/            # 动作模块
        ├── mod.rs
        ├── types.rs
        ├── registry.rs
        ├── matcher.rs
        └── executor.rs
```

## 文件说明

### 根目录

| 文件 | 说明 |
|------|------|
| [Agents.md](Agents.md) | Agent 首轮上下文：项目概览、架构、文件树与各文件职责 |
| [mouse-next.md](mouse-next.md) | 产品需求文档。记录输入输出、手势效果、各功能模块规格；开发中需保持准确 |
| [Cargo.toml](Cargo.toml) | 包名 `mouse-next`，依赖 `windows`（Win32 Hook / SendInput）和 `ctrlc` |
| [Cargo.lock](Cargo.lock) | Cargo 依赖版本锁定，应纳入版本控制 |
| [.gitignore](.gitignore) | 忽略 Rust 构建产物 `target/` |

### src/

| 文件 | 说明 |
|------|------|
| [src/main.rs](src/main.rs) | 入口。创建三个 `mpsc` channel，启动 Hook / 处理 / 匹配器 / 执行器线程；主线程等待 Ctrl+C 退出 |
| [src/config.rs](src/config.rs) | `方向阈值` 结构体：基础 100px、水平倍率 2.0；上下用 `垂直()`，左右用 `水平()` |

### src/input/ — 输入监测模块

| 文件 | 说明 |
|------|------|
| [src/input/mod.rs](src/input/mod.rs) | 输入模块入口，导出 `启动_hook`、`启动处理器`、`方向` |
| [src/input/types.rs](src/input/types.rs) | `原始鼠标事件` 与 `方向`（开始/结束/上/下/左/右） |
| [src/input/hook.rs](src/input/hook.rs) | `WH_MOUSE_LL` 低层 Hook，独立线程 + 消息泵 |
| [src/input/processor.rs](src/input/processor.rs) | 处理线程，驱动 `监测器` |
| [src/input/monitor.rs](src/input/monitor.rs) | 轨迹状态机：积累位移、发送方向；同轴反向重置；各轴独立触发；无任何移动时补发右键单击 |

### src/action/ — 动作模块

| 文件 | 说明 |
|------|------|
| [src/action/mod.rs](src/action/mod.rs) | 导出 `启动匹配器`、`启动执行器` |
| [src/action/types.rs](src/action/types.rs) | `动作`、`动作类型`（重复/独占）、`虚拟键操作` |
| [src/action/registry.rs](src/action/registry.rs) | 默认动作表：下右/上右/左/右 |
| [src/action/matcher.rs](src/action/matcher.rs) | 轨迹队列匹配、会话锁定、连续方向 dedup |
| [src/action/executor.rs](src/action/executor.rs) | `SendInput` 执行按键序列 |

## 运行与验证

```bash
cargo run      # 启动手势，浏览器中按住右键拖动测试
cargo test     # 运行 monitor / matcher 单元测试
```

## 后续模块（未实现）

- 开始/结束 的 Ctrl 按住/松开
- 白名单：仅在指定程序前台时生效
- 管理浮窗：类似 Alt+Tab 的多 tab 切换界面
