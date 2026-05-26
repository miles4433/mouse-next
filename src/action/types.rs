use crate::input::方向;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 虚拟键操作 {
    按下(VIRTUAL_KEY),
    抬起(VIRTUAL_KEY),
    打印(&'static str),
}

pub type 待执行动作 = Vec<虚拟键操作>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 动作类型 {
    重复,
    独占,
}

#[derive(Debug, Clone)]
pub struct 动作 {
    pub 轨迹: Vec<方向>,
    pub 按键: 待执行动作,
    pub 类型: 动作类型,
}

pub fn 是轨迹方向(方向: 方向) -> bool {
    matches!(方向, 方向::上 | 方向::下 | 方向::左 | 方向::右)
}
