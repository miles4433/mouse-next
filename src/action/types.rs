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
    pub 名称: &'static str,
    pub 快捷键: &'static str,
    pub 触发序列: Vec<方向>,
    pub 按键: 待执行动作,
    pub 类型: 动作类型,
}
