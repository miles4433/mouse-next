use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::input::{原始鼠标事件, 方向};

use super::repaint::{Overlay重绘信号, 请求重绘};
use super::state::{Overlay事件结果, Overlay共享状态};

pub fn 启动_overlay输入(
    原始事件接收端: Receiver<原始鼠标事件>,
    方向发送端: Sender<方向>,
    状态: Overlay共享状态,
    重绘信号: Overlay重绘信号,
) {
    thread::spawn(move || {
        while let Ok(事件) = 原始事件接收端.recv() {
            let (事件结果, 需要重绘) = 状态
                .lock()
                .map(|mut 状态| {
                    let 事件结果 = 状态.处理原始事件(事件);
                    (事件结果, 状态.需要重绘())
                })
                .unwrap_or((Overlay事件结果::无, false));

            转发方向(&方向发送端, 事件结果);

            if 需要重绘 {
                请求重绘(&重绘信号);
            }
        }
    });
}

fn 转发方向(方向发送端: &Sender<方向>, 事件结果: Overlay事件结果) {
    match 事件结果 {
        Overlay事件结果::无 => {}
        Overlay事件结果::会话开始 => {
            let _ = 方向发送端.send(方向::开始);
        }
        Overlay事件结果::会话结束 => {
            let _ = 方向发送端.send(方向::结束);
        }
        Overlay事件结果::方向(方向) => {
            let _ = 方向发送端.send(方向);
        }
    }
}
