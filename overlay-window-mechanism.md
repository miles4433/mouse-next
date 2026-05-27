# Overlay 窗口机制

## 当前方案

小尺寸隐藏 + 右键触发后放大 + 纯事件驱动重绘。

- 空闲：`1x1` 窗口位于 `(0, 0)`，不绘制内容，不轮询。
- 显示：右键按下时 Win32 `SetWindowPos` 一次性放大到当前显示器工作区的 **99%**（居中），避免四边完全贴满。
- 绘制：hook 更新状态后 `request_repaint()` 唤醒 egui，显示期间不再 resize/move。
- 隐藏：右键抬起后缩回 `1x1`。
- 焦点：`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | SWP_NOACTIVATE`。
- 输入：轨迹、按钮 hover、显隐全部由 hook 全局坐标驱动，不依赖窗口鼠标事件。

## 覆盖面积验证

全屏覆盖工作区（100% `rcWork`）时，手势期间浏览器 `Ctrl+Tab` 虽能切换 tab，但新页面像未渲染；缩小到 75% 后恢复正常。进一步验证 **99% 居中** 仍正常。

结论：overlay 不能四边完全贴满工作区，需保留少量未覆盖边距，否则会影响前台应用绘制/刷新。

## 模块职责

```
Hook ──原始事件──► overlay/input ──更新状态 + request_repaint──► eframe 绘制
处理器 ──方向(上/下/左/右)──► overlay/input ──更新提示文字 + request_repaint
```

- `state.rs`：显示生命周期、轨迹、按钮命中、提示文字。
- `input.rs`：消费 hook/方向事件，仅在状态变化时请求重绘。
- `repaint.rs`：跨线程 `egui::Context::request_repaint`。
- `window.rs`：Win32 窗口样式、几何切换、egui 绘制。

## 已放弃的方向

常驻工作区大窗口、ShowWindow 显隐、鼠标穿透实验、`WS_EX_LAYERED`、定时轮询重绘——均不再使用。
