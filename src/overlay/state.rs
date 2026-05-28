use std::sync::{Arc, Mutex};

use eframe::egui::{pos2, vec2, ColorImage, Pos2, Rect};

use crate::action::会话预览;
use crate::config::{显示鼠标轨迹, 格边长, 格步长};
use crate::input::{原始鼠标事件, 方向};

pub const 隐藏窗口大小: eframe::egui::Vec2 = eframe::egui::vec2(1.0, 1.0);
pub const 隐藏窗口位置: Pos2 = pos2(0.0, 0.0);

const 采样距离: f32 = 2.0;
const 格过渡速度: f32 = 4.0;
const 颜色褪色速度: f32 = 2.5;
const 不透明度速度: f32 = 3.5;

pub type Overlay共享状态 = Arc<Mutex<Overlay状态>>;

#[derive(Debug)]
pub enum Overlay事件结果 {
    无,
    会话开始,
    会话结束 {
        补发位置: Option<Pos2>,
    },
    方向(方向),
}

#[derive(Debug, Clone)]
pub struct 冻结缓存条目 {
    pub 图像: Arc<ColorImage>,
    pub 屏幕左: i32,
    pub 屏幕上: i32,
}

/// 单个可见格子的动画状态
#[derive(Debug, Clone)]
pub struct 格动画 {
    pub 世界坐标: (i32, i32),
    pub 中心: Pos2,         // 当前屏幕位置（动画过渡中）
    pub 目标中心: Pos2,
    pub 颜色进度: f32,      // 1=激活色, 0=白色
    pub 不透明度: f32,
    pub 目标不透明度: f32,
}

#[derive(Debug)]
pub struct Overlay状态 {
    pub 显示: bool,
    pub 锚点: Option<Pos2>,
    pub 窗口原点: Pos2,
    pub 轨迹点: Vec<Pos2>,

    /// 当前中心格的世界坐标
    pub 当前坐标: (i32, i32),
    /// 当前中心格的屏幕位置（用于计算邻格位置）
    pub 当前中心: Pos2,

    /// 活跃的可见格子（用于渲染和动画）
    pub 格子们: Vec<格动画>,

    pub 需要重定位: bool,
    动作预览: 会话预览,
    已触发手势: bool,
    pub 鼠标位置: Pos2,

    pub overlay句柄: Option<isize>,
    pub 缓存冻结帧: Option<Arc<冻结缓存条目>>,
    pub 缓存刷新代际: u64,
    pub 冻结帧: Option<ColorImage>,
    pub 冻结帧屏幕原点: Option<Pos2>,
    pub 冻结不透明度: f32,
    pub 冻结淡出中: bool,
}

impl Overlay状态 {
    pub fn 新建共享() -> Overlay共享状态 {
        Arc::new(Mutex::new(Self::默认()))
    }

    pub fn 默认() -> Self {
        Self {
            显示: false,
            锚点: None,
            窗口原点: 隐藏窗口位置,
            轨迹点: Vec::new(),
            当前坐标: (0, 0),
            当前中心: Pos2::ZERO,
            格子们: Vec::new(),
            需要重定位: false,
            动作预览: 会话预览::新建(),
            已触发手势: false,
            鼠标位置: Pos2::ZERO,
            overlay句柄: None,
            缓存冻结帧: None,
            缓存刷新代际: 0,
            冻结帧: None,
            冻结帧屏幕原点: None,
            冻结不透明度: 0.0,
            冻结淡出中: false,
        }
    }

    pub fn 清除冻结层(&mut self) {
        self.缓存冻结帧 = None;
        self.冻结帧 = None;
        self.冻结帧屏幕原点 = None;
        self.冻结不透明度 = 0.0;
        self.冻结淡出中 = false;
    }

    pub fn 启用显示冻结帧(&mut self) {
        if let Some(条目) = &self.缓存冻结帧 {
            self.冻结帧 = Some(条目.图像.as_ref().clone());
            self.冻结帧屏幕原点 = Some(pos2(条目.屏幕左 as f32, 条目.屏幕上 as f32));
            self.冻结不透明度 = crate::config::冻结层峰值不透明度;
            self.冻结淡出中 = true;
        }
    }

    pub fn 推进缓存刷新代际(&mut self) -> u64 {
        self.缓存刷新代际 += 1;
        self.缓存刷新代际
    }

    pub fn 需要重绘(&self) -> bool {
        self.显示 || self.需要重定位 || self.冻结淡出中
    }

    pub fn 当前格矩形(&self) -> Rect {
        Rect::from_center_size(self.当前中心, vec2(格边长, 格边长))
    }

    pub fn 处理原始事件(&mut self, 事件: 原始鼠标事件) -> Overlay事件结果 {
        match 事件 {
            原始鼠标事件::右键按下 { x, y } => {
                let 点 = pos2(x as f32, y as f32);
                self.显示 = true;
                self.锚点 = Some(点);
                self.鼠标位置 = 点;
                self.当前坐标 = (0, 0);
                self.当前中心 = 点;
                self.动作预览.重置();
                self.已触发手势 = false;
                self.清除冻结层();
                self.轨迹点.clear();
                if 显示鼠标轨迹 {
                    self.轨迹点.push(点);
                }
                self.重建三格();
                self.需要重定位 = true;
                Overlay事件结果::会话开始
            }
            原始鼠标事件::鼠标移动 { x, y } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 点 = pos2(x as f32, y as f32);
                self.鼠标位置 = 点;
                if 显示鼠标轨迹 {
                    self.追加轨迹点(点);
                }
                self.检测邻格进入(点)
            }
            原始鼠标事件::右键抬起 { x, y } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 点 = pos2(x as f32, y as f32);
                if 显示鼠标轨迹 {
                    self.追加轨迹点(点);
                }
                let 补发位置 = self.关闭();
                Overlay事件结果::会话结束 { 补发位置 }
            }
        }
    }

    /// 推进动画：位置过渡、颜色衰减、不透明度过渡
    pub fn 推进动画(&mut self, _当前时间: f64, 帧间隔: f32) -> bool {
        if !self.显示 {
            return false;
        }
        let mut 活跃 = false;
        for 格 in &mut self.格子们 {
            // 不透明度
            if (格.不透明度 - 格.目标不透明度).abs() > 0.001 {
                格.不透明度 = 平滑逼近(格.不透明度, 格.目标不透明度, 帧间隔, 不透明度速度);
                活跃 = true;
            }
            // 位置
            if 格.中心.distance(格.目标中心) > 0.3 {
                格.中心 = pos2(
                    平滑逼近(格.中心.x, 格.目标中心.x, 帧间隔, 格过渡速度),
                    平滑逼近(格.中心.y, 格.目标中心.y, 帧间隔, 格过渡速度),
                );
                活跃 = true;
            }
            // 颜色衰减
            if 格.颜色进度 > 0.001 {
                格.颜色进度 = 平滑逼近(格.颜色进度, 0.0, 帧间隔, 颜色褪色速度);
                if 格.颜色进度 < 0.003 {
                    格.颜色进度 = 0.0;
                }
                活跃 = true;
            }
        }
        // 清理完全透明的格子
        self.格子们.retain(|格| 格.不透明度 > 0.005 || 格.目标不透明度 > 0.005);
        活跃
    }
}

// ── 内部辅助 ──

impl Overlay状态 {
    fn 追加轨迹点(&mut self, 点: Pos2) {
        if let Some(最后点) = self.轨迹点.last_mut() {
            if 最后点.distance(点) < 采样距离 {
                *最后点 = 点;
                return;
            }
        }
        self.轨迹点.push(点);
    }

    /// 根据当前坐标重建左/中/右三格（初始状态，无动画）
    fn 重建三格(&mut self) {
        self.格子们.clear();
        let 步长 = 格步长;
        for (dx, 是世界坐标) in [
            (-1, (self.当前坐标.0 - 1, self.当前坐标.1)),
            (0, self.当前坐标),
            (1, (self.当前坐标.0 + 1, self.当前坐标.1)),
        ] {
            let 目标 = self.当前中心 + vec2(dx as f32 * 步长, 0.0);
            self.格子们.push(格动画 {
                世界坐标: 是世界坐标,
                中心: 目标,
                目标中心: 目标,
                颜色进度: 0.0,
                不透明度: if dx == 0 { 1.0 } else { 0.55 },
                目标不透明度: if dx == 0 { 1.0 } else { 0.55 },
            });
        }
    }

    fn 检测邻格进入(&mut self, 点: Pos2) -> Overlay事件结果 {
        // 先检测左右邻格（有视觉反馈）
        let 步长 = 格步长;
        let 边长 = 格边长;

        // 左邻格
        let 左中心 = self.当前中心 + vec2(-步长, 0.0);
        let 左矩形 = Rect::from_center_size(左中心, vec2(边长, 边长));
        if 左矩形.contains(点) {
            return self.方向触发(方向::左);
        }

        // 右邻格
        let 右中心 = self.当前中心 + vec2(步长, 0.0);
        let 右矩形 = Rect::from_center_size(右中心, vec2(边长, 边长));
        if 右矩形.contains(点) {
            return self.方向触发(方向::右);
        }

        // 上下方向（隐形，仅检测）
        if self.当前格矩形().contains(点) {
            return Overlay事件结果::无;
        }

        for (方向, (dx, dy)) in &[
            (方向::上, (0, -1)),
            (方向::下, (0, 1)),
        ] {
            let 邻中心 = self.当前中心 + vec2(*dx as f32 * 步长, *dy as f32 * 步长);
            let 邻矩形 = Rect::from_center_size(邻中心, vec2(边长, 边长));
            if 邻矩形.contains(点) {
                return self.方向触发(*方向);
            }
        }

        Overlay事件结果::无
    }

    fn 方向触发(&mut self, 方向: 方向) -> Overlay事件结果 {
        let 步长 = 格步长;

        let (dx, dy) = match 方向 {
            方向::左 => (-1, 0),
            方向::右 => (1, 0),
            方向::上 => (0, -1),
            方向::下 => (0, 1),
            _ => return Overlay事件结果::无,
        };
        self.当前坐标 = (self.当前坐标.0 + dx, self.当前坐标.1 + dy);
        self.当前中心 = self.当前中心 + vec2(dx as f32 * 步长, dy as f32 * 步长);

        let 新三格坐标: Vec<(i32, i32)> = vec![
            (self.当前坐标.0 - 1, self.当前坐标.1),
            self.当前坐标,
            (self.当前坐标.0 + 1, self.当前坐标.1),
        ];

        for 格 in &mut self.格子们 {
            if 新三格坐标.contains(&格.世界坐标) {
                let x_offset = 格.世界坐标.0 - self.当前坐标.0;
                格.目标中心 = self.当前中心 + vec2(x_offset as f32 * 步长, 0.0);
                if 格.世界坐标 == self.当前坐标 {
                    格.颜色进度 = 1.0;
                    格.目标不透明度 = 1.0;
                }
            } else {
                格.目标不透明度 = 0.0;
            }
        }

        for 坐标 in &新三格坐标 {
            if !self.格子们.iter().any(|格| 格.世界坐标 == *坐标) {
                let x_offset = 坐标.0 - self.当前坐标.0;
                let 目标 = self.当前中心 + vec2(x_offset as f32 * 步长, 0.0);
                self.格子们.push(格动画 {
                    世界坐标: *坐标,
                    中心: 目标,
                    目标中心: 目标,
                    颜色进度: 0.0,
                    不透明度: 0.0,
                    目标不透明度: if x_offset == 0 { 1.0 } else { 0.55 },
                });
            }
        }

        self.动作预览.应用方向(方向);
        self.已触发手势 = true;
        Overlay事件结果::方向(方向)
    }
    fn 关闭(&mut self) -> Option<Pos2> {
        let 补发位置 = if self.已触发手势 {
            None
        } else {
            self.锚点
        };
        self.显示 = false;
        self.锚点 = None;
        self.当前坐标 = (0, 0);
        self.格子们.clear();
        self.动作预览.重置();
        self.已触发手势 = false;
        self.清除冻结层();
        self.需要重定位 = true;
        补发位置
    }
}

// ── 工具函数 ──

pub(crate) fn 平滑逼近(当前: f32, 目标: f32, 帧间隔: f32, 速度: f32) -> f32 {
    if 帧间隔 <= 0.0 {
        return 目标;
    }
    let 系数 = 1.0 - (-速度 * 帧间隔).exp();
    当前 + (目标 - 当前) * 系数
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;

    fn 按右键(状态: &mut Overlay状态, x: i32, y: i32) -> Overlay事件结果 {
        状态.处理原始事件(原始鼠标事件::右键按下 { x, y })
    }

    fn 移动(状态: &mut Overlay状态, x: i32, y: i32) -> Overlay事件结果 {
        状态.处理原始事件(原始鼠标事件::鼠标移动 { x, y })
    }

    #[test]
    fn 右键创建三格() {
        let mut 状态 = Overlay状态::默认();
        按右键(&mut 状态, 200, 200);
        assert!(状态.显示);
        assert_eq!(状态.格子们.len(), 3);
        // 中格不透明度 1.0，左右格 0.55
        let 中格 = 状态.格子们.iter().find(|格| 格.世界坐标 == (0, 0)).unwrap();
        assert_eq!(中格.不透明度, 1.0);
        let 左格 = 状态.格子们.iter().find(|格| 格.世界坐标 == (-1, 0)).unwrap();
        assert_eq!(左格.不透明度, 0.55);
    }

    #[test]
    fn 右移触发方向() {
        let mut 状态 = Overlay状态::默认();
        按右键(&mut 状态, 200, 200);
        let 结果 = 移动(&mut 状态, 306, 200); // 右邻格中心
        assert!(matches!(结果, Overlay事件结果::方向(方向::右)));
        assert_eq!(状态.当前坐标, (1, 0));
    }

    #[test]
    fn 右移后中格激活色() {
        let mut 状态 = Overlay状态::默认();
        按右键(&mut 状态, 200, 200);
        移动(&mut 状态, 306, 200);
        let 中格 = 状态.格子们.iter().find(|格| 格.世界坐标 == (1, 0)).unwrap();
        assert!(中格.颜色进度 > 0.99);
    }

    #[test]
    fn 颜色进度随时间衰减() {
        let mut 状态 = Overlay状态::默认();
        按右键(&mut 状态, 200, 200);
        移动(&mut 状态, 306, 200);
        for _ in 0..90 {
            状态.推进动画(0.0, 0.016);
        }
        let 中格 = 状态.格子们.iter().find(|格| 格.世界坐标 == (1, 0)).unwrap();
        assert!(中格.颜色进度 < 0.3);
    }

    #[test]
    fn 未触发手势结束补发() {
        let mut 状态 = Overlay状态::默认();
        按右键(&mut 状态, 10, 20);
        let 结果 = 状态.处理原始事件(原始鼠标事件::右键抬起 { x: 10, y: 20 });
        assert!(matches!(
            结果,
            Overlay事件结果::会话结束 { 补发位置: Some(_) }
        ));
    }

    #[test]
    fn 已触发手势结束不补发() {
        let mut 状态 = Overlay状态::默认();
        按右键(&mut 状态, 200, 200);
        移动(&mut 状态, 306, 200);
        let 结果 = 状态.处理原始事件(原始鼠标事件::右键抬起 { x: 306, y: 200 });
        assert!(matches!(
            结果,
            Overlay事件结果::会话结束 { 补发位置: None }
        ));
    }
}
