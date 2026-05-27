use std::collections::HashMap;

use eframe::egui::{Pos2, Rect, vec2};

use super::state::{宫格类型, 宫格项};

const 标签变换时长: f32 = 0.36;
const 进入交错秒: f32 = 0.075;
const 淡入淡出速度: f32 = 5.5;
const 缩放过渡速度: f32 = 6.0;
const 矩形过渡速度: f32 = 9.0;

#[derive(Debug, Clone)]
struct 动画格 {
    世界坐标: (i32, i32),
    矩形: Rect,
    目标矩形: Rect,
    类型: 宫格类型,
    标签: Option<&'static str>,
    旧标签: Option<&'static str>,
    标签变换开始: Option<f64>,
    不透明度: f32,
    目标不透明度: f32,
    缩放: f32,
    目标缩放: f32,
    进入解锁时间: f64,
    正在退出: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct 格绘制参数 {
    pub 世界坐标: (i32, i32),
    pub 矩形: Rect,
    pub 类型: 宫格类型,
    pub 标签: Option<&'static str>,
    pub 旧标签: Option<&'static str>,
    pub 标签变换进度: f32,
    pub 不透明度: f32,
    pub 缩放: f32,
}

pub struct 宫格动画器 {
    格子: HashMap<(i32, i32), 动画格>,
}

impl 宫格动画器 {
    pub fn 新建() -> Self {
        Self {
            格子: HashMap::new(),
        }
    }

    pub fn 重置(&mut self) {
        self.格子.clear();
    }

    pub fn 同步(&mut self, 当前时间: f64, 新列表: &[宫格项], 触发方向: Option<crate::input::方向>) {
        let 新映射: HashMap<(i32, i32), &宫格项> =
            新列表.iter().map(|项| (项.世界坐标, 项)).collect();

        for 动画 in self.格子.values_mut() {
            if !新映射.contains_key(&动画.世界坐标) && !动画.正在退出 {
                动画.正在退出 = true;
                动画.目标不透明度 = 0.0;
                动画.目标缩放 = 0.9;
                动画.旧标签 = None;
                动画.标签变换开始 = None;
            }
        }

        let mut 新进入坐标 = Vec::new();
        for 坐标 in 新映射.keys() {
            match self.格子.get(坐标) {
                None => 新进入坐标.push(*坐标),
                Some(动画) if 动画.正在退出 => 新进入坐标.push(*坐标),
                Some(_) => {}
            }
        }
        新进入坐标.sort_by(|a, b| 进入排序键(触发方向, *a).cmp(&进入排序键(触发方向, *b)));

        for (坐标, 项) in &新映射 {
            if let Some(动画) = self.格子.get_mut(坐标) {
                if 动画.正在退出 {
                    continue;
                }
                动画.目标矩形 = 项.矩形;
                动画.类型 = 项.类型;
                动画.目标不透明度 = 1.0;
                动画.目标缩放 = 1.0;
                if 动画.标签 != 项.标签 {
                    动画.旧标签 = 动画.标签;
                    动画.标签 = 项.标签;
                    动画.标签变换开始 = Some(当前时间);
                }
                continue;
            }
        }

        let mut 进入序号 = 0u32;
        for 坐标 in 新进入坐标 {
            let 项 = 新映射[&坐标];
            let 延迟 = 进入序号 as f32 * 进入交错秒;
            进入序号 += 1;
            self.格子.insert(
                坐标,
                动画格 {
                    世界坐标: 坐标,
                    矩形: 项.矩形,
                    目标矩形: 项.矩形,
                    类型: 项.类型,
                    标签: 项.标签,
                    旧标签: None,
                    标签变换开始: None,
                    不透明度: 0.0,
                    目标不透明度: 1.0,
                    缩放: 0.9,
                    目标缩放: 1.0,
                    进入解锁时间: 当前时间 + 延迟 as f64,
                    正在退出: false,
                },
            );
        }
    }

    pub fn 更新(&mut self, 当前时间: f64, 帧间隔: f32) -> bool {
        self.格子.retain(|_, 动画| {
            if 当前时间 >= 动画.进入解锁时间 {
                动画.不透明度 = 平滑逼近(动画.不透明度, 动画.目标不透明度, 帧间隔, 淡入淡出速度);
                动画.缩放 = 平滑逼近(动画.缩放, 动画.目标缩放, 帧间隔, 缩放过渡速度);
            }

            if !动画.正在退出 {
                动画.矩形 = 矩形平滑逼近(动画.矩形, 动画.目标矩形, 帧间隔, 矩形过渡速度);
            }

            if !(动画.正在退出 && 动画.不透明度 < 0.015 && 当前时间 >= 动画.进入解锁时间) {
                return true;
            }
            false
        });
        self.有活跃动画(当前时间)
    }

    pub fn 有活跃动画(&self, 当前时间: f64) -> bool {
        self.格子.values().any(|动画| 动画.仍在动画中(当前时间))
    }

    pub fn 绘制列表(&self, 当前时间: f64) -> Vec<格绘制参数> {
        let mut 列表: Vec<_> = self
            .格子
            .values()
            .filter(|动画| 动画.不透明度 > 0.01)
            .map(|动画| 动画.到绘制参数(当前时间))
            .collect();
        列表.sort_by(|a, b| {
            a.不透明度
                .partial_cmp(&b.不透明度)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        列表
    }
}

impl 动画格 {
    fn 仍在动画中(&self, 当前时间: f64) -> bool {
        if 当前时间 < self.进入解锁时间 {
            return true;
        }
        if self.正在退出 {
            return self.不透明度 > 0.015;
        }
        if (self.不透明度 - self.目标不透明度).abs() > 0.01 {
            return true;
        }
        if (self.缩放 - self.目标缩放).abs() > 0.01 {
            return true;
        }
        if 矩形距离(self.矩形, self.目标矩形) > 0.5 {
            return true;
        }
        self.标签变换开始
            .map(|开始| {
                let 进度 = ((当前时间 - 开始) as f32 / 标签变换时长).clamp(0.0, 1.0);
                进度 < 1.0
            })
            .unwrap_or(false)
    }

    fn 到绘制参数(&self, 当前时间: f64) -> 格绘制参数 {
        let 标签变换进度 = self
            .标签变换开始
            .map(|开始| 缓入缓出(((当前时间 - 开始) as f32 / 标签变换时长).clamp(0.0, 1.0)))
            .unwrap_or(1.0);

        格绘制参数 {
            世界坐标: self.世界坐标,
            矩形: self.矩形,
            类型: self.类型,
            标签: self.标签,
            旧标签: if 标签变换进度 < 1.0 {
                self.旧标签
            } else {
                None
            },
            标签变换进度,
            不透明度: self.不透明度,
            缩放: self.缩放,
        }
    }
}

fn 进入排序键(方向: Option<crate::input::方向>, 坐标: (i32, i32)) -> i32 {
    match 方向 {
        Some(crate::input::方向::右) => -坐标.0,
        Some(crate::input::方向::左) => 坐标.0,
        Some(crate::input::方向::下) => -坐标.1,
        Some(crate::input::方向::上) => 坐标.1,
        _ => 坐标.0.abs() + 坐标.1.abs(),
    }
}

pub(crate) fn 平滑逼近(当前: f32, 目标: f32, 帧间隔: f32, 速度: f32) -> f32 {
    if 帧间隔 <= 0.0 {
        return 目标;
    }
    let 系数 = 1.0 - (-速度 * 帧间隔).exp();
    当前 + (目标 - 当前) * 系数
}

fn 矩形平滑逼近(当前: Rect, 目标: Rect, 帧间隔: f32, 速度: f32) -> Rect {
    if 帧间隔 <= 0.0 {
        return 目标;
    }
    let 系数 = 1.0 - (-速度 * 帧间隔).exp();
    Rect::from_min_max(
        当前.min + (目标.min - 当前.min) * 系数,
        当前.max + (目标.max - 当前.max) * 系数,
    )
}

fn 矩形距离(a: Rect, b: Rect) -> f32 {
    a.center().distance(b.center())
}

fn 缓入缓出(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

pub fn 缩放矩形(矩形: Rect, 缩放: f32) -> Rect {
    let 中心 = 矩形.center();
    let 半尺寸 = 矩形.size() * 0.5 * 缩放;
    Rect::from_min_max(中心 - 半尺寸, 中心 + 半尺寸)
}

pub fn 混合颜色(颜色: eframe::egui::Color32, 不透明度: f32) -> eframe::egui::Color32 {
    eframe::egui::Color32::from_rgba_unmultiplied(
        颜色.r(),
        颜色.g(),
        颜色.b(),
        (颜色.a() as f32 * 不透明度).round() as u8,
    )
}

pub fn 绘制标签(
    画笔: &eframe::egui::Painter,
    中心: Pos2,
    旧标签: Option<&str>,
    新标签: Option<&str>,
    变换进度: f32,
    字号: f32,
    颜色: eframe::egui::Color32,
) {
    use eframe::egui::{Align2, FontId};

    if 旧标签.is_none() || 变换进度 >= 1.0 {
        if let Some(文字) = 新标签 {
            画笔.text(
                中心,
                Align2::CENTER_CENTER,
                文字,
                FontId::proportional(字号),
                颜色,
            );
        }
        return;
    }

    if let Some(文字) = 旧标签 {
        let 偏移 = vec2(0.0, -10.0 * 变换进度);
        let 透明 = 混合颜色(颜色, 1.0 - 变换进度);
        画笔.text(
            中心 + 偏移,
            Align2::CENTER_CENTER,
            文字,
            FontId::proportional(字号 * (1.0 - 0.06 * 变换进度)),
            透明,
        );
    }

    if let Some(文字) = 新标签 {
        let 偏移 = vec2(0.0, 10.0 * (1.0 - 变换进度));
        let 透明 = 混合颜色(颜色, 变换进度);
        画笔.text(
            中心 + 偏移,
            Align2::CENTER_CENTER,
            文字,
            FontId::proportional(字号 * (0.94 + 0.06 * 变换进度)),
            透明,
        );
    }
}
