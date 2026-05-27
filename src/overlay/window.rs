use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect,
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

use crate::config::显示鼠标轨迹;
use super::animation::{混合颜色, 宫格动画器, 缩放矩形, 绘制标签};
use super::repaint::{Overlay重绘信号, 注册重绘上下文};
use super::state::{Overlay共享状态, 宫格类型, 隐藏窗口位置, 隐藏窗口大小};

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
                宫格动画: 宫格动画器::新建(),
                上次显示: false,
                上次宫格快照: None,
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
    宫格动画: 宫格动画器,
    上次显示: bool,
    上次宫格快照: Option<(i32, i32)>,
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
        let 宫格中心坐标 = 状态.宫格中心坐标;
        let 宫格列表 = 状态.宫格列表.clone();
        let 悬停宫格 = 状态.悬停宫格;
        let 最近方向 = 状态.最近方向;
        drop(状态);

        if 显示 && !self.上次显示 {
            self.宫格动画.重置();
            self.上次宫格快照 = None;
        }
        if !显示 && self.上次显示 {
            self.宫格动画.重置();
            self.上次宫格快照 = None;
        }
        self.上次显示 = 显示;

        if 显示 {
            let 当前时间 = 上下文.input(|输入| 输入.time);
            let 帧间隔 = 上下文.input(|输入| 输入.stable_dt).clamp(0.001, 0.05);
            let 当前快照 = 宫格中心坐标;
            if self.上次宫格快照 != Some(当前快照) {
                self.宫格动画.同步(当前时间, &宫格列表, 最近方向);
                self.上次宫格快照 = Some(当前快照);
            }
            if self.宫格动画.更新(当前时间, 帧间隔) {
                上下文.request_repaint();
            }

            let 悬停世界坐标 =
                悬停宫格.and_then(|索引| 宫格列表.get(索引).map(|项| 项.世界坐标));
            let 绘制列表 = self.宫格动画.绘制列表(当前时间);

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(上下文, |ui| {
                    let 画笔 = ui.painter();
                    let 偏移 = 窗口原点.to_vec2();
                    let 局部轨迹点: Vec<Pos2> = 轨迹点.iter().map(|点| *点 - 偏移).collect();

                    if 显示鼠标轨迹 && 局部轨迹点.len() >= 2 {
                        画笔.add(egui::Shape::line(
                            局部轨迹点.clone(),
                            Stroke::new(2.0, Color32::from_white_alpha(140)),
                        ));
                    }

                    for 参数 in 绘制列表 {
                        let 局部矩形 = Rect::from_min_max(
                            参数.矩形.min - 偏移,
                            参数.矩形.max - 偏移,
                        );
                        let 缩放矩形 = 缩放矩形(局部矩形, 参数.缩放);
                        let 已悬停 = 悬停世界坐标 == Some(参数.世界坐标);
                        绘制浅色玻璃宫格(
                            &画笔,
                            缩放矩形,
                            参数.类型,
                            已悬停,
                            参数.不透明度,
                        );
                        if 参数.标签.is_some() || 参数.旧标签.is_some() {
                            let 文字色 = if 已悬停 && 参数.类型 != 宫格类型::空白 {
                                混合颜色(Color32::from_rgb(30, 90, 210), 参数.不透明度)
                            } else {
                                混合颜色(Color32::from_rgb(40, 45, 55), 参数.不透明度)
                            };
                            let 字号 = 标签字号(参数.标签);
                            绘制标签(
                                &画笔,
                                缩放矩形.center(),
                                参数.旧标签,
                                参数.标签,
                                参数.标签变换进度,
                                字号,
                                文字色,
                            );
                        }
                    }
                });
        }
    }
}

fn 标签字号(标签: Option<&str>) -> f32 {
    match 标签.map(|文字| 文字.chars().count()).unwrap_or(0) {
        0 | 1 => 28.0,
        _ => 20.0,
    }
}

fn 绘制浅色玻璃宫格(
    画笔: &egui::Painter,
    矩形: Rect,
    类型: 宫格类型,
    已悬停: bool,
    不透明度: f32,
) {
    let 是空白 = 类型 == 宫格类型::空白;
    let 是中心 = 类型 == 宫格类型::中心;
    let 可交互 = !是空白 && !是中心;

    let 阴影 = 矩形.translate(vec2(0.0, 1.5)).expand(0.5);
    画笔.rect_filled(
        阴影,
        宫格圆角,
        混合颜色(Color32::from_rgba_unmultiplied(0, 0, 0, 18), 不透明度),
    );

    let 填充 = if 已悬停 && 可交互 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 118)
    } else if 是中心 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 92)
    } else if 是空白 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 28)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 72)
    };
    画笔.rect_filled(矩形, 宫格圆角, 混合颜色(填充, 不透明度));

    if !是空白 {
        let 高光条 = Rect::from_min_max(
            pos2(矩形.min.x + 8.0, 矩形.min.y + 5.0),
            pos2(矩形.max.x - 8.0, 矩形.min.y + 6.5),
        );
        画笔.rect_filled(
            高光条,
            1.0,
            混合颜色(Color32::from_rgba_unmultiplied(255, 255, 255, 90), 不透明度),
        );
    }

    let 边框 = if 已悬停 && 可交互 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 200)
    } else if 是空白 {
        Color32::from_rgba_unmultiplied(255, 255, 255, 45)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 130)
    };
    画笔.rect_stroke(
        矩形,
        宫格圆角,
        Stroke::new(if 已悬停 && 可交互 { 1.2 } else { 0.75 }, 混合颜色(边框, 不透明度)),
        StrokeKind::Outside,
    );

    if 已悬停 && 可交互 {
        画笔.rect_stroke(
            矩形.shrink(3.0),
            宫格圆角 - 3.0,
            Stroke::new(
                1.0,
                混合颜色(Color32::from_rgba_unmultiplied(80, 150, 255, 90), 不透明度),
            ),
            StrokeKind::Inside,
        );
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
