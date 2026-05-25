# MOUSE-NEXT Agent 上下文

系统级鼠标手势工具（Rust / Windows）。在系统层面监测鼠标按键与轨迹，识别上下左右方向，后续将映射为键盘按键以控制浏览器 tab 等。

**当前进度**：输入监测模块已实现（`cargo run` 可测试方向检测与 CLI 彩色输出）。键盘映射、白名单、浮窗等尚未实现。

## 架构

三线程单向事件流：

```
Hook 线程 ──原始事件 channel──► 处理线程 ──方向 channel──► 主线程
  WH_MOUSE_LL 转发              监测器状态机              ANSI 彩色打印
  固定规则吞/透传               轨迹积累 + 补发右键
```

- Hook 回调只读 Windows 事件、转发到 channel、决定吞掉或透传；不维护状态
- 处理线程持有 `监测器`，维护跨事件状态
- 右键按下/抬起分别发送 `方向::开始` / `方向::结束`
- 触发方向阈值（100px）时，`累积_x` 与 `累积_y` 一并清零

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
    ├── config.rs          # 全局常量
    └── input/             # 输入监测模块
        ├── mod.rs         # 模块导出
        ├── types.rs       # 事件与方向类型
        ├── hook.rs        # WH_MOUSE_LL Hook
        ├── processor.rs   # 原始事件消费线程
        └── monitor.rs     # 轨迹状态机
```

## 文件说明

### 根目录

| 文件 | 说明 |
|------|------|
| [Agents.md](Agents.md) | Agent 首轮上下文：项目概览、架构、文件树与各文件职责 |
| [mouse-next.md](mouse-next.md) | 产品需求文档。记录输入输出、手势效果（右键+滑动→tab/shift+tab）、各功能模块规格；开发中需保持准确 |
| [Cargo.toml](Cargo.toml) | 包名 `mouse-next`，依赖 `windows`（Win32 Hook / SendInput）和 `ctrlc` |
| [Cargo.lock](Cargo.lock) | Cargo 依赖版本锁定，应纳入版本控制 |
| [.gitignore](.gitignore) | 忽略 Rust 构建产物 `target/` |

### src/

| 文件 | 说明 |
|------|------|
| [src/main.rs](src/main.rs) | 入口。创建两个 `mpsc` channel，启动处理线程与 Hook 线程；主线程接收 `方向` 事件并以 ANSI 彩色打印（开始/结束/上下左右）；Ctrl+C 退出 |
| [src/config.rs](src/config.rs) | 全局配置常量。当前仅 `方向阈值 = 100`（屏幕像素） |

### src/input/ — 输入监测模块

| 文件 | 说明 |
|------|------|
| [src/input/mod.rs](src/input/mod.rs) | 输入模块入口，导出 `启动_hook`、`启动处理器`、`方向` |
| [src/input/types.rs](src/input/types.rs) | 核心类型：`原始鼠标事件`（右键按下/抬起、鼠标移动）和 `方向`（开始/结束/上/下/左/右） |
| [src/input/hook.rs](src/input/hook.rs) | `WH_MOUSE_LL` 低层 Hook。独立线程 + 消息泵；回调内解析 `MSLLHOOKSTRUCT`，跳过 `LLMHF_INJECTED`，按固定规则吞掉右键按下/抬起、透传鼠标移动；通过 `thread_local` 绑定 `Sender` 转发原始事件 |
| [src/input/processor.rs](src/input/processor.rs) | 处理线程。循环 `recv` 原始事件，驱动 `监测器` |
| [src/input/monitor.rs](src/input/monitor.rs) | 轨迹状态机。右键按下发送开始并进入记录、移动时积累位移、达阈值发送方向并重置 xy 累积、右键抬起发送结束且若无手势则补发右键单击。含单元测试 |

## 运行与验证

```bash
cargo run      # 启动监测，按住右键拖动测试
cargo test     # 运行 monitor 单元测试
```

## 后续模块（未实现）

- 键盘映射：消费 `方向` channel，输出 Ctrl+Tab / Ctrl+Shift+Tab 等
- 白名单：仅在指定程序前台时生效
- 管理浮窗：类似 Alt+Tab 的多 tab 切换界面
