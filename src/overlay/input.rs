use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::action::{待执行动作, 虚拟键操作};
use crate::input::{原始鼠标事件, 方向};

use super::repaint::{Overlay重绘信号, 请求重绘};
use super::state::Overlay共享状态;

pub fn 启动_overlay输入(
    原始事件接收端: Receiver<原始鼠标事件>,
    方向接收端: Receiver<方向>,
    执行发送端: Sender<待执行动作>,
    状态: Overlay共享状态,
    重绘信号: Overlay重绘信号,
) {
    let 原始状态 = 状态.clone();
    let 原始重绘 = 重绘信号.clone();
    thread::spawn(move || {
        while let Ok(事件) = 原始事件接收端.recv() {
            let (待打印, 需要重绘) = 原始状态
                .lock()
                .map(|mut 状态| {
                    let 待打印 = 状态.处理原始事件(事件);
                    (待打印, 状态.需要重绘())
                })
                .unwrap_or((None, false));
            if 需要重绘 {
                请求重绘(&原始重绘);
            }
            if let Some(文字) = 待打印 {
                let _ = 执行发送端.send(vec![虚拟键操作::打印(文字)]);
            }
        }
    });

    thread::spawn(move || {
        while let Ok(方向) = 方向接收端.recv() {
            if !方向.是轨迹方向() {
                continue;
            }
            let 需要重绘 = 状态
                .lock()
                .map(|mut 状态| 状态.处理方向(方向))
                .unwrap_or(false);
            if 需要重绘 {
                请求重绘(&重绘信号);
            }
        }
    });
}
