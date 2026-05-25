mod action;
mod config;
mod input;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use action::{启动匹配器, 启动执行器};
use input::{启动_hook, 启动处理器};

fn main() {
    let (原始事件发送端, 原始事件接收端) = mpsc::channel();
    let (方向发送端, 方向接收端) = mpsc::channel();
    let (执行发送端, 执行接收端) = mpsc::channel();
    let 停止 = Arc::new(AtomicBool::new(false));

    启动处理器(原始事件接收端, 方向发送端);
    启动匹配器(方向接收端, 执行发送端);
    启动执行器(执行接收端);
    启动_hook(原始事件发送端, 停止.clone());

    let 停止_ctrlc = 停止.clone();
    ctrlc::set_handler(move || {
        停止_ctrlc.store(true, Ordering::Relaxed);
        std::process::exit(0);
    })
    .expect("注册 Ctrl+C 失败");

    println!("mouse-next 已启动，按住右键拖动测试，Ctrl+C 退出");

    while !停止.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }
}
