use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_SHIFT, VK_TAB, VK_T, VK_W};

use crate::action::types::{动作, 动作类型, 虚拟键操作};
use crate::input::方向;

fn 组合键(按键: &[虚拟键操作]) -> Vec<虚拟键操作> {
    按键.to_vec()
}

pub fn 默认动作列表() -> Vec<动作> {
    vec![
        动作 {
            轨迹: vec![方向::下, 方向::右],
            按键: 组合键(&[
                虚拟键操作::按下(VK_CONTROL),
                虚拟键操作::按下(VK_W),
                虚拟键操作::抬起(VK_W),
                虚拟键操作::抬起(VK_CONTROL),
            ]),
            类型: 动作类型::独占,
        },
        动作 {
            轨迹: vec![方向::上, 方向::右],
            按键: 组合键(&[
                虚拟键操作::按下(VK_CONTROL),
                虚拟键操作::按下(VK_SHIFT),
                虚拟键操作::按下(VK_T),
                虚拟键操作::抬起(VK_T),
                虚拟键操作::抬起(VK_SHIFT),
                虚拟键操作::抬起(VK_CONTROL),
            ]),
            类型: 动作类型::独占,
        },
        动作 {
            轨迹: vec![方向::左],
            按键: 组合键(&[
                虚拟键操作::按下(VK_CONTROL),
                虚拟键操作::按下(VK_SHIFT),
                虚拟键操作::按下(VK_TAB),
                虚拟键操作::抬起(VK_TAB),
                虚拟键操作::抬起(VK_SHIFT),
                虚拟键操作::抬起(VK_CONTROL),
            ]),
            类型: 动作类型::重复,
        },
        动作 {
            轨迹: vec![方向::右],
            按键: 组合键(&[
                虚拟键操作::按下(VK_CONTROL),
                虚拟键操作::按下(VK_TAB),
                虚拟键操作::抬起(VK_TAB),
                虚拟键操作::抬起(VK_CONTROL),
            ]),
            类型: 动作类型::重复,
        },
    ]
}
