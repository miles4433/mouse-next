use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use super::panel_table::{
    self, 匹配松手动作, 匹配触碰动作, 同槽重复动作, 宫格事件, 按键克隆,
};
use crate::action::types::待执行动作;

pub struct 匹配器 {
    执行发送端: Sender<待执行动作>,
    会话中: bool,
    已锁定: bool,
    已执行独占: bool,
}

impl 匹配器 {
    pub fn 新建(执行发送端: Sender<待执行动作>) -> Self {
        Self {
            执行发送端,
            会话中: false,
            已锁定: false,
            已执行独占: false,
        }
    }

    pub fn 处理事件(&mut self, 事件: 宫格事件) {
        match 事件 {
            宫格事件::会话开始 => {
                self.会话中 = true;
                self.已锁定 = false;
                self.已执行独占 = false;
            }
            宫格事件::会话结束 => {
                self.会话中 = false;
                self.已锁定 = false;
                self.已执行独占 = false;
            }
            宫格事件::进入 { 队列, .. } => {
                if !self.会话中 || self.已锁定 {
                    return;
                }
                if let Some(动作) = 匹配触碰动作(&队列) {
                    self.执行(动作);
                }
            }
            宫格事件::松手 { 队列, .. } => {
                if !self.会话中 || self.已锁定 {
                    return;
                }
                if let Some(动作) = 匹配松手动作(&队列) {
                    self.执行(动作);
                }
            }
            宫格事件::同槽重复(槽) => {
                if !self.会话中 || self.已锁定 {
                    return;
                }
                if let Some(动作) = 同槽重复动作(0, 槽) {
                    self.执行(动作);
                }
            }
        }
    }

    fn 执行(&mut self, 动作: &panel_table::动作表项) {
        let _ = self.执行发送端.send(按键克隆(动作));
        if 动作.独占 {
            self.已锁定 = true;
            self.已执行独占 = true;
        }
    }
}

pub fn 启动匹配器(宫格接收端: Receiver<宫格事件>, 执行发送端: Sender<待执行动作>) {
    thread::spawn(move || {
        let mut 匹配器 = 匹配器::新建(执行发送端);
        while let Ok(事件) = 宫格接收端.recv() {
            匹配器.处理事件(事件);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use panel_table::{处理进入槽, 宫格槽};
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_SHIFT, VK_T, VK_TAB, VK_W};

    use crate::action::types::虚拟键操作;

    fn 右滑动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_TAB),
            虚拟键操作::抬起(VK_TAB),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    fn 恢复动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_SHIFT),
            虚拟键操作::按下(VK_T),
            虚拟键操作::抬起(VK_T),
            虚拟键操作::抬起(VK_SHIFT),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    fn 关闭动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_W),
            虚拟键操作::抬起(VK_W),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    #[test]
    fn 进入右格触碰执行() {
        let (发送, 接收) = std::sync::mpsc::channel();
        let mut m = 匹配器::新建(发送);
        m.处理事件(宫格事件::会话开始);
        let mut q = Vec::new();
        处理进入槽(0, &mut q, 宫格槽::右);
        m.处理事件(宫格事件::进入 {
            槽: 宫格槽::右,
            层: 0,
            队列: q,
        });
        assert_eq!(接收.try_recv().unwrap(), 右滑动作());
    }

    #[test]
    fn 松手恢复执行() {
        let (发送, 接收) = std::sync::mpsc::channel();
        let mut m = 匹配器::新建(发送);
        m.处理事件(宫格事件::会话开始);
        let mut q = Vec::new();
        处理进入槽(0, &mut q, 宫格槽::上);
        处理进入槽(1, &mut q, 宫格槽::右上);
        m.处理事件(宫格事件::松手 {
            槽: 宫格槽::右上,
            层: 1,
            队列: q,
        });
        assert_eq!(接收.try_recv().unwrap(), 恢复动作());
        m.处理事件(宫格事件::同槽重复(宫格槽::右));
        assert!(接收.try_recv().is_err());
    }

    #[test]
    fn 同槽重复左键() {
        let (发送, 接收) = std::sync::mpsc::channel();
        let mut m = 匹配器::新建(发送);
        m.处理事件(宫格事件::会话开始);
        m.处理事件(宫格事件::同槽重复(宫格槽::右));
        assert_eq!(接收.try_recv().unwrap(), 右滑动作());
    }
}
