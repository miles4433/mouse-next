use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 虚拟键操作 {
    按下(VIRTUAL_KEY),
    抬起(VIRTUAL_KEY),
    打印(&'static str),
}

pub type 待执行动作 = Vec<虚拟键操作>;
