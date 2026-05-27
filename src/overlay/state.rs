use std::sync::{Arc, Mutex};

use eframe::egui::{pos2, vec2, Pos2, Rect};

use crate::input::{原始鼠标事件, 方向};

pub const 隐藏窗口大小: eframe::egui::Vec2 = eframe::egui::vec2(1.0, 1.0);
pub const 隐藏窗口位置: Pos2 = pos2(0.0, 0.0);

const 采样距离: f32 = 2.0;
const 按钮宽度: f32 = 86.0;
const 按钮高度: f32 = 32.0;
const 按钮间距: f32 = 8.0;
const 按钮上偏移: f32 = 70.0;

pub type Overlay共享状态 = Arc<Mutex<Overlay状态>>;

#[derive(Debug, Clone)]
pub struct Overlay按钮 {
    pub 标题: &'static str,
    pub 动作文字: &'static str,
    pub 矩形: Rect,
}

#[derive(Debug)]
pub struct Overlay状态 {
    pub 显示: bool,
    pub 锚点: Option<Pos2>,
    pub 窗口原点: Pos2,
    pub 轨迹点: Vec<Pos2>,
    pub 按钮列表: Vec<Overlay按钮>,
    pub 选中按钮: Option<usize>,
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
            按钮列表: Vec::new(),
            选中按钮: None,
            提示文字: String::new(),
            需要重定位: false,
        }
    }

    pub fn 处理原始事件(&mut self, 事件: 原始鼠标事件) -> Option<&'static str> {
        match 事件 {
            原始鼠标事件::右键按下 { x, y } => {
                let 点 = pos2(x as f32, y as f32);
                self.显示 = true;
                self.锚点 = Some(点);
                self.轨迹点.clear();
                self.轨迹点.push(点);
                self.按钮列表 = 默认按钮(点);
                self.选中按钮 = None;
                self.提示文字 = "手势开始".to_string();
                self.需要重定位 = true;
                None
            }
            原始鼠标事件::鼠标移动 { x, y } => {
                if self.显示 {
                    let 点 = pos2(x as f32, y as f32);
                    self.追加轨迹点(点);
                    self.更新选中按钮(点);
                }
                None
            }
            原始鼠标事件::右键抬起 { x, y } => {
                if !self.显示 {
                    return None;
                }
                let 点 = pos2(x as f32, y as f32);
                self.追加轨迹点(点);
                self.更新选中按钮(点);
                let 动作 = self
                    .选中按钮
                    .and_then(|索引| self.按钮列表.get(索引))
                    .map(|按钮| 按钮.动作文字);
                self.关闭();
                动作
            }
        }
    }

    pub fn 处理方向(&mut self, 方向: 方向) {
        if !self.显示 {
            return;
        }
        self.提示文字 = match 方向 {
            方向::开始 => "手势开始".to_string(),
            方向::结束 => String::new(),
            方向::上 => "上".to_string(),
            方向::下 => "下".to_string(),
            方向::左 => "左".to_string(),
            方向::右 => "右".to_string(),
        };
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

    fn 更新选中按钮(&mut self, 点: Pos2) {
        self.选中按钮 = self.按钮列表.iter().position(|按钮| 按钮.矩形.contains(点));
    }

    fn 关闭(&mut self) {
        self.显示 = false;
        self.锚点 = None;
        self.选中按钮 = None;
        self.提示文字.clear();
        self.需要重定位 = true;
    }
}

fn 默认按钮(锚点: Pos2) -> Vec<Overlay按钮> {
    let 数据 = [
        ("关闭", "假功能：关闭标签"),
        ("恢复", "假功能：恢复标签"),
        ("下个", "假功能：下一个标签"),
        ("上个", "假功能：上一个标签"),
    ];
    let 总宽度 = 按钮宽度 * 数据.len() as f32 + 按钮间距 * (数据.len().saturating_sub(1)) as f32;
    let 起点 = pos2(锚点.x - 总宽度 / 2.0, 锚点.y - 按钮上偏移);

    数据
        .iter()
        .enumerate()
        .map(|(索引, (标题, 动作文字))| {
            let 左上 = 起点 + vec2((按钮宽度 + 按钮间距) * 索引 as f32, 0.0);
            Overlay按钮 {
                标题,
                动作文字,
                矩形: Rect::from_min_size(左上, vec2(按钮宽度, 按钮高度)),
            }
        })
        .collect()
}
