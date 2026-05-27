# Overlay 窗口机制尝试记录

本文记录 egui overlay 原型阶段关于 Windows 透明窗口、最大化、焦点和穿透机制的尝试与判断。它是实验记录，不是最终方案说明。

## 目标

需要一个手势期间临时出现的 overlay：

- 平时不影响当前前台应用。
- 右键按下后显示轨迹、按钮、方向/动作提示。
- 鼠标移动可根据全局坐标命中按钮。
- 右键抬起后隐藏内部 UI，并发送选中按钮对应动作。
- 长期希望支持类似 Alt+Tab 的半透明菜单、tab 列表和排序。

由于鼠标输入已经由 `WH_MOUSE_LL` hook 处理，overlay 不需要依赖窗口自身的鼠标事件。理想模型是：窗口只负责画，hover 和触发都来自全局鼠标事件。

## 用户偏好与约束

这些偏好会影响后续方案选择：

- 不接受长期手写复杂 UI。轨迹线、临时调试背景这类简单图形可以手写，但按钮高亮、选择状态、列表、排序、tab 面板、菜单布局等不应长期靠自绘硬写。
- 未来 UI 会从简单手势轨迹演进到半透明菜单、tab 列表、功能按钮、排序和选择，因此需要保留 egui 这类 UI 框架的布局和交互能力。
- 高性能图形和复杂 UI 能力都重要；如果窗口机制需要下沉到 Win32/DirectComposition，也应尽量让 egui 继续负责 UI，而不是转向纯手写 GUI。
- Layered Window + 自绘只适合作为简单轨迹层或验证透明机制，不适合作为长期主方案。
- 架构上应避免为了单个窗口实验引入过重的重构；优先保持核心输入链路简单可靠，把窗口机制封装在 overlay 边界内。

## 当前实现状态

当前原型使用 `eframe/egui`，采用**小尺寸隐藏 + 触发后放大**方案：

- 空闲时窗口保持 `1x1`，位于屏幕 `(0, 0)`，不覆盖桌面。
- 右键按下时一次性放大到当前显示器 `rcWork`，显示轨迹、按钮和提示。
- 显示期间只重绘内容，不再发送 resize/move 命令。
- 右键抬起后缩回 `1x1`，清空 UI 状态。
- 继续保留 `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` 和 `SWP_NOACTIVATE`。
- 隐藏态不绘制任何内容，避免调试背景挡鼠标。

以下章节中关于常驻大窗口、穿透、`ShowWindow` 显隐等尝试为历史记录，不再作为当前实现方向。

## 已尝试的窗口模型（历史）

### 小透明窗口

早期小窗口使用：

- `with_transparent(true)`
- `with_decorations(false)`
- `with_always_on_top()`
- `with_taskbar(false)`
- 创建后补 `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`

结果：

- 小尺寸透明窗口可以显示。
- 显示轨迹、文字、按钮可行。
- 轨迹超出窗口范围后无法继续绘制，这是窗口尺寸限制。

判断：

- eframe 的普通透明窗口在小尺寸下可用。
- 这说明透明绘制本身不是完全不可行，问题主要出现在大尺寸/全屏/穿透/焦点组合上。

### 写死全屏尺寸

尝试把窗口设置为大尺寸，例如 `3840x2160`，并移动到 `(0, 0)`。

结果：

- 出现全屏黑屏。
- 分辨率写死不适配当前系统和多屏。

判断：

- 不应写死分辨率。
- 大尺寸透明 eframe 窗口可能触发不同的 DWM 合成路径，透明背景被当成黑色。

### 虚拟屏幕尺寸

尝试使用：

- `SM_XVIRTUALSCREEN`
- `SM_YVIRTUALSCREEN`
- `SM_CXVIRTUALSCREEN`
- `SM_CYVIRTUALSCREEN`

结果：

- 尺寸获取方式比写死标准，但仍出现黑屏。

判断：

- 尺寸获取方式正确，但不能解决 eframe 大透明窗口的合成问题。
- 覆盖所有显示器不是当前最优目标，应该先覆盖当前鼠标所在显示器。

### 当前显示器无边框全屏

尝试使用：

- `MonitorFromPoint`
- `GetMonitorInfoW`
- `MONITORINFO.rcMonitor`

将窗口放到当前鼠标所在显示器完整区域。

结果：

- 仍存在黑屏/焦点风险。

判断：

- 这更接近无边框全屏，可能仍容易触发 Windows 窗口化全屏、DWM 优化或特殊合成路径。

### 系统最大化

尝试先把窗口移动到锚点附近，再发送：

- `ViewportCommand::Maximized(true)`

结果：

- 会出现明显或轻微的 Windows 最大化动画。
- 动画来自系统窗口管理器/DWM，不是 egui 自己的动画。

判断：

- `Maximized(true)` 会让 Windows 按普通窗口最大化状态处理，因此可能播放系统最大化动画。
- 若要避免动画，不应使用最大化状态，而应手动设置普通无边框窗口的大小和位置。

### ShowWindow 显示/隐藏

尝试使用：

- `ShowWindow(SW_HIDE)`
- `ShowWindow(SW_SHOWNOACTIVATE)`
- `SetWindowPos(..., SWP_NOACTIVATE | SWP_SHOWWINDOW)`

结果：

- 可能干扰焦点。
- 隐藏后出现再也打不开的情况。

判断：

- `ShowWindow` 与 winit/eframe 的窗口状态管理可能不同步。
- 对 eframe 窗口不宜混用 Win32 显示/隐藏作为主要生命周期机制。

### 常驻窗口，只隐藏内容

尝试让窗口常驻：

- 不 `ShowWindow`
- 不移到屏幕外
- 不销毁窗口
- 不最大化/还原
- `visible=false` 时 egui 不画内部元素
- `visible=true` 时画背景、轨迹、按钮和提示

结果：

- 这是目前更适合继续验证的方向。
- 能避免窗口显示/隐藏和最大化动画对焦点的影响。

判断：

- 对当前架构最合理：窗口一直在，内容显隐由状态控制。
- 如果最终能解决穿透和透明显示，这是最稳定的交互模型。

## 穿透尝试

### eframe / winit `with_mouse_passthrough(true)`

尝试使用：

- `ViewportBuilder::with_mouse_passthrough(true)`

结果：

- 当前环境下会导致 overlay 内容不显示。

判断：

- 不能直接用于当前 eframe 透明大窗口。
- 可能是 winit 平台 hit-test/窗口透明与 eframe 渲染组合产生了兼容问题。

### Win32 `WS_EX_TRANSPARENT`

尝试手动加：

- `WS_EX_TRANSPARENT`
- `SetWindowPos(..., SWP_FRAMECHANGED)`

结果：

- 单独看机制是正确的，但在当前 eframe 窗口上显示不稳定。

判断：

- `WS_EX_TRANSPARENT` 是 Windows 原生鼠标穿透机制。
- 但 eframe/winit 创建的渲染窗口不一定适合在创建后随意改 hit-test 行为。
- 若要可靠穿透，可能需要更底层地接管窗口创建或窗口过程。

### Win32 `WS_EX_LAYERED`

尝试加：

- `WS_EX_LAYERED`

结果：

- 会导致 eframe 内容不显示。

判断：

- eframe 透明窗口不是 `UpdateLayeredWindow` 那套绘制机制。
- 强行加 `WS_EX_LAYERED` 会破坏 eframe 渲染路径。
- Layered Window 适合原生 Win32 + 自己绘制，不适合直接套在 eframe 窗口上。

## 焦点判断

目标是右键按下、移动、抬起全程不改变前台焦点，否则 `SendInput` 发出的快捷键可能进入 overlay。

已使用：

- `WS_EX_NOACTIVATE`
- `WS_EX_TOOLWINDOW`
- `SetWindowPos(..., SWP_NOACTIVATE)`

判断：

- 创建后补 `WS_EX_NOACTIVATE` 有帮助，但不一定覆盖所有 eframe/winit 创建和显示过程。
- `ShowWindow`、最大化、窗口位置大小变化都可能影响焦点或窗口激活状态。
- 最稳妥的方向是创建阶段就带 noactivate/toolwindow/topmost/passthrough 样式，而不是创建后补。

## DWM / MPO / 全屏优化判断

MPO 是 Windows DWM、DXGI 和显卡驱动在合成阶段的优化机制，应用不能直接要求“使用 MPO”。

相关判断：

- 小透明窗口可用，大尺寸透明窗口黑屏，说明合成路径可能随窗口尺寸和样式变化。
- 无边框全屏、大尺寸透明窗口可能被 DWM 或窗口化优化按特殊 surface 处理。
- MPO 可能是问题来源之一，但不是可直接依赖的解决方案。

更适合的长期方向：

- DirectComposition + DXGI alpha surface + egui-wgpu 自组装。
- 或 Win32 Layered Window + 自己的软件渲染，但这不适合未来复杂 UI。

## 当前结论

短期继续使用 eframe/egui 做 UI 原型，但窗口机制需要谨慎：

- 保持核心输入链路不变。
- Overlay 不参与识别，只展示事件和结果。
- 用 `rcWork` 获取当前显示器工作区，避免写死尺寸。
- 避免 `ShowWindow` 和 `Maximized(true)` 作为常规显示机制。
- 优先验证“常驻透明窗口 + 内容显隐”的模型。
- 暂时不要再把 `WS_EX_LAYERED` 直接加到 eframe 窗口上。
- `with_mouse_passthrough(true)` 和 `WS_EX_TRANSPARENT` 需要单独实验，不能假定稳定。

## 后续可尝试方向

### 方向 A：继续 eframe，逐步收敛

- 先保证大窗口显示稳定。
- 再尝试不抢焦点。
- 最后尝试穿透。
- 每次只改一层机制，避免多个变量同时变化。

适合短期原型验证。

### 方向 B：winit/egui 自组装

不使用 eframe，改为自己接：

- winit window
- egui-winit input
- egui-wgpu renderer
- 自定义窗口创建和样式

优点是能更早控制窗口属性。缺点是工程复杂度明显上升。

### 方向 C：DirectComposition + egui-wgpu

用 Win32/DirectComposition 控制透明合成窗口，用 egui-wgpu 负责 UI 渲染。

这是长期更合理的系统级 overlay 方向：

- 保留 egui 的 UI 能力。
- GPU 渲染性能更好。
- 透明合成路径更接近 Windows 原生 DWM 机制。

但实现成本高于当前 eframe 原型。

### 方向 D：Layered Window + 自绘

用：

- `WS_EX_LAYERED`
- `UpdateLayeredWindow`
- tiny-skia / Direct2D / GDI+

适合轨迹和简单按钮，但不适合未来复杂菜单、排序和 tab 列表 UI。

因此不作为当前优先方向。
