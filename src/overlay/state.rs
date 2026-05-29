use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui::{pos2, ColorImage, Pos2};

use crate::action::{
    处理进入槽, 命中槽, 匹配松手动作, 匹配触碰动作, 同槽重复动作, 全部槽位, 宫格事件, 宫格槽, 层,
    槽位中心, 槽离开需截屏,
};

pub const 隐藏窗口大小: eframe::egui::Vec2 = eframe::egui::vec2(1.0, 1.0);
pub const 隐藏窗口位置: Pos2 = pos2(0.0, 0.0);

const 颜色褪色速度: f32 = 2.5;

pub type Overlay共享状态 = Arc<Mutex<Overlay状态>>;

#[derive(Debug, PartialEq)]
pub enum Overlay事件结果 {
    无,
    会话开始,
    会话结束 {
        补发位置: Option<Pos2>,
    },
    转发(宫格事件),
    离格截屏,
    会话结束流程 {
        松手: Option<宫格事件>,
        补发位置: Option<Pos2>,
    },
}

#[derive(Debug, Clone)]
pub struct 冻结缓存条目 {
    pub 图像: Arc<ColorImage>,
    pub 屏幕左: i32,
    pub 屏幕上: i32,
}

#[derive(Debug, Clone)]
pub struct 格动画 {
    pub 槽: 宫格槽,
    pub 中心: Pos2,
    pub 颜色进度: f32,
}

#[derive(Debug)]
pub struct Overlay状态 {
    pub 显示: bool,
    pub 锚点: Option<Pos2>,
    pub 窗口原点: Pos2,
    pub 当前层: 层,
    pub 步进队列: Vec<(层, 宫格槽)>,
    pub 当前槽: Option<宫格槽>,
    pub 上次槽: Option<宫格槽>,
    pub 格子们: Vec<格动画>,
    pub 需要重定位: bool,
    pub 已触发手势: bool,
    pub 鼠标位置: Pos2,
    pub 面板会话中: Arc<AtomicBool>,
    pub overlay句柄: Option<isize>,
    pub 缓存冻结帧: Option<Arc<冻结缓存条目>>,
    pub 缓存刷新代际: u64,
    pub 冻结帧: Option<ColorImage>,
    pub 冻结帧屏幕原点: Option<Pos2>,
    pub 冻结不透明度: f32,
    pub 冻结淡出中: bool,
}

impl Overlay状态 {
    pub fn 新建共享(面板会话中: Arc<AtomicBool>) -> Overlay共享状态 {
        Arc::new(Mutex::new(Self::默认(面板会话中)))
    }

    pub fn 默认(面板会话中: Arc<AtomicBool>) -> Self {
        Self {
            显示: false,
            锚点: None,
            窗口原点: 隐藏窗口位置,
            当前层: 0,
            步进队列: Vec::new(),
            当前槽: None,
            上次槽: None,
            格子们: Vec::new(),
            需要重定位: true,
            已触发手势: false,
            鼠标位置: Pos2::ZERO,
            面板会话中,
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

    pub fn 处理原始事件(&mut self, 事件: crate::input::原始鼠标事件) -> Overlay事件结果 {
        match 事件 {
            crate::input::原始鼠标事件::右键按下 { x, y } => {
                let 点 = pos2(x as f32, y as f32);
                self.显示 = true;
                self.面板会话中.store(true, Ordering::Relaxed);
                self.锚点 = Some(点);
                self.鼠标位置 = 点;
                self.当前层 = 0;
                self.步进队列.clear();
                self.当前槽 = None;
                self.上次槽 = None;
                self.已触发手势 = false;
                self.清除冻结层();
                self.重建九宫格(点);
                self.需要重定位 = true;
                Overlay事件结果::会话开始
            }
            crate::input::原始鼠标事件::鼠标移动 { x, y } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 点 = pos2(x as f32, y as f32);
                self.鼠标位置 = 点;
                self.处理槽变化(点)
            }
            crate::input::原始鼠标事件::左键按下 { x, y } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 点 = pos2(x as f32, y as f32);
                self.鼠标位置 = 点;
                let 锚点 = self.锚点.unwrap_or(点);
                if let Some(槽) = 命中槽(锚点, 点) {
                    if 同槽重复动作(self.当前层, 槽).is_some() {
                        self.已触发手势 = true;
                        if let Some(格) = self.格子们.iter_mut().find(|格| 格.槽 == 槽) {
                            格.颜色进度 = 1.0;
                        }
                        return Overlay事件结果::转发(宫格事件::同槽重复(槽));
                    }
                }
                Overlay事件结果::无
            }
            crate::input::原始鼠标事件::右键抬起 { x: _, y: _ } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 松手 = self.当前槽.and_then(|槽| {
                    if 匹配松手动作(&self.步进队列).is_some() {
                        self.已触发手势 = true;
                        Some(宫格事件::松手 {
                            槽,
                            层: self.当前层,
                            队列: self.步进队列.clone(),
                        })
                    } else {
                        None
                    }
                });
                let 补发位置 = self.关闭();
                Overlay事件结果::会话结束流程 {
                    松手,
                    补发位置,
                }
            }
        }
    }

    fn 关闭(&mut self) -> Option<Pos2> {
        let 补发位置 = if self.已触发手势 {
            None
        } else {
            self.锚点
        };
        self.显示 = false;
        self.面板会话中.store(false, Ordering::Relaxed);
        self.锚点 = None;
        self.当前层 = 0;
        self.步进队列.clear();
        self.当前槽 = None;
        self.上次槽 = None;
        self.格子们.clear();
        self.已触发手势 = false;
        self.清除冻结层();
        self.需要重定位 = true;
        补发位置
    }

    fn 重建九宫格(&mut self, 锚点: Pos2) {
        self.格子们 = 全部槽位()
            .iter()
            .map(|项| 格动画 {
                槽: 项.槽,
                中心: 槽位中心(锚点, 项.槽),
                颜色进度: 0.0,
            })
            .collect();
    }

    fn 处理槽变化(&mut self, 点: Pos2) -> Overlay事件结果 {
        let 锚点 = match self.锚点 {
            Some(锚) => 锚,
            None => return Overlay事件结果::无,
        };
        let 新槽 = 命中槽(锚点, 点);
        if 新槽 == self.当前槽 {
            return Overlay事件结果::无;
        }

        let mut 结果 = Overlay事件结果::无;

        if let Some(旧) = self.当前槽 {
            if 槽离开需截屏(旧) {
                结果 = Overlay事件结果::离格截屏;
            }
        }

        self.上次槽 = self.当前槽;
        self.当前槽 = 新槽;

        let Some(槽) = 新槽 else {
            return 结果;
        };

        if let Some(新层) = 处理进入槽(self.当前层, &mut self.步进队列, 槽) {
            self.当前层 = 新层;
            if let Some(格) = self.格子们.iter_mut().find(|格| 格.槽 == 槽) {
                格.颜色进度 = 1.0;
            }
            self.已触发手势 = true;
            let 进入事件 = 宫格事件::进入 {
                槽,
                层: self.当前层,
                队列: self.步进队列.clone(),
            };
            if 匹配触碰动作(&self.步进队列).is_some() {
                return Overlay事件结果::转发(进入事件);
            }
            if 结果 != Overlay事件结果::无 {
                return 结果;
            }
            return Overlay事件结果::转发(进入事件);
        }

        结果
    }

    pub fn 推进动画(&mut self, _当前时间: f64, 帧间隔: f32) -> bool {
        if !self.显示 {
            return false;
        }
        let mut 活跃 = false;
        for 格 in &mut self.格子们 {
            if 格.颜色进度 > 0.001 {
                格.颜色进度 = 平滑逼近(格.颜色进度, 0.0, 帧间隔, 颜色褪色速度);
                if 格.颜色进度 < 0.003 {
                    格.颜色进度 = 0.0;
                }
                活跃 = true;
            }
        }
        活跃
    }
}

pub(crate) fn 平滑逼近(当前: f32, 目标: f32, 帧间隔: f32, 速度: f32) -> f32 {
    if 帧间隔 <= 0.0 {
        return 目标;
    }
    let 系数 = 1.0 - (-速度 * 帧间隔).exp();
    当前 + (目标 - 当前) * 系数
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn 会话标志() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    #[test]
    fn 右键创建九格() {
        let mut 状态 = Overlay状态::默认(会话标志());
        状态.处理原始事件(crate::input::原始鼠标事件::右键按下 { x: 200, y: 200 });
        assert!(状态.显示);
        assert_eq!(状态.格子们.len(), 9);
    }

    #[test]
    fn 进入右格转发() {
        let mut 状态 = Overlay状态::默认(会话标志());
        状态.处理原始事件(crate::input::原始鼠标事件::右键按下 { x: 200, y: 200 });
        let 右中心 = 槽位中心(pos2(200.0, 200.0), 宫格槽::右);
        let 结果 = 状态.处理原始事件(crate::input::原始鼠标事件::鼠标移动 {
            x: 右中心.x as i32,
            y: 右中心.y as i32,
        });
        assert!(matches!(结果, Overlay事件结果::转发(宫格事件::进入 { .. })));
    }
}
