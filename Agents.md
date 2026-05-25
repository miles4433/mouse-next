# MOUSE-NEXT Agent 上下文

系统级鼠标手势工具（Rust / Windows）。在系统层面监测鼠标按键与轨迹，识别方向并映射为键盘快捷键，控制浏览器 tab 等。

**当前进度**：输入监测模块与动作模块已实现（`cargo run` 可测试手势与快捷键输出）。白名单、浮窗、开始/结束的 Ctrl 按住/松开尚未实现。

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
