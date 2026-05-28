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

/// tab 切换时是否启用 PrintWindow 冻结层
pub const 启用冻结层: bool = true;

/// 冻结层淡出时长（秒）
pub const 冻结淡出时长: f32 = 0.5;

/// 动作执行后延迟多久重新截屏缓存（毫秒）
pub const 冻结缓存延迟毫秒: u64 = 50;

/// 冻结层峰值不透明度（<1 可透出已切换的新页面）
pub const 冻结层峰值不透明度: f32 = 0.9;

pub fn 冻结淡出速度() -> f32 {
    -0.015_f32.ln() / 冻结淡出时长
}

// ── 单格 ──

/// 格子的视觉边长（像素）
pub const 格边长: f32 = 96.0;

/// 相邻格子中心之间的距离 = 边长 + 间距
pub const 格步长: f32 = 格边长 + 10.0;
