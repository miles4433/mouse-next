use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use crate::action::{宫格事件, 匹配触碰动作, 动作是_tab};
use crate::config::启用冻结层;
use crate::input::{原始鼠标事件, 调度补发右键单击};

use super::capture::调度延迟刷新缓存;
use super::repaint::{Overlay重绘信号, 请求重绘};
use super::state::{Overlay事件结果, Overlay共享状态};

pub fn 启动_overlay输入(
    原始事件接收端: Receiver<原始鼠标事件>,
    宫格发送端: Sender<宫格事件>,
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

            let 额外重绘 = 转发宫格事件(&宫格发送端, &状态, &重绘信号, 事件结果);

            if 需要重绘 || 额外重绘 {
                请求重绘(&重绘信号);
            }
        }
    });
}

fn 排除句柄(状态: &Overlay共享状态) -> Option<isize> {
    状态.lock().ok().and_then(|状态| 状态.overlay句柄)
}

fn 安排缓存刷新(状态: &Overlay共享状态, 重绘信号: &Overlay重绘信号) {
    调度延迟刷新缓存(状态.clone(), 重绘信号.clone(), 排除句柄(状态));
}

fn 转发宫格事件(
    宫格发送端: &Sender<宫格事件>,
    状态: &Overlay共享状态,
    重绘信号: &Overlay重绘信号,
    事件结果: Overlay事件结果,
) -> bool {
    match 事件结果 {
        Overlay事件结果::无 => false,
        Overlay事件结果::会话开始 => {
            let _ = 宫格发送端.send(宫格事件::会话开始);
            if 启用冻结层 {
                安排缓存刷新(状态, 重绘信号);
            }
            false
        }
        Overlay事件结果::会话结束 { 补发位置 } => {
            let _ = 宫格发送端.send(宫格事件::会话结束);
            if let Some(位置) = 补发位置 {
                调度补发右键单击(位置.x as i32, 位置.y as i32);
            }
            false
        }
        Overlay事件结果::会话结束流程 { 松手, 补发位置 } => {
            if let Some(事件) = 松手 {
                let _ = 宫格发送端.send(事件);
            }
            let _ = 宫格发送端.send(宫格事件::会话结束);
            if let Some(位置) = 补发位置 {
                调度补发右键单击(位置.x as i32, 位置.y as i32);
            }
            false
        }
        Overlay事件结果::离格截屏 => {
            if !启用冻结层 {
                return false;
            }
            安排缓存刷新(状态, 重绘信号);
            if let Ok(mut 状态) = 状态.lock() {
                if 状态.缓存冻结帧.is_some() {
                    状态.启用显示冻结帧();
                    return true;
                }
            }
            false
        }
        Overlay事件结果::转发(事件) => {
            let mut 额外 = false;
            if let 宫格事件::进入 { ref 队列, .. } = 事件 {
                if 启用冻结层 {
                    if let Some(动作) = 匹配触碰动作(队列) {
                        if 动作是_tab(动作) {
                            if let Ok(mut 状态) = 状态.lock() {
                                if 状态.缓存冻结帧.is_some() {
                                    状态.启用显示冻结帧();
                                    额外 = true;
                                }
                            }
                            安排缓存刷新(状态, 重绘信号);
                        }
                    }
                }
            }
            let _ = 宫格发送端.send(事件);
            额外
        }
    }
}
