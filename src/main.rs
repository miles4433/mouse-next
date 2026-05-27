mod action;
mod config;
mod input;
mod overlay;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use action::{启动匹配器, 启动执行器};
use input::{启动_hook, 启动处理器};
use overlay::{Overlay状态, 启动_overlay输入, 新建重绘信号, 运行_overlay窗口};

fn main() {
    let (原始事件发送端, 原始事件接收端) = mpsc::channel();
    let (overlay原始事件发送端, overlay原始事件接收端) = mpsc::channel();
    let (方向发送端, 方向接收端) = mpsc::channel();
    let (overlay方向发送端, overlay方向接收端) = mpsc::channel();
    let (执行发送端, 执行接收端) = mpsc::channel();
    let 停止 = Arc::new(AtomicBool::new(false));
    let overlay状态 = Overlay状态::新建共享();
    let overlay重绘 = 新建重绘信号();

    启动处理器(原始事件接收端, 方向发送端, overlay方向发送端);
    启动匹配器(方向接收端, 执行发送端.clone());
    启动_overlay输入(
        overlay原始事件接收端,
        overlay方向接收端,
        执行发送端,
        overlay状态.clone(),
        overlay重绘.clone(),
    );
    启动执行器(执行接收端);
    启动_hook(原始事件发送端, overlay原始事件发送端, 停止.clone());

    let 停止_ctrlc = 停止.clone();
    ctrlc::set_handler(move || {
        停止_ctrlc.store(true, Ordering::Relaxed);
        std::process::exit(0);
    })
    .expect("注册 Ctrl+C 失败");

    println!("mouse-next 已启动，按住右键拖动测试，Ctrl+C 退出");

    运行_overlay窗口(overlay状态, overlay重绘);
    停止.store(true, Ordering::Relaxed);
}
