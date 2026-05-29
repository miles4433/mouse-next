use crate::action::registry::默认动作列表;
use crate::action::types::{动作, 动作类型};
use crate::input::方向;

#[derive(Debug)]
pub struct 会话预览 {
    方向队列: Vec<方向>,
    已锁定: bool,
    动作列表: Vec<动作>,
}

impl 会话预览 {
    pub fn 新建() -> Self {
        Self {
            方向队列: Vec::new(),
            已锁定: false,
            动作列表: 默认动作列表(),
        }
    }

    pub fn 重置(&mut self) {
        self.方向队列.clear();
        self.已锁定 = false;
    }

    pub fn 应用方向(&mut self, 方向: 方向) -> Option<&'static str> {
        if !方向.是步进方向() || self.已锁定 {
            return None;
        }
        self.方向队列.push(方向);
        self.尝试匹配()
    }

    fn 尝试匹配(&mut self) -> Option<&'static str> {
        let mut 最后触发 = None;
        self.方向队列.dedup();

        loop {
            if self.方向队列.is_empty() {
                break;
            }

            let 队首是有效起点 = self
                .动作列表
                .iter()
                .any(|动作| 动作.触发序列.first() == self.方向队列.first());
            if !队首是有效起点 {
                self.方向队列.remove(0);
                continue;
            }

            let 等待独占补全 = self.等待独占补全();

            if let Some(动作) = self.动作列表.iter().find(|动作| {
                if 等待独占补全 && 动作.类型 == 动作类型::重复 {
                    return false;
                }
                队列以前缀匹配(&self.方向队列, &动作.触发序列)
            }) {
                let 长度 = 动作.触发序列.len();
                let 类型 = 动作.类型;
                最后触发 = Some(动作.名称);
                self.方向队列.drain(0..长度);
                if 类型 == 动作类型::独占 {
                    self.已锁定 = true;
                    self.方向队列.clear();
                }
                continue;
            }

            break;
        }

        最后触发
    }

    fn 等待独占补全(&self) -> bool {
        self.动作列表.iter().any(|动作| {
            动作.类型 == 动作类型::独占
                && 动作.触发序列.len() > self.方向队列.len()
                && 动作.触发序列.starts_with(&self.方向队列)
        })
    }
}

fn 队列以前缀匹配(队列: &[方向], 前缀: &[方向]) -> bool {
    队列.len() >= 前缀.len() && 队列[..前缀.len()] == *前缀
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 右步触发_ctrl_tab() {
        let mut 预览 = 会话预览::新建();
        assert_eq!(预览.应用方向(方向::右), Some("右边"));
    }

    #[test]
    fn 下后右触发_ctrl_w() {
        let mut 预览 = 会话预览::新建();
        assert_eq!(预览.应用方向(方向::下), None);
        assert_eq!(预览.应用方向(方向::右), Some("关闭"));
    }
}
