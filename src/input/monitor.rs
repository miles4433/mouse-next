use std::sync::mpsc::Sender;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT,
};

use crate::config::阈值;
use crate::input::types::{原始鼠标事件, 方向};

enum 状态 {
    空闲,
    记录中 {
        累积_x: i32,
        累积_y: i32,
        上次_x: i32,
        上次_y: i32,
        已触发手势: bool,
        有移动: bool,
    },
}

pub struct 监测器 {
    状态: 状态,
    方向发送端列表: Vec<Sender<方向>>,
}

impl 监测器 {
    #[cfg(test)]
    pub fn 新建(方向发送端: Sender<方向>) -> Self {
        Self::新建多个(vec![方向发送端])
    }

    pub fn 新建多个(方向发送端列表: Vec<Sender<方向>>) -> Self {
        Self {
            状态: 状态::空闲,
            方向发送端列表,
        }
    }

    pub fn 处理(&mut self, 事件: 原始鼠标事件) {
        match 事件 {
            原始鼠标事件::右键按下 { x, y } => {
                self.状态 = 状态::记录中 {
                    累积_x: 0,
                    累积_y: 0,
                    上次_x: x,
                    上次_y: y,
                    已触发手势: false,
                    有移动: false,
                };
                self.发送方向(方向::开始);
            }
            原始鼠标事件::鼠标移动 { x, y } => {
                let 新方向 = if let 状态::记录中 {
                    累积_x,
                    累积_y,
                    上次_x,
                    上次_y,
                    已触发手势,
                    有移动,
                } = &mut self.状态
                {
                    *有移动 = true;
                    let 增量_x = x - *上次_x;
                    let 增量_y = y - *上次_y;
                    *上次_x = x;
                    *上次_y = y;
                    Self::更新累积(累积_x, 增量_x);
                    Self::更新累积(累积_y, 增量_y);
                    Self::收集方向(累积_x, 累积_y, 已触发手势)
                } else {
                    Vec::new()
                };
                for 方向 in 新方向 {
                    self.发送方向(方向);
                }
            }
            原始鼠标事件::右键抬起 { .. } => {
                let 需要补发 = matches!(
                    self.状态,
                    状态::记录中 {
                        有移动: false, ..
                    }
                );
                if 需要补发 {
                    补发右键单击();
                }
                self.状态 = 状态::空闲;
                self.发送方向(方向::结束);
            }
        }
    }

    fn 发送方向(&self, 方向: 方向) {
        for 发送端 in &self.方向发送端列表 {
            let _ = 发送端.send(方向);
        }
    }

    fn 更新累积(累积: &mut i32, 增量: i32) {
        if 增量 == 0 {
            return;
        }
        if *累积 != 0 && 增量.signum() != 累积.signum() {
            *累积 = 增量;
        } else {
            *累积 += 增量;
        }
    }

    fn 收集方向(
        累积_x: &mut i32, 累积_y: &mut i32, 已触发手势: &mut bool
    ) -> Vec<方向> {
        let mut 结果 = Vec::new();
        // 垂直优先，且各轴触发时只清零本轴，保留另一轴积累以支持 下右/上右 等多步轨迹
        while *累积_y >= 阈值.垂直() {
            *累积_y = 0;
            *已触发手势 = true;
            结果.push(方向::下);
        }
        while *累积_y <= -阈值.垂直() {
            *累积_y = 0;
            *已触发手势 = true;
            结果.push(方向::上);
        }
        while *累积_x >= 阈值.水平() {
            *累积_x = 0;
            *已触发手势 = true;
            结果.push(方向::右);
        }
        while *累积_x <= -阈值.水平() {
            *累积_x = 0;
            *已触发手势 = true;
            结果.push(方向::左);
        }
        结果
    }
}

fn 补发右键单击() {
    let 输入 = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: MOUSEEVENTF_RIGHTDOWN,
                    ..Default::default()
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: MOUSEEVENTF_RIGHTUP,
                    ..Default::default()
                },
            },
        },
    ];
    unsafe {
        SendInput(&输入, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    use crate::config::阈值;

    #[test]
    fn 累积达阈值发送方向() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 阈值.水平(),
            y: 0,
        });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::右);
        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 无位移时抬起需补发右键() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::右键抬起 { x: 0, y: 0 });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::结束);
        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 有位移但未达阈值时不补发右键() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 80, y: 0 });
        监测器.处理(原始鼠标事件::右键抬起 { x: 80, y: 0 });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::结束);
        assert!(接收端.try_recv().is_err());
        assert!(matches!(监测器.状态, 状态::空闲));
    }

    #[test]
    fn 方向反转时重置累积() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 80, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 0, y: 0 });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert!(接收端.try_recv().is_err());

        // 反向滑过阈值应触发，而非被抵消
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: -阈值.水平(),
            y: 0,
        });
        assert_eq!(接收端.try_recv().unwrap(), 方向::左);
    }

    #[test]
    fn 换轴时清零另一轴累积() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 80, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 80,
            y: 阈值.垂直(),
        });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::下);
        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 斜向下右同帧触发两方向() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 阈值.水平(),
            y: 阈值.垂直(),
        });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::下);
        assert_eq!(接收端.try_recv().unwrap(), 方向::右);
        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 先下后右分步触发() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 0,
            y: 阈值.垂直(),
        });
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 阈值.水平(),
            y: 阈值.垂直(),
        });

        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::下);
        assert_eq!(接收端.try_recv().unwrap(), 方向::右);
        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 触发单轴后不重置另一轴() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 阈值.水平(),
            y: 阈值.垂直(),
        });
        assert_eq!(接收端.try_recv().unwrap(), 方向::开始);
        assert_eq!(接收端.try_recv().unwrap(), 方向::下);
        assert_eq!(接收端.try_recv().unwrap(), 方向::右);
        assert!(接收端.try_recv().is_err());

        // 后续小幅移动不应立刻再触发
        监测器.处理(原始鼠标事件::鼠标移动 {
            x: 阈值.水平() + 阈值.垂直() / 2,
            y: 阈值.垂直() + 阈值.垂直() / 2,
        });
        assert!(接收端.try_recv().is_err());
    }
}
