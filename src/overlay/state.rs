use std::sync::{Arc, Mutex};

use eframe::egui::{pos2, vec2, Pos2, Rect};

use crate::config::显示鼠标轨迹;
use crate::input::{原始鼠标事件, 方向};

pub const 隐藏窗口大小: eframe::egui::Vec2 = eframe::egui::vec2(1.0, 1.0);
pub const 隐藏窗口位置: Pos2 = pos2(0.0, 0.0);

const 采样距离: f32 = 2.0;
const 宫格边长: f32 = 96.0;
const 宫格间距: f32 = 10.0;

pub type Overlay共享状态 = Arc<Mutex<Overlay状态>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 宫格类型 {
    中心,
    上,
    下,
    左,
    右,
    空白,
}

#[derive(Debug, Clone)]
pub struct 宫格项 {
    pub 类型: 宫格类型,
    pub 矩形: Rect,
    pub 标签: Option<&'static str>,
}

#[derive(Debug)]
pub enum Overlay事件结果 {
    无,
    会话开始,
    会话结束,
    方向(方向),
}

#[derive(Debug)]
pub struct Overlay状态 {
    pub 显示: bool,
    pub 锚点: Option<Pos2>,
    pub 窗口原点: Pos2,
    pub 轨迹点: Vec<Pos2>,
    pub 宫格中心: Pos2,
    pub 宫格列表: Vec<宫格项>,
    pub 悬停宫格: Option<usize>,
    pub 上次悬停宫格: Option<usize>,
    pub 提示文字: String,
    pub 需要重定位: bool,
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
            宫格中心: Pos2::ZERO,
            宫格列表: Vec::new(),
            悬停宫格: None,
            上次悬停宫格: None,
            提示文字: String::new(),
            需要重定位: false,
        }
    }

    pub fn 需要重绘(&self) -> bool {
        self.显示 || self.需要重定位
    }

    pub fn 处理原始事件(&mut self, 事件: 原始鼠标事件) -> Overlay事件结果 {
        match 事件 {
            原始鼠标事件::右键按下 { x, y } => {
                let 点 = pos2(x as f32, y as f32);
                self.显示 = true;
                self.锚点 = Some(点);
                self.宫格中心 = 点;
                self.轨迹点.clear();
                if 显示鼠标轨迹 {
                    self.轨迹点.push(点);
                }
                self.宫格列表 = 生成宫格(点);
                self.悬停宫格 = 命中宫格(&self.宫格列表, 点);
                self.上次悬停宫格 = self.悬停宫格;
                self.提示文字 = "手势开始".to_string();
                self.需要重定位 = true;
                Overlay事件结果::会话开始
            }
            原始鼠标事件::鼠标移动 { x, y } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 点 = pos2(x as f32, y as f32);
                if 显示鼠标轨迹 {
                    self.追加轨迹点(点);
                }
                self.更新宫格交互(点)
            }
            原始鼠标事件::右键抬起 { x, y } => {
                if !self.显示 {
                    return Overlay事件结果::无;
                }
                let 点 = pos2(x as f32, y as f32);
                if 显示鼠标轨迹 {
                    self.追加轨迹点(点);
                }
                self.关闭();
                Overlay事件结果::会话结束
            }
        }
    }

    fn 追加轨迹点(&mut self, 点: Pos2) {
        if let Some(最后点) = self.轨迹点.last_mut() {
            if 最后点.distance(点) < 采样距离 {
                *最后点 = 点;
                return;
            }
        }
        self.轨迹点.push(点);
    }

    fn 更新宫格交互(&mut self, 点: Pos2) -> Overlay事件结果 {
        let 当前索引 = 命中宫格(&self.宫格列表, 点);
        self.悬停宫格 = 当前索引;

        let Some(索引) = 当前索引 else {
            self.上次悬停宫格 = None;
            return Overlay事件结果::无;
        };

        if Some(索引) == self.上次悬停宫格 {
            return Overlay事件结果::无;
        }
        self.上次悬停宫格 = Some(索引);

        let 宫格 = &self.宫格列表[索引];
        let 方向 = match 宫格.类型 {
            宫格类型::上 => Some(方向::上),
            宫格类型::下 => Some(方向::下),
            宫格类型::左 => Some(方向::左),
            宫格类型::右 => Some(方向::右),
            宫格类型::中心 | 宫格类型::空白 => None,
        };

        let Some(方向) = 方向 else {
            return Overlay事件结果::无;
        };

        self.提示文字 = match 方向 {
            方向::上 => "上".to_string(),
            方向::下 => "下".to_string(),
            方向::左 => "左".to_string(),
            方向::右 => "右".to_string(),
            _ => String::new(),
        };

        let 新中心 = 宫格.矩形.center();
        self.宫格中心 = 新中心;
        self.宫格列表 = 生成宫格(新中心);
        self.悬停宫格 = 命中宫格(&self.宫格列表, 点);
        self.上次悬停宫格 = self.悬停宫格;

        Overlay事件结果::方向(方向)
    }

    fn 关闭(&mut self) {
        self.显示 = false;
        self.锚点 = None;
        self.悬停宫格 = None;
        self.上次悬停宫格 = None;
        self.提示文字.clear();
        self.需要重定位 = true;
    }
}

fn 命中宫格(宫格列表: &[宫格项], 点: Pos2) -> Option<usize> {
    宫格列表.iter().position(|宫格| 宫格.矩形.contains(点))
}

fn 生成宫格(中心: Pos2) -> Vec<宫格项> {
    let 步长 = 宫格边长 + 宫格间距;
    let 布局: [(i32, i32, 宫格类型, Option<&'static str>); 9] = [
        (-1, -1, 宫格类型::空白, None),
        (0, -1, 宫格类型::上, Some("↑")),
        (1, -1, 宫格类型::空白, None),
        (-1, 0, 宫格类型::左, Some("←")),
        (0, 0, 宫格类型::中心, Some("·")),
        (1, 0, 宫格类型::右, Some("→")),
        (-1, 1, 宫格类型::空白, None),
        (0, 1, 宫格类型::下, Some("↓")),
        (1, 1, 宫格类型::空白, None),
    ];

    布局
        .iter()
        .map(|(列偏移, 行偏移, 类型, 标签)| {
            let 中心偏移 = vec2(*列偏移 as f32 * 步长, *行偏移 as f32 * 步长);
            let 矩形 = Rect::from_center_size(中心 + 中心偏移, vec2(宫格边长, 宫格边长));
            宫格项 {
                类型: *类型,
                矩形,
                标签: *标签,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 进入方向格后重建宫格() {
        let mut 状态 = Overlay状态::默认();
        let 起点 = pos2(200.0, 200.0);
        状态.处理原始事件(原始鼠标事件::右键按下 {
            x: 起点.x as i32,
            y: 起点.y as i32,
        });
        assert_eq!(状态.宫格列表.len(), 9);

        let 右格 = 状态
            .宫格列表
            .iter()
            .find(|宫格| 宫格.类型 == 宫格类型::右)
            .expect("应有右格");
        let 右点 = 右格.矩形.center();

        let 结果 = 状态.处理原始事件(原始鼠标事件::鼠标移动 {
            x: 右点.x as i32,
            y: 右点.y as i32,
        });
        assert!(matches!(结果, Overlay事件结果::方向(方向::右)));
        assert_eq!(状态.宫格中心, 右点);

        let 中心格 = 状态
            .宫格列表
            .iter()
            .find(|宫格| 宫格.类型 == 宫格类型::中心)
            .expect("应有中心格");
        assert_eq!(中心格.矩形.center(), 右点);
    }
}
