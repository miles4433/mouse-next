# MOUSE-NEXT Agent 上下文

系统级鼠标手势工具（Rust / Windows）。在系统层面监测鼠标按键，通过表驱动的固定九宫格面板映射为键盘快捷键，控制浏览器 tab 等。

**当前进度**：表驱动九宫格 overlay 已落地。`panel_table.rs` 为唯一配置源（9 槽注册、层 0/±1、动作序、触碰/松手）。overlay / 匹配器被动读表；已删除方向 channel、三格接续展开、`registry` / `preview` / `animation`。左/右 tab 触碰可重复（左键同格）；恢复/关闭为 `上→右上` / `下→右下` 松手。冻结层在离格截屏、tab 执行后淡出。

**当前目标**：保持表驱动架构稳定；增删动作只改 `panel_table`。白名单、Ctrl、真实 tab 数据、非右键触发键尚未实现。

**计划状态**：同步线程 + `std::sync::mpsc`；Tokio 不在本阶段引入。

## 当前计划待办

- [x] 表驱动 `panel_table`（宫格注册 + 层切换 + 动作表 + 查询 API）
- [x] 固定 3×3 九宫格（锚点居中，不平移）
- [x] 层 0 左/右触碰 tab；层切换上/下；复合动作右上/右下松手
- [x] 左键同格重复 tab；跨层无效；锁符号 + 悬停消锁
- [x] `宫格事件` channel 替代 `方向` channel
- [x] 离格截屏 + tab 冻结层淡出
- [ ] 格内字体/图标细化（当前 Unicode 符号）

## 已确认设计偏好

- **单一数据源**：动作、层、偏移、触发方式、锁态 → 全部在 `panel_table`，业务代码禁止硬编码「上=进一层」。
- **被动 GUI / 被动执行**：overlay 只命中/绘制；matcher 只读表 `SendInput`。
- Hook 只采集事件、会话内吞右键与左键、转发 `原始鼠标事件`。
- 右键抬起关闭面板；未触发手势则补发右键单击。
- 简单可靠：同步 channel，不引入 Tokio。

## 架构

```
Hook 线程 ──原始事件──► Overlay 输入线程
  吞右键/会话内左键          │ 命中槽 → 宫格事件 channel
                             ▼
                     匹配器（读 panel_table）
                             │
                             ▼
主线程 egui ◄── 重绘 ──► 执行器 SendInput
```

- **层**：`0` 水平（左/右 tab）、`1` 上层（右上恢复）、`-1` 下层（右下关闭）
- **步进队列**：`Vec<(层, 宫格槽)>`，序匹配；独占动作后会话锁定
- **九宫格槽**：左上/左下空角弱显；上/下切层；原点回层清序

## 动作表（默认，均在 panel_table）

| 名称 | 触发序列 | 方式 | 可重复 |
|------|----------|------|--------|
| 左边 | `(0,左)` | 触碰 | 是 |
| 右边 | `(0,右)` | 触碰 | 是 |
| 恢复 | `(0,上)→(1,右上)` | 松手 | 否 |
| 关闭 | `(0,下)→(-1,右下)` | 松手 | 否 |

## 关键实现原则

- 浮窗：空闲 1×1，手势期工作区 99% 居中；egui 遍历注册表绘 9 格。
- 冻结层：离开 `离开截屏=true` 的槽时缓存；tab 触碰后显示并淡出。
- 通道：`宫格事件` + `原始鼠标事件`；`Arc<AtomicBool>` 标记会话供 Hook 吞左键。

### 模块结构

```
src/
├── action/
│   ├── panel_table.rs  # 声明表 + 查询 API（唯一配置）
│   ├── matcher.rs      # 宫格事件 → 执行
│   ├── executor.rs
│   └── types.rs
├── input/
│   ├── hook.rs
│   ├── replay.rs
│   └── types.rs
└── overlay/
    ├── state.rs        # 命中、层/队列、发事件
    ├── input.rs
    ├── window.rs
    ├── capture.rs
    └── repaint.rs
```

## 文件树

```
mouse-next/
├── Agents.md
├── mouse-next.md
├── Cargo.toml
└── src/
    ├── main.rs
    ├── config.rs
    ├── input/
    ├── action/
    │   ├── panel_table.rs
    │   ├── matcher.rs
    │   ├── executor.rs
    │   └── types.rs
    └── overlay/
        ├── state.rs
        ├── input.rs
        ├── window.rs
        ├── capture.rs
        └── repaint.rs
```

## 命名规范

- 业务类型 / 函数 / 变量：主要用中文
- Win32 / channel / Hook：保留英文
- 文件名 / 模块路径：英文

## 运行与验证

```bash
cargo run
cargo test
```
