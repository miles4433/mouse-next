use eframe::egui::{pos2, vec2, Pos2, Rect};

use crate::action::types::{待执行动作, 虚拟键操作};
use crate::config::{格边长, 格步长};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_SHIFT, VK_T, VK_TAB, VK_W};

pub type 层 = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum 宫格槽 {
    左上,
    上,
    右上,
    左,
    原点,
    右,
    左下,
    下,
    右下,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 触发方式 {
    触碰,
    松手,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 显示级别 {
    空角弱显,
    常显,
}

#[derive(Debug, Clone, Copy)]
pub struct 宫格注册项 {
    pub 槽: 宫格槽,
    pub 偏移: (i32, i32),
    pub 显示: 显示级别,
    pub 离开截屏: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct 层切换项 {
    pub 从层: 层,
    pub 槽: 宫格槽,
    pub 到层: 层,
    pub 记入序列: bool,
    pub 清空序列: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct 动作表项 {
    pub 名称: &'static str,
    pub 快捷键: &'static str,
    pub 触发序列: &'static [(层, 宫格槽)],
    pub 触发方式: 触发方式,
    pub 可重复: bool,
    pub 独占: bool,
    pub 按键: &'static [虚拟键操作],
    pub 是_tab: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 格符号 {
    空,
    左,
    右,
    上,
    下,
    关闭,
    恢复,
}

#[derive(Debug, Clone, Copy)]
pub struct 格显示 {
    pub 符号: 格符号,
    pub 锁定: bool,
    pub 悬停: bool,
    pub 显示级别: 显示级别,
    pub 标签: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum 宫格事件 {
    会话开始,
    会话结束,
    进入 {
        槽: 宫格槽,
        层: 层,
        队列: Vec<(层, 宫格槽)>,
    },
    同槽重复(宫格槽),
    松手 {
        槽: 宫格槽,
        层: 层,
        队列: Vec<(层, 宫格槽)>,
    },
}

const 宫格注册表: &[宫格注册项] = &[
    宫格注册项 {
        槽: 宫格槽::左上,
        偏移: (-1, -1),
        显示: 显示级别::空角弱显,
        离开截屏: false,
    },
    宫格注册项 {
        槽: 宫格槽::上,
        偏移: (0, -1),
        显示: 显示级别::常显,
        离开截屏: true,
    },
    宫格注册项 {
        槽: 宫格槽::右上,
        偏移: (1, -1),
        显示: 显示级别::常显,
        离开截屏: true,
    },
    宫格注册项 {
        槽: 宫格槽::左,
        偏移: (-1, 0),
        显示: 显示级别::常显,
        离开截屏: true,
    },
    宫格注册项 {
        槽: 宫格槽::原点,
        偏移: (0, 0),
        显示: 显示级别::常显,
        离开截屏: false,
    },
    宫格注册项 {
        槽: 宫格槽::右,
        偏移: (1, 0),
        显示: 显示级别::常显,
        离开截屏: true,
    },
    宫格注册项 {
        槽: 宫格槽::左下,
        偏移: (-1, 1),
        显示: 显示级别::空角弱显,
        离开截屏: false,
    },
    宫格注册项 {
        槽: 宫格槽::下,
        偏移: (0, 1),
        显示: 显示级别::常显,
        离开截屏: true,
    },
    宫格注册项 {
        槽: 宫格槽::右下,
        偏移: (1, 1),
        显示: 显示级别::常显,
        离开截屏: true,
    },
];

const 层切换表: &[层切换项] = &[
    层切换项 {
        从层: 0,
        槽: 宫格槽::上,
        到层: 1,
        记入序列: true,
        清空序列: false,
    },
    层切换项 {
        从层: 0,
        槽: 宫格槽::下,
        到层: -1,
        记入序列: true,
        清空序列: false,
    },
    层切换项 {
        从层: 0,
        槽: 宫格槽::原点,
        到层: 0,
        记入序列: false,
        清空序列: true,
    },
    层切换项 {
        从层: 1,
        槽: 宫格槽::原点,
        到层: 0,
        记入序列: false,
        清空序列: true,
    },
    层切换项 {
        从层: -1,
        槽: 宫格槽::原点,
        到层: 0,
        记入序列: false,
        清空序列: true,
    },
];

const 按键_关闭: &[虚拟键操作] = &[
    虚拟键操作::按下(VK_CONTROL),
    虚拟键操作::按下(VK_W),
    虚拟键操作::抬起(VK_W),
    虚拟键操作::抬起(VK_CONTROL),
];

const 按键_恢复: &[虚拟键操作] = &[
    虚拟键操作::按下(VK_CONTROL),
    虚拟键操作::按下(VK_SHIFT),
    虚拟键操作::按下(VK_T),
    虚拟键操作::抬起(VK_T),
    虚拟键操作::抬起(VK_SHIFT),
    虚拟键操作::抬起(VK_CONTROL),
];

const 按键_左边: &[虚拟键操作] = &[
    虚拟键操作::按下(VK_CONTROL),
    虚拟键操作::按下(VK_SHIFT),
    虚拟键操作::按下(VK_TAB),
    虚拟键操作::抬起(VK_TAB),
    虚拟键操作::抬起(VK_SHIFT),
    虚拟键操作::抬起(VK_CONTROL),
];

const 按键_右边: &[虚拟键操作] = &[
    虚拟键操作::按下(VK_CONTROL),
    虚拟键操作::按下(VK_TAB),
    虚拟键操作::抬起(VK_TAB),
    虚拟键操作::抬起(VK_CONTROL),
];

const 动作表: &[动作表项] = &[
    动作表项 {
        名称: "关闭",
        快捷键: "Ctrl+W",
        触发序列: &[(0, 宫格槽::下), (-1, 宫格槽::右下)],
        触发方式: 触发方式::松手,
        可重复: false,
        独占: true,
        按键: 按键_关闭,
        是_tab: false,
    },
    动作表项 {
        名称: "恢复",
        快捷键: "Ctrl+Shift+T",
        触发序列: &[(0, 宫格槽::上), (1, 宫格槽::右上)],
        触发方式: 触发方式::松手,
        可重复: false,
        独占: true,
        按键: 按键_恢复,
        是_tab: false,
    },
    动作表项 {
        名称: "左边",
        快捷键: "Ctrl+Shift+Tab",
        触发序列: &[(0, 宫格槽::左)],
        触发方式: 触发方式::触碰,
        可重复: true,
        独占: false,
        按键: 按键_左边,
        是_tab: true,
    },
    动作表项 {
        名称: "右边",
        快捷键: "Ctrl+Tab",
        触发序列: &[(0, 宫格槽::右)],
        触发方式: 触发方式::触碰,
        可重复: true,
        独占: false,
        按键: 按键_右边,
        是_tab: true,
    },
];

pub fn 全部槽位() -> &'static [宫格注册项] {
    宫格注册表
}

pub fn 动作表条目() -> &'static [动作表项] {
    动作表
}

pub fn 槽偏移(槽: 宫格槽) -> (i32, i32) {
    宫格注册表
        .iter()
        .find(|项| 项.槽 == 槽)
        .map(|项| 项.偏移)
        .unwrap_or((0, 0))
}

pub fn 槽位中心(锚点: Pos2, 槽: 宫格槽) -> Pos2 {
    let (dx, dy) = 槽偏移(槽);
    锚点 + vec2(dx as f32 * 格步长, dy as f32 * 格步长)
}

pub fn 槽位矩形(锚点: Pos2, 槽: 宫格槽) -> Rect {
    Rect::from_center_size(槽位中心(锚点, 槽), vec2(格边长, 格边长))
}

pub fn 命中槽(锚点: Pos2, 点: Pos2) -> Option<宫格槽> {
    let mut 命中: Option<(宫格槽, f32)> = None;
    for 项 in 宫格注册表 {
        let 矩形 = 槽位矩形(锚点, 项.槽);
        if 矩形.contains(点) {
            let 中心 = 矩形.center();
            let 距离 = (pos2(点.x, 点.y) - 中心).length_sq();
            if 命中.map(|(_, d)| 距离 < d).unwrap_or(true) {
                命中 = Some((项.槽, 距离));
            }
        }
    }
    命中.map(|(槽, _)| 槽)
}

pub fn 应用层切换(当前层: 层, 槽: 宫格槽) -> Option<层切换项> {
    层切换表
        .iter()
        .find(|项| 项.从层 == 当前层 && 项.槽 == 槽)
        .copied()
}

pub fn 进入槽是否允许(当前层: 层, 槽: 宫格槽) -> bool {
    if let Some(_切换) = 应用层切换(当前层, 槽) {
        return true;
    }
    if 当前层 == 0 {
        return matches!(槽, 宫格槽::左 | 宫格槽::右);
    }
    if 当前层 == 1 {
        return 槽 == 宫格槽::右上;
    }
    if 当前层 == -1 {
        return 槽 == 宫格槽::右下;
    }
    false
}

pub fn 处理进入槽(
    当前层: 层,
    队列: &mut Vec<(层, 宫格槽)>,
    槽: 宫格槽,
) -> Option<层> {
    if !进入槽是否允许(当前层, 槽) {
        return None;
    }

    let 新层 = if let Some(切换) = 应用层切换(当前层, 槽) {
        if 切换.清空序列 {
            队列.clear();
        }
        if 切换.记入序列 {
            队列.push((当前层, 槽));
        }
        切换.到层
    } else if 当前层 == 0 && matches!(槽, 宫格槽::左 | 宫格槽::右) {
        队列.push((0, 槽));
        0
    } else if 当前层 == 1 && 槽 == 宫格槽::右上 {
        队列.push((1, 槽));
        1
    } else if 当前层 == -1 && 槽 == 宫格槽::右下 {
        队列.push((-1, 槽));
        -1
    } else {
        return None;
    };

    Some(新层)
}

fn 队列以前缀匹配(队列: &[(层, 宫格槽)], 完整序列: &[(层, 宫格槽)]) -> bool {
    完整序列.len() >= 队列.len() && 完整序列[..队列.len()] == *队列
}

pub fn 等待独占补全(队列: &[(层, 宫格槽)]) -> bool {
    动作表.iter().any(|动作| {
        动作.独占
            && 动作.触发序列.len() > 队列.len()
            && 队列以前缀匹配(队列, 动作.触发序列)
    })
}

pub fn 完全匹配动作(队列: &[(层, 宫格槽)]) -> Option<&'static 动作表项> {
    动作表.iter().find(|动作| {
        队列.len() == 动作.触发序列.len() && 队列 == 动作.触发序列
    })
}

pub fn 匹配触碰动作(队列: &[(层, 宫格槽)]) -> Option<&'static 动作表项> {
    if 等待独占补全(队列) {
        return None;
    }
    完全匹配动作(队列).filter(|动作| 动作.触发方式 == 触发方式::触碰)
}

pub fn 匹配松手动作(队列: &[(层, 宫格槽)]) -> Option<&'static 动作表项> {
    完全匹配动作(队列).filter(|动作| 动作.触发方式 == 触发方式::松手)
}

pub fn 同槽重复动作(当前层: 层, 槽: 宫格槽) -> Option<&'static 动作表项> {
    if 当前层 != 0 {
        return None;
    }
    动作表.iter().find(|动作| {
        动作.可重复
            && 动作.触发方式 == 触发方式::触碰
            && 动作.触发序列 == &[(0, 槽)]
    })
}

pub fn 按键克隆(动作: &动作表项) -> 待执行动作 {
    动作.按键.to_vec()
}

pub fn 槽离开需截屏(槽: 宫格槽) -> bool {
    宫格注册表
        .iter()
        .find(|项| 项.槽 == 槽)
        .is_some_and(|项| 项.离开截屏)
}

pub fn 动作是_tab(动作: &动作表项) -> bool {
    动作.是_tab
}

fn 槽基础符号(槽: 宫格槽) -> 格符号 {
    match 槽 {
        宫格槽::左 => 格符号::左,
        宫格槽::右 => 格符号::右,
        宫格槽::上 => 格符号::上,
        宫格槽::下 => 格符号::下,
        宫格槽::右上 => 格符号::恢复,
        宫格槽::右下 => 格符号::关闭,
        _ => 格符号::空,
    }
}

fn 槽是否锁定(当前层: 层, 队列: &[(层, 宫格槽)], 槽: 宫格槽) -> bool {
    match 槽 {
        宫格槽::左 | 宫格槽::右 => 当前层 != 0,
        宫格槽::右上 => 当前层 != 1 || !队列以前缀匹配(队列, &[(0, 宫格槽::上)]),
        宫格槽::右下 => 当前层 != -1 || !队列以前缀匹配(队列, &[(0, 宫格槽::下)]),
        宫格槽::上 | 宫格槽::下 => false,
        _ => true,
    }
}

pub fn 槽显示状态(
    当前层: 层,
    队列: &[(层, 宫格槽)],
    悬停槽: Option<宫格槽>,
    槽: 宫格槽,
) -> 格显示 {
    let 注册 = 宫格注册表.iter().find(|项| 项.槽 == 槽);
    let 显示级别 = 注册.map(|项| 项.显示).unwrap_or(显示级别::常显);
    let 悬停 = 悬停槽 == Some(槽);
    let 锁定 = 槽是否锁定(当前层, 队列, 槽) && !悬停;
    let 符号 = 槽基础符号(槽);
    let 标签 = if 锁定 {
        None
    } else {
        完全匹配动作(队列)
            .filter(|动作| 动作.触发序列.last().map(|(_, s)| *s) == Some(槽))
            .map(|动作| 动作.名称)
            .or_else(|| {
                if 槽 == 宫格槽::右上 && 当前层 == 1 {
                    Some("恢复")
                } else if 槽 == 宫格槽::右下 && 当前层 == -1 {
                    Some("关闭")
                } else if 槽 == 宫格槽::左 && 当前层 == 0 {
                    Some("左边")
                } else if 槽 == 宫格槽::右 && 当前层 == 0 {
                    Some("右边")
                } else {
                    None
                }
            })
    };

    格显示 {
        符号,
        锁定,
        悬停,
        显示级别,
        标签,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 零层进右匹配触碰() {
        let mut q = Vec::new();
        let 层 = 处理进入槽(0, &mut q, 宫格槽::右).unwrap();
        assert_eq!(层, 0);
        assert_eq!(q, vec![(0, 宫格槽::右)]);
        assert!(匹配触碰动作(&q).is_some());
    }

    #[test]
    fn 上后右上序列可松手匹配() {
        let mut q = Vec::new();
        处理进入槽(0, &mut q, 宫格槽::上);
        处理进入槽(1, &mut q, 宫格槽::右上);
        let 动作 = 匹配松手动作(&q).unwrap();
        assert_eq!(动作.名称, "恢复");
    }

    #[test]
    fn 右后上不误匹配恢复() {
        let mut q = Vec::new();
        处理进入槽(0, &mut q, 宫格槽::右);
        assert!(匹配触碰动作(&q).is_some());
        处理进入槽(0, &mut q, 宫格槽::上);
        assert!(匹配松手动作(&q).is_none());
    }

    #[test]
    fn 一层进左不允许() {
        assert!(!进入槽是否允许(1, 宫格槽::左));
    }

    #[test]
    fn 等待独占时不误触右tab() {
        let mut q = vec![(0, 宫格槽::上)];
        assert!(等待独占补全(&q));
        assert!(匹配触碰动作(&q).is_none());
    }
}
