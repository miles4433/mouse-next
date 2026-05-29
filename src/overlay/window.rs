use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect,
    Stroke, StrokeKind, TextureHandle, Vec2,
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

use crate::action::{槽显示状态, 格符号, 显示级别};
use crate::config::{格边长, 冻结层峰值不透明度, 冻结淡出速度};
use super::repaint::{Overlay重绘信号, 注册重绘上下文};
use super::state::{Overlay共享状态, 隐藏窗口位置, 隐藏窗口大小, 平滑逼近};

const 默认显示大小: egui::Vec2 = egui::vec2(520.0, 320.0);
const 显示区域比例: f32 = 0.99;
const 宫格圆角: f32 = 18.0;

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
                上次显示: false,
                冻结纹理: None,
                冻结帧屏幕原点: None,
                冻结帧像素大小: None,
                刚重定位: false,
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
    上次显示: bool,
    冻结纹理: Option<TextureHandle>,
    冻结帧屏幕原点: Option<Pos2>,
    冻结帧像素大小: Option<[usize; 2]>,
    刚重定位: bool,
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

        if let Some(hwnd) = self.hwnd {
            状态.overlay句柄 = Some(hwnd.0 as isize);
        }

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
                self.刚重定位 = true;
            }
            状态.需要重定位 = false;
        }

        let 帧间隔 = 上下文
            .input(|输入| 输入.stable_dt)
            .clamp(0.001, 0.05);
        let 当前时间 = 上下文.input(|输入| 输入.time);

        if 状态.推进动画(当前时间, 帧间隔) {
            上下文.request_repaint();
        }

        // 冻结帧纹理上传
        if let Some(图像) = 状态.冻结帧.take() {
            let 屏幕原点 = 状态.冻结帧屏幕原点.take();
            let 像素大小 = 图像.size;
            self.冻结纹理 = Some(上下文.load_texture(
                format!("freeze_{}", 当前时间),
                图像,
                egui::TextureOptions::NEAREST,
            ));
            self.冻结帧屏幕原点 = 屏幕原点;
            self.冻结帧像素大小 = Some(像素大小);
            状态.冻结不透明度 = 冻结层峰值不透明度;
            状态.冻结淡出中 = true;
            上下文.request_repaint();
        }

        if 状态.冻结淡出中 {
            状态.冻结不透明度 =
                平滑逼近(状态.冻结不透明度, 0.0, 帧间隔, 冻结淡出速度());
            if 状态.冻结不透明度 < 0.015 {
                状态.冻结不透明度 = 0.0;
                状态.冻结淡出中 = false;
                self.冻结纹理 = None;
                self.冻结帧屏幕原点 = None;
                self.冻结帧像素大小 = None;
            } else {
                上下文.request_repaint();
            }
        }

        let 显示 = 状态.显示;
        let 窗口原点 = 状态.窗口原点;
        let 冻结不透明度 = 状态.冻结不透明度;
        let 当前层 = 状态.当前层;
        let 队列 = 状态.步进队列.clone();
        let 悬停槽 = 状态.当前槽;
        let 格子快照: Vec<格绘制快照> = 状态
            .格子们
            .iter()
            .map(|格| {
                let 显示 = 槽显示状态(当前层, &队列, 悬停槽, 格.槽);
                格绘制快照 {
                    中心: 格.中心,
                    颜色进度: 格.颜色进度,
                    显示,
                }
            })
            .collect();

        drop(状态);

        if self.刚重定位 {
            self.刚重定位 = false;
            上下文.request_repaint();
            return;
        }

        if !显示 && self.上次显示 {
            self.冻结纹理 = None;
            self.冻结帧屏幕原点 = None;
            self.冻结帧像素大小 = None;
        }
        self.上次显示 = 显示;

        if 显示 {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(上下文, |ui| {
                    let 画笔 = ui.painter();
                    let 区域 = ui.max_rect();

                    if let Some(纹理) = &self.冻结纹理 {
                        if 冻结不透明度 > 0.01 {
                            if let (Some(屏幕原点), Some(像素大小)) =
                                (self.冻结帧屏幕原点, self.冻结帧像素大小)
                            {
                                绘制对齐冻结层(
                                    &画笔,
                                    纹理,
                                    像素大小,
                                    屏幕原点,
                                    窗口原点,
                                    区域,
                                    上下文.pixels_per_point(),
                                    冻结不透明度,
                                );
                            }
                        }
                    }

                    let 偏移 = 窗口原点.to_vec2();
                    let 半 = 格边长 / 2.0;

                    for 格 in &格子快照 {
                        let 局部矩形 = Rect::from_min_max(
                            pos2(格.中心.x - 半 - 偏移.x, 格.中心.y - 半 - 偏移.y),
                            pos2(格.中心.x + 半 - 偏移.x, 格.中心.y + 半 - 偏移.y),
                        );
                        绘制单格(&画笔, 局部矩形, &格.显示, 格.颜色进度, 上下文);
                    }
                });
        }
    }
}

#[derive(Clone)]
struct 格绘制快照 {
    中心: Pos2,
    颜色进度: f32,
    显示: crate::action::格显示,
}

// ── 绘制 ──

fn 绘制对齐冻结层(
    画笔: &egui::Painter,
    纹理: &TextureHandle,
    像素大小: [usize; 2],
    截屏屏幕原点: Pos2,
    overlay屏幕原点: Pos2,
    裁剪区域: Rect,
    像素比例: f32,
    不透明度: f32,
) {
    let 宽 = 像素大小[0] as f32;
    let 高 = 像素大小[1] as f32;
    if 宽 <= 0.0 || 高 <= 0.0 || 像素比例 <= 0.0 {
        return;
    }

    let 目标 = Rect::from_min_size(
        pos2(
            (截屏屏幕原点.x - overlay屏幕原点.x) / 像素比例,
            (截屏屏幕原点.y - overlay屏幕原点.y) / 像素比例,
        ),
        vec2(宽 / 像素比例, 高 / 像素比例),
    );

    let 可见 = 目标.intersect(裁剪区域);
    if 可见.width() <= 0.0 || 可见.height() <= 0.0 {
        return;
    }

    let uv = Rect::from_min_max(
        pos2(
            (可见.min.x - 目标.min.x) / 目标.width(),
            (可见.min.y - 目标.min.y) / 目标.height(),
        ),
        pos2(
            (可见.max.x - 目标.min.x) / 目标.width(),
            (可见.max.y - 目标.min.y) / 目标.height(),
        ),
    );

    画笔.image(
        纹理.id(),
        可见,
        uv,
        Color32::WHITE.gamma_multiply(不透明度),
    );
}

fn 符号文字(符号: 格符号) -> &'static str {
    match 符号 {
        格符号::左 => "←",
        格符号::右 => "→",
        格符号::上 => "↑",
        格符号::下 => "↓",
        格符号::关闭 => "×",
        格符号::恢复 => "↩",
        格符号::空 => "",
    }
}

fn 绘制单格(
    画笔: &egui::Painter,
    矩形: Rect,
    显示: &crate::action::格显示,
    颜色进度: f32,
    上下文: &egui::Context,
) {
    let 基础不透明 = match 显示.显示级别 {
        显示级别::空角弱显 => 0.35,
        显示级别::常显 => 1.0,
    };
    if 基础不透明 < 0.01 {
        return;
    }

    let 绿 = Color32::from_rgb(52, 199, 89);
    let 白底 = if 显示.悬停 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 240)
    } else {
        Color32::from_rgba_unmultiplied(242, 242, 247, 200)
    };

    let 填充色 = lerp_color32(绿, 白底, 1.0 - 颜色进度);
    let 最终填充 = alpha_color(填充色, 基础不透明);
    画笔.rect_filled(矩形, 宫格圆角, 最终填充);

    let 边框宽 = if 显示.悬停 { 1.2 } else { 0.6 };
    let 边框色 = Color32::from_rgba_unmultiplied(0, 0, 0, if 显示.悬停 { 40 } else { 18 });
    画笔.rect_stroke(
        矩形,
        宫格圆角,
        Stroke::new(边框宽, alpha_color(边框色, 基础不透明)),
        StrokeKind::Inside,
    );

    let 文案 = 显示
        .标签
        .map(|s| s.to_string())
        .unwrap_or_else(|| 符号文字(显示.符号).to_string());
    if !文案.is_empty() {
        let 字号 = 22.0;
        画笔.text(
            矩形.center(),
            egui::Align2::CENTER_CENTER,
            文案,
            egui::FontId::proportional(字号),
            alpha_color(Color32::from_rgb(30, 30, 30), 基础不透明),
        );
    }

    if 显示.锁定 {
        let 锁字号 = 14.0;
        let 锁位置 = pos2(矩形.max.x - 10.0, 矩形.min.y + 12.0);
        画笔.text(
            锁位置,
            egui::Align2::CENTER_CENTER,
            "🔒",
            egui::FontId::proportional(锁字号),
            alpha_color(Color32::from_rgb(120, 120, 120), 0.85),
        );
    }

    let _ = 上下文;
}

fn alpha_color(颜色: Color32, 不透明度: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        颜色.r(),
        颜色.g(),
        颜色.b(),
        (颜色.a() as f32 * 不透明度).round() as u8,
    )
}

fn lerp_color32(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

// ── 窗口管理 ──

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
                let 显示宽 = 宽 * 显示区域比例;
                let 显示高 = 高 * 显示区域比例;
                return 窗口范围 {
                    原点: pos2(
                        矩形.left as f32 + (宽 - 显示宽) / 2.0,
                        矩形.top as f32 + (高 - 显示高) / 2.0,
                    ),
                    大小: vec2(显示宽, 显示高),
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
