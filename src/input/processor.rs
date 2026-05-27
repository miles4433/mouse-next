use std::sync::mpsc::Receiver;
use std::thread;

use super::monitor::监测器;
use super::types::原始鼠标事件;

pub fn 启动处理器(原始事件接收端: Receiver<原始鼠标事件>) {
    thread::spawn(move || {
        let mut 监测器 = 监测器::新建多个(Vec::new());
        while let Ok(事件) = 原始事件接收端.recv() {
            监测器.处理(事件);
        }
    });
}
