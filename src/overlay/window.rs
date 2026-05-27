use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect,
    Stroke, StrokeKind, Vec2,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use super::repaint::{Overlay重绘信号, 注册重绘上下文};
use super::state::{Overlay共享状态, 隐藏窗口位置, 隐藏窗口大小};

const 默认显示大小: egui::Vec2 = egui::vec2(520.0, 320.0);

#[derive(Clone, Copy, PartialEq)]
struct 窗口范围 {
    原点: Pos2,
    大小: Vec2,
}

fn 隐藏范围() -> 窗口范围 {
    窗口范围 {
        原点: 隐藏窗口位置,
        大小: 隐藏窗口大小,
    }
}

pub fn 运行_overlay窗口(状态: Overlay共享状态, 重绘信号: Overlay重绘信号) {
    let 初始范围 = 隐藏范围();
    let 选项 = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("mouse-next overlay")
            .with_inner_size(初始范围.大小)
            .with_position(初始范围.原点)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_taskbar(false)
            .with_resizable(false),
        ..Default::default()
    };

    if let Err(错误) = eframe::run_native(
        "mouse-next overlay",
        选项,
        Box::new(|创建上下文| {
            let hwnd = 设置窗口不抢焦点(创建上下文);
            设置字体(&创建上下文.egui_ctx);
            Ok(Box::new(OverlayApp {
                状态,
                重绘信号,
                当前范围: 初始范围,
                hwnd,
            }))
        }),
    ) {
        eprintln!("浮窗启动失败: {错误}");
    }
}

struct OverlayApp {
    状态: Overlay共享状态,
    重绘信号: Overlay重绘信号,
    当前范围: 窗口范围,
    hwnd: Option<HWND>,
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, 上下文: &egui::Context, _frame: &mut eframe::Frame) {
        注册重绘上下文(&self.重绘信号, 上下文);

        if 上下文.input(|输入| 输入.key_pressed(egui::Key::Escape)) {
            上下文.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let Ok(mut 状态) = self.状态.lock() else {
            return;
        };

        if 状态.需要重定位 {
            let 目标范围 = if 状态.显示 {
                let 范围 = 状态
                    .锚点
                    .map(获取显示范围)
                    .unwrap_or(窗口范围 {
                        原点: pos2(0.0, 0.0),
                        大小: 默认显示大小,
                    });
                状态.窗口原点 = 范围.原点;
                范围
            } else {
                隐藏范围()
            };

            if self.当前范围 != 目标范围 {
                if let Some(hwnd) = self.hwnd {
                    同步窗口范围(hwnd, 目标范围);
                }
                self.当前范围 = 目标范围;
            }
            状态.需要重定位 = false;
        }

        let 显示 = 状态.显示;
        let 窗口原点 = 状态.窗口原点;
        let 轨迹点 = 状态.轨迹点.clone();
        let 按钮列表 = 状态.按钮列表.clone();
        let 选中按钮 = 状态.选中按钮;
        let 提示文字 = 状态.提示文字.clone();
        drop(状态);

        if 显示 {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(上下文, |ui| {
                    let 画笔 = ui.painter();
                    let 偏移 = 窗口原点.to_vec2();
                    let 局部轨迹点: Vec<Pos2> = 轨迹点.iter().map(|点| *点 - 偏移).collect();

                    if 局部轨迹点.len() >= 2 {
                        画笔.add(egui::Shape::line(
                            局部轨迹点.clone(),
                            Stroke::new(3.0, Color32::from_white_alpha(190)),
                        ));
                    }

                    for (索引, 按钮) in 按钮列表.iter().enumerate() {
                        let 局部矩形 =
                            Rect::from_min_max(按钮.矩形.min - 偏移, 按钮.矩形.max - 偏移);
                        let 已选中 = Some(索引) == 选中按钮;
                        let 填充 = if 已选中 {
                            Color32::from_rgba_unmultiplied(80, 150, 255, 180)
                        } else {
                            Color32::from_rgba_unmultiplied(30, 30, 30, 150)
                        };
                        let 边框 = if 已选中 {
                            Color32::from_rgb(180, 220, 255)
                        } else {
                            Color32::from_white_alpha(120)
                        };
                        画笔.rect_filled(局部矩形, 8.0, 填充);
                        画笔.rect_stroke(
                            局部矩形,
                            8.0,
                            Stroke::new(1.0, 边框),
                            StrokeKind::Outside,
                        );
                        画笔.text(
                            局部矩形.center(),
                            Align2::CENTER_CENTER,
                            按钮.标题,
                            FontId::proportional(16.0),
                            Color32::WHITE,
                        );
                    }

                    if !提示文字.is_empty() {
                        let 位置 = 局部轨迹点
                            .last()
                            .copied()
                            .unwrap_or(pos2(默认显示大小.x / 2.0, 默认显示大小.y / 2.0))
                            + vec2(18.0, -18.0);
                        画笔.text(
                            位置,
                            Align2::LEFT_CENTER,
                            提示文字,
                            FontId::proportional(24.0),
                            Color32::WHITE,
                        );
                    }
                });
        }
    }
}

fn 设置窗口不抢焦点(创建上下文: &eframe::CreationContext<'_>) -> Option<HWND> {
    let Ok(窗口句柄) = 创建上下文.window_handle() else {
        return None;
    };
    let RawWindowHandle::Win32(窗口句柄) = 窗口句柄.as_raw() else {
        return None;
    };

    let hwnd = HWND(窗口句柄.hwnd.get() as *mut core::ffi::c_void);
    unsafe {
        let 样式 = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let 新样式 = 样式 | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, 新样式);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
    Some(hwnd)
}

fn 同步窗口范围(hwnd: HWND, 范围: 窗口范围) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            范围.原点.x as i32,
            范围.原点.y as i32,
            范围.大小.x as i32,
            范围.大小.y as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

fn 获取显示范围(锚点: Pos2) -> 窗口范围 {
    unsafe {
        let monitor = MonitorFromPoint(
            POINT {
                x: 锚点.x as i32,
                y: 锚点.y as i32,
            },
            MONITOR_DEFAULTTONEAREST,
        );
        let mut 信息 = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(monitor, &mut 信息).as_bool() {
            let 矩形 = 信息.rcWork;
            let 宽 = (矩形.right - 矩形.left) as f32;
            let 高 = (矩形.bottom - 矩形.top) as f32;
            if 宽 > 0.0 && 高 > 0.0 {
                return 窗口范围 {
                    原点: pos2(矩形.left as f32, 矩形.top as f32),
                    大小: vec2(宽, 高),
                };
            }
        }
    }

    窗口范围 {
        原点: pos2(锚点.x - 默认显示大小.x / 2.0, 锚点.y - 默认显示大小.y / 2.0),
        大小: 默认显示大小,
    }
}

fn 设置字体(上下文: &egui::Context) {
    let Some((字体路径, 字体数据)) = 读取字体文件() else {
        eprintln!("未找到 font 目录中的字体文件，继续使用 egui 默认字体");
        return;
    };

    let mut 字体定义 = FontDefinitions::default();
    字体定义.font_data.insert(
        "mouse_next_font".to_owned(),
        Arc::new(FontData::from_owned(字体数据)),
    );
    字体定义
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "mouse_next_font".to_owned());
    字体定义
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "mouse_next_font".to_owned());
    上下文.set_fonts(字体定义);
    println!("浮窗字体: {}", 字体路径.display());
}

fn 读取字体文件() -> Option<(PathBuf, Vec<u8>)> {
    let mut 字体路径列表 = std::fs::read_dir("font")
        .ok()?
        .filter_map(|结果| 结果.ok().map(|项| 项.path()))
        .filter(|路径| {
            路径
                .extension()
                .and_then(|后缀| 后缀.to_str())
                .map(|后缀| matches!(后缀.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    字体路径列表.sort();
    let 字体路径 = 字体路径列表.into_iter().next()?;
    let 字体数据 = std::fs::read(&字体路径).ok()?;
    Some((字体路径, 字体数据))
}
