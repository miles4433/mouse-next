# MOUSE-NEXT Agent 上下文

系统级鼠标手势工具（Rust / Windows）。在系统层面监测鼠标按键与轨迹，识别方向并映射为键盘快捷键，控制浏览器 tab 等。

**当前进度**：输入监测、动作模块、egui overlay 九宫格手势 UI 已可用。方向触发已改为 overlay 宫格命中；轨迹距离判断代码保留但不再驱动动作。白名单、Ctrl 按住/松开、真实 tab 数据尚未实现。

**当前目标**：在现有 overlay 窗口模型上继续演进 UI（宫格动画、菜单、窗口预览等），不再验证穿透/常驻大窗口方案。

**计划状态**：已确认方案，计划文件为 `egui 手势浮窗`。当前阶段优先保持同步线程 + `std::sync::mpsc` 的简单可靠风格；Tokio 不在本阶段引入，后续外部通信模块需要异步 I/O 时再在模块内部局部封装。

## 当前计划待办

目标：先稳定 egui overlay 的窗口行为，使手势期间能可靠显示轨迹和按钮，并且不抢焦点、不破坏原本快捷键输入；不要在本阶段扩展真实 tab 数据、浏览器扩展或复杂菜单系统。

- [x] 接入 `eframe/egui` overlay，展示右键手势轨迹和少量功能按钮。
- [x] 改为 3×3 动态九宫格：移入上/下/左/右格触发方向，以该格为新中心重建宫格。
- [x] 宫格动画：保留格标签 morph、范围外淡出、新格逐个淡入。
- [x] 方向唯一来源为 overlay 宫格命中；`monitor` 轨迹阈值逻辑保留但不接动作匹配器。
- [x] 小尺寸隐藏 + 触发放大 + 事件驱动重绘的 overlay 窗口模型。

## 已确认设计偏好

- 简单可靠优先：核心输入链路继续使用同步线程和 channel，不为当前浮窗提前引入 Tokio。
- Hook 低层稳定：Hook 只采集 Windows 鼠标事件、判断右键吞掉/透传、发送事件，不维护浮窗状态、不做手势识别。
- 方向触发在 overlay：`monitor` 仍积累位移但不再向匹配器发方向；移入宫格上/下/左/右时 overlay 发送 `方向`，驱动原有动作表。
- UI 不参与动作匹配逻辑，只负责宫格命中检测、视觉反馈和方向事件转发。
- UI 可以消费原始坐标来控制打开、画线、关闭；轨迹显示由 `config::显示鼠标轨迹` 控制，默认关闭。
- 方向和动作提示来自 overlay 宫格交互，可用于显示图标、文字、动作名。
- 右键抬起只关闭浮窗并发送 `方向::结束`，不再绑定假按钮打印。
- 后续外部数据模块独立：浏览器扩展、VSCode tab、Windows 资源管理器 tab 获取都应独立成数据服务，不直接侵入 Hook 或手势处理器。
- 文档保留高层上下文、关键取舍、项目偏好；具体实现细节优先放在代码结构和必要注释里。
- 修改应贴合当前代码风格，避免为未来可能性提前引入重架构。
- 不接受长期手写复杂 UI；未来按钮、菜单、tab 列表、排序和选择交互应尽量保留 egui 这类 UI 框架能力，自绘只适合简单轨迹层或机制验证。
- overlay 窗口机制的详细实验记录见 [overlay-window-mechanism.md](overlay-window-mechanism.md)。

## 架构

五线程单向事件流：

```
Hook 线程 ──原始事件 channel──► 处理线程（monitor，仅保留逻辑）
  WH_MOUSE_LL 转发              不再向匹配器发方向
  固定规则吞/透传                     │
                                      │
Hook 线程 ──原始事件 channel──► Overlay 输入线程
                                      │ 宫格命中 → 方向 channel
                                      ▼
                              匹配器线程 ◄── 方向::开始/结束/上/下/左/右
                              轨迹队列 + 动作匹配
                                      │
                                      ▼
主线程（egui 窗口）◄── 重绘信号 ── 执行 queue ──► 执行器线程
                                      SendInput
```

- Hook 回调只读 Windows 事件、转发到 channel、决定吞掉或透传；不维护状态
- 处理线程仍驱动 `监测器`（补发右键等），但方向 channel 已断开
- overlay 在 开始~结束 会话内，由宫格命中发送方向；匹配器逻辑不变
- 匹配器在 开始~结束 会话内匹配动作；独占动作触发后会话锁定至 结束
- 执行器将匹配到的按键序列通过 `SendInput` 发送

## 浮窗计划架构

egui 浮窗负责宫格 UI 与方向触发，不参与动作匹配。动作仍由匹配器 + 执行器完成。

```
Hook 线程
  ├── 原始鼠标事件 -> 处理线程（monitor，保留逻辑）
  └── 原始鼠标事件 -> Overlay 状态 -> egui 透明浮窗
                              │
                              └── 宫格命中 -> 方向 channel -> 匹配器 -> 执行器
```

- `原始鼠标事件` 是小型 `Copy` 数据，Hook 向处理器和 Overlay 各发送一份。
- 右键按下：展开 3×3 宫格，发送 `方向::开始`；四角为空白，中心/方向格显示符号。
- 移入上/下/左/右相邻格：立即发送对应方向，并以该格为新中心重建宫格；保留格标签 morph，范围外淡出，新格逐个淡入。
- 右键抬起：关闭浮窗，发送 `方向::结束`。
- 轨迹绘制可选，默认关闭（`config::显示鼠标轨迹`）。
- 浮窗窗口：空闲 `1x1` 隐藏，手势期间放大到工作区 99%（居中）。

## 关键实现原则

- 浮窗窗口：egui 负责绘制，Win32 负责几何切换和不抢焦点；输入与重绘均由 hook 事件驱动。
- 轨迹：hook 更新状态，`request_repaint()` 触发 egui 重绘，使用简单降采样控制点数。
- 通道：`std::sync::mpsc` 是单消费者；当前用 Hook 向核心逻辑和 Overlay 各发送一份事件，避免引入分发器或事件总线。
- Tokio：不是当前浮窗阶段的基础依赖；后续外部通信需要异步 I/O 时，在对应模块局部封装，对主程序暴露 channel。
- 线程模型：当前多线程 channel 已经满足事件驱动需求；Hook、处理器、匹配器、执行器、UI 各自保持清晰边界。

### 预期模块结构

```
src/
├── overlay/
│   ├── mod.rs       # Overlay 模块入口
│   ├── state.rs     # 宫格状态、命中检测、世界坐标、方向事件
│   ├── animation.rs # 宫格进入/退出/标签 morph 动画
│   ├── input.rs     # 消费 hook 事件，转发方向到匹配器
│   ├── repaint.rs   # 跨线程 request_repaint
│   └── window.rs    # egui 绘制、浅色玻璃样式、Win32 几何切换
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
- UI 形态可演进：当前为动态九宫格 + 动画，后续可替换为 tab 列表选择器或类似 Alt+Tab 的切换面板。

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
    ├── config.rs          # 方向阈值、显示鼠标轨迹等全局配置
    ├── input/             # 输入监测模块
    │   ├── mod.rs
    │   ├── types.rs
    │   ├── hook.rs
    │   ├── processor.rs
    │   └── monitor.rs
    ├── action/            # 动作模块
    │   ├── mod.rs
    │   ├── types.rs
    │   ├── registry.rs
    │   ├── matcher.rs
    │   └── executor.rs
    └── overlay/           # egui 浮窗模块
        ├── mod.rs
        ├── state.rs
        ├── animation.rs
        ├── input.rs
        ├── repaint.rs
        └── window.rs
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
| [src/main.rs](src/main.rs) | 入口。创建 channel，启动 Hook / 处理 / 匹配器 / Overlay 输入 / 执行器；主线程运行 egui 浮窗 |
| [src/config.rs](src/config.rs) | `方向阈值`（基础 100px、水平倍率 2.0）；`显示鼠标轨迹`（默认 false） |

### src/input/ — 输入监测模块

| 文件 | 说明 |
|------|------|
| [src/input/mod.rs](src/input/mod.rs) | 输入模块入口，导出 `启动_hook`、`启动处理器`、`方向` |
| [src/input/types.rs](src/input/types.rs) | `原始鼠标事件` 与 `方向`（开始/结束/上/下/左/右） |
| [src/input/hook.rs](src/input/hook.rs) | `WH_MOUSE_LL` 低层 Hook，独立线程 + 消息泵 |
| [src/input/processor.rs](src/input/processor.rs) | 处理线程，驱动 `监测器`（方向不再输出到匹配器） |
| [src/input/monitor.rs](src/input/monitor.rs) | 轨迹状态机：积累位移、发送方向（代码保留）；无任何移动时补发右键单击 |

### src/overlay/ — 浮窗模块

| 文件 | 说明 |
|------|------|
| [src/overlay/mod.rs](src/overlay/mod.rs) | 导出 `运行_overlay窗口`、`启动_overlay输入` 等 |
| [src/overlay/state.rs](src/overlay/state.rs) | 九宫格生成、世界坐标、命中检测、方向事件 |
| [src/overlay/animation.rs](src/overlay/animation.rs) | 宫格进入/退出/标签 morph 动画 |
| [src/overlay/input.rs](src/overlay/input.rs) | 消费原始事件，转发方向到匹配器，请求重绘 |
| [src/overlay/repaint.rs](src/overlay/repaint.rs) | 跨线程 `request_repaint` |
| [src/overlay/window.rs](src/overlay/window.rs) | egui 绘制、浅色玻璃样式、Win32 窗口几何 |

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
