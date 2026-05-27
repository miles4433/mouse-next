#[derive(Debug, Clone, Copy)]
pub struct 方向阈值 {
    pub 基础: i32,
    pub 水平倍率: f64,
}

impl 方向阈值 {
    pub const 默认: Self = Self {
        基础: 100,
        水平倍率: 2.0,
    };

    pub const fn 垂直(&self) -> i32 {
        self.基础
    }

    pub fn 水平(&self) -> i32 {
        (self.基础 as f64 * self.水平倍率).round() as i32
    }
}

pub const 阈值: 方向阈值 = 方向阈值::默认;

/// 是否在 overlay 中绘制鼠标移动轨迹
pub const 显示鼠标轨迹: bool = false;
