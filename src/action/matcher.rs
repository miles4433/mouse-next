use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::action::registry::默认动作列表;
use crate::action::types::{动作, 动作类型, 待执行动作, 是轨迹方向};
use crate::input::方向;

pub struct 匹配器 {
    轨迹队列: Vec<方向>,
    动作列表: Vec<动作>,
    执行发送端: Sender<待执行动作>,
    会话中: bool,
    已锁定: bool,
}

impl 匹配器 {
    pub fn 新建(执行发送端: Sender<待执行动作>) -> Self {
        Self {
            轨迹队列: Vec::new(),
            动作列表: 默认动作列表(),
            执行发送端,
            会话中: false,
            已锁定: false,
        }
    }

    pub fn 处理方向(&mut self, 事件: 方向) {
        match 事件 {
            方向::开始 => {
                self.会话中 = true;
                self.已锁定 = false;
                self.轨迹队列.clear();
            }
            方向::结束 => {
                self.会话中 = false;
                self.已锁定 = false;
                self.轨迹队列.clear();
            }
            d if 是轨迹方向(d) => {
                if !self.会话中 || self.已锁定 {
                    return;
                }
                self.轨迹队列.push(d);
                self.尝试匹配();
            }
            _ => {}
        }
    }

    fn 尝试匹配(&mut self) {
        self.轨迹队列.dedup();

        loop {
            if self.轨迹队列.is_empty() {
                break;
            }

            let 队首是有效起点 = self
                .动作列表
                .iter()
                .any(|动作| 动作.轨迹.first() == self.轨迹队列.first());
            if !队首是有效起点 {
                self.轨迹队列.remove(0);
                continue;
            }

            let 等待独占补全 = self.等待独占补全();

            if let Some(动作) = self.动作列表.iter().find(|动作| {
                if 等待独占补全 && 动作.类型 == 动作类型::重复 {
                    return false;
                }
                队列以前缀匹配(&self.轨迹队列, &动作.轨迹)
            }) {
                let 长度 = 动作.轨迹.len();
                let 类型 = 动作.类型;
                self.轨迹队列.drain(0..长度);
                let _ = self.执行发送端.send(动作.按键.clone());
                if 类型 == 动作类型::独占 {
                    self.已锁定 = true;
                    self.轨迹队列.clear();
                }
                continue;
            }

            break;
        }
    }

    fn 等待独占补全(&self) -> bool {
        self.动作列表.iter().any(|动作| {
            动作.类型 == 动作类型::独占
                && 动作.轨迹.len() > self.轨迹队列.len()
                && 动作.轨迹.starts_with(&self.轨迹队列)
        })
    }
}

fn 队列以前缀匹配(队列: &[方向], 前缀: &[方向]) -> bool {
    队列.len() >= 前缀.len() && 队列[..前缀.len()] == *前缀
}

pub fn 启动匹配器(
    方向接收端: Receiver<方向>, 执行发送端: Sender<待执行动作>
) {
    thread::spawn(move || {
        let mut 匹配器 = 匹配器::新建(执行发送端);
        while let Ok(事件) = 方向接收端.recv() {
            匹配器.处理方向(事件);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_CONTROL, VK_SHIFT, VK_T, VK_TAB, VK_W};

    use crate::action::types::虚拟键操作;

    fn 开始会话(匹配器: &mut 匹配器) {
        匹配器.处理方向(方向::开始);
    }

    fn 右滑动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_TAB),
            虚拟键操作::抬起(VK_TAB),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    fn 左滑动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_SHIFT),
            虚拟键操作::按下(VK_TAB),
            虚拟键操作::抬起(VK_TAB),
            虚拟键操作::抬起(VK_SHIFT),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    fn 下右动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_W),
            虚拟键操作::抬起(VK_W),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    fn 上右动作() -> 待执行动作 {
        vec![
            虚拟键操作::按下(VK_CONTROL),
            虚拟键操作::按下(VK_SHIFT),
            虚拟键操作::按下(VK_T),
            虚拟键操作::抬起(VK_T),
            虚拟键操作::抬起(VK_SHIFT),
            虚拟键操作::抬起(VK_CONTROL),
        ]
    }

    #[test]
    fn 未开始时不触发() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        匹配器.处理方向(方向::右);

        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 右滑触发_ctrl_tab() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::右);

        assert_eq!(执行接收端.try_recv().unwrap(), 右滑动作());
        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 单独上方向不触发() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::上);

        assert!(执行接收端.try_recv().is_err());
        assert_eq!(匹配器.轨迹队列, vec![方向::上]);

        匹配器.处理方向(方向::结束);
        assert!(匹配器.轨迹队列.is_empty());
    }

    #[test]
    fn 连续右左各触发一次() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::右);
        匹配器.处理方向(方向::左);

        assert_eq!(执行接收端.try_recv().unwrap(), 右滑动作());
        assert_eq!(执行接收端.try_recv().unwrap(), 左滑动作());
        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 开始结束清空队列且仅执行有效方向() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        匹配器.处理方向(方向::开始);
        匹配器.处理方向(方向::右);
        匹配器.处理方向(方向::结束);

        assert_eq!(执行接收端.try_recv().unwrap(), 右滑动作());
        assert!(执行接收端.try_recv().is_err());
        assert!(匹配器.轨迹队列.is_empty());
    }

    #[test]
    fn 连续右滑触发两次() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::右);
        匹配器.处理方向(方向::右);

        assert_eq!(执行接收端.try_recv().unwrap(), 右滑动作());
        assert_eq!(执行接收端.try_recv().unwrap(), 右滑动作());
        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 下右触发_ctrl_w_并锁定() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::下);
        匹配器.处理方向(方向::右);

        assert_eq!(执行接收端.try_recv().unwrap(), 下右动作());
        assert!(执行接收端.try_recv().is_err());

        匹配器.处理方向(方向::右);
        匹配器.处理方向(方向::左);
        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 上右触发_ctrl_shift_t_并锁定() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::上);
        匹配器.处理方向(方向::右);

        assert_eq!(执行接收端.try_recv().unwrap(), 上右动作());
        assert!(执行接收端.try_recv().is_err());

        匹配器.处理方向(方向::右);
        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 重复方向折叠后匹配独占动作() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::下);
        匹配器.处理方向(方向::下);
        匹配器.处理方向(方向::右);

        assert_eq!(执行接收端.try_recv().unwrap(), 下右动作());
        assert!(执行接收端.try_recv().is_err());
    }

    #[test]
    fn 等待独占补全时不误触重复动作() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::上);

        assert!(执行接收端.try_recv().is_err());

        匹配器.处理方向(方向::右);
        assert_eq!(执行接收端.try_recv().unwrap(), 上右动作());
    }

    #[test]
    fn 独占触发后忽略后续方向直到结束() {
        let (执行发送端, 执行接收端) = std::sync::mpsc::channel();
        let mut 匹配器 = 匹配器::新建(执行发送端);

        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::下);
        匹配器.处理方向(方向::右);
        匹配器.处理方向(方向::左);

        assert_eq!(执行接收端.try_recv().unwrap(), 下右动作());
        assert!(执行接收端.try_recv().is_err());

        匹配器.处理方向(方向::结束);
        开始会话(&mut 匹配器);
        匹配器.处理方向(方向::左);

        assert_eq!(执行接收端.try_recv().unwrap(), 左滑动作());
    }
}
