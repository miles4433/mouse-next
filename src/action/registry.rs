use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_SHIFT, VK_T, VK_TAB, VK_W};

use crate::action::types::{动作, 动作类型, 待执行动作, 虚拟键操作};
use crate::input::方向;

/// 动作配置表的一行：名称、触发轨迹、快捷键与触发类型。
#[derive(Debug, Clone, Copy)]
pub struct 动作条目 {
    pub 名称: &'static str,
    pub 轨迹: &'static [方向],
    pub 快捷键: &'static str,
    pub 类型: 动作类型,
}

#[derive(Debug, Clone, Copy)]
enum 动作键 {
    关闭,
    恢复,
    左边,
    右边,
}

/// 全部手势动作的配置表，顺序影响匹配优先级。
const 动作表: &[动作条目] = &[
    动作条目 {
        名称: "关闭",
        轨迹: &[方向::下, 方向::右],
        快捷键: "Ctrl+W",
        类型: 动作类型::独占,
    },
    动作条目 {
        名称: "恢复",
        轨迹: &[方向::上, 方向::右],
        快捷键: "Ctrl+Shift+T",
        类型: 动作类型::独占,
    },
    动作条目 {
        名称: "左边",
        轨迹: &[方向::左],
        快捷键: "Ctrl+Shift+Tab",
        类型: 动作类型::重复,
    },
    动作条目 {
        名称: "右边",
        轨迹: &[方向::右],
        快捷键: "Ctrl+Tab",
        类型: 动作类型::重复,
    },
];

pub fn 动作表条目() -> &'static [动作条目] {
    动作表
}

pub fn 默认动作列表() -> Vec<动作> {
    动作表.iter().map(条目转动作).collect()
}

fn 条目转动作(条目: &动作条目) -> 动作 {
    let 键 = match 条目.名称 {
        "关闭" => 动作键::关闭,
        "恢复" => 动作键::恢复,
        "左边" => 动作键::左边,
        "右边" => 动作键::右边,
        _ => unreachable!(),
    };
    动作 {
        名称: 条目.名称,
        快捷键: 条目.快捷键,
        轨迹: 条目.轨迹.to_vec(),
        按键: 按键序列(键),
        类型: 条目.类型,
    }
}

fn 按键序列(键: 动作键) -> 待执行动作 {
    match 键 {
        动作键::关闭 => vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_W),
            虚拟键操作::抬起(VK_W),
            虚拟键操作::抬起(VK_CONTROL),
        ],
        动作键::恢复 => vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_SHIFT),
            虚拟键操作::按下(VK_T),
            虚拟键操作::抬起(VK_T),
            虚拟键操作::抬起(VK_SHIFT),
            虚拟键操作::抬起(VK_CONTROL),
        ],
        动作键::左边 => vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_SHIFT),
            虚拟键操作::按下(VK_TAB),
            虚拟键操作::抬起(VK_TAB),
            虚拟键操作::抬起(VK_SHIFT),
            虚拟键操作::抬起(VK_CONTROL),
        ],
        动作键::右边 => vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_TAB),
            虚拟键操作::抬起(VK_TAB),
            虚拟键操作::抬起(VK_CONTROL),
        ],
    }
}
