use std::sync::mpsc::Sender;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEINPUT,
};

use crate::config::方向阈值;
use crate::input::types::{原始鼠标事件, 方向};

enum 状态 {
    空闲,
    记录中 {
        累积_x: i32,
        累积_y: i32,
        上次_x: i32,
        上次_y: i32,
        已触发手势: bool,
    },
}

pub struct 监测器 {
    状态: 状态,
    方向发送端: Sender<方向>,
}

impl 监测器 {
    pub fn 新建(方向发送端: Sender<方向>) -> Self {
        Self {
            状态: 状态::空闲,
            方向发送端,
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
                };
            }
            原始鼠标事件::鼠标移动 { x, y } => {
                let 新方向 = if let 状态::记录中 {
                    累积_x,
                    累积_y,
                    上次_x,
                    上次_y,
                    已触发手势,
                } = &mut self.状态
                {
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
                    let _ = self.方向发送端.send(方向);
                }
            }
            原始鼠标事件::右键抬起 { .. } => {
                let 需要补发 = matches!(
                    self.状态,
                    状态::记录中 {
                        已触发手势: false,
                        ..
                    }
                );
                if 需要补发 {
                    补发右键单击();
                }
                self.状态 = 状态::空闲;
            }
        }
    }

    fn 更新累积(累积: &mut i32, 增量: i32) {
        *累积 += 增量;
    }

    fn 收集方向(累积_x: &mut i32, 累积_y: &mut i32, 已触发手势: &mut bool) -> Vec<方向> {
        let mut 结果 = Vec::new();
        while *累积_x >= 方向阈值 {
            *累积_x = 0;
            *累积_y = 0;
            *已触发手势 = true;
            结果.push(方向::右);
        }
        while *累积_x <= -方向阈值 {
            *累积_x = 0;
            *累积_y = 0;
            *已触发手势 = true;
            结果.push(方向::左);
        }
        while *累积_y >= 方向阈值 {
            *累积_x = 0;
            *累积_y = 0;
            *已触发手势 = true;
            结果.push(方向::下);
        }
        while *累积_y <= -方向阈值 {
            *累积_x = 0;
            *累积_y = 0;
            *已触发手势 = true;
            结果.push(方向::上);
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

    #[test]
    fn 累积达阈值发送方向() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 100, y: 0 });

        assert_eq!(接收端.try_recv().unwrap(), 方向::右);
        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 未触发手势时抬起需补发标记() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::右键抬起 { x: 0, y: 0 });

        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 方向反转时抵消累积() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 80, y: 0 });
        监测器.处理(原始鼠标事件::鼠标移动 { x: 0, y: 0 });

        assert!(接收端.try_recv().is_err());
    }

    #[test]
    fn 触发阈值后重置_xy_累积() {
        let (发送端, 接收端) = mpsc::channel();
        let mut 监测器 = 监测器::新建(发送端);

        监测器.处理(原始鼠标事件::右键按下 { x: 0, y: 0 });
        // 斜向移动同时积累 x、y，触发一次后两者应清零
        监测器.处理(原始鼠标事件::鼠标移动 { x: 100, y: 100 });

        assert_eq!(接收端.try_recv().unwrap(), 方向::右);
        assert!(接收端.try_recv().is_err());

        // 未达阈值的后续移动不应立刻再触发
        监测器.处理(原始鼠标事件::鼠标移动 { x: 150, y: 150 });
        assert!(接收端.try_recv().is_err());
    }
}
