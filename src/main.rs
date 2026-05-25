mod config;
mod input;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

use input::{启动_hook, 启动处理器, 方向};

fn main() {
    let (原始事件发送端, 原始事件接收端) = mpsc::channel();
    let (方向发送端, 方向接收端) = mpsc::channel();
    let 停止 = Arc::new(AtomicBool::new(false));

    启动处理器(原始事件接收端, 方向发送端);
    启动_hook(原始事件发送端, 停止.clone());

    let 停止_ctrlc = 停止.clone();
    ctrlc::set_handler(move || {
        停止_ctrlc.store(true, Ordering::Relaxed);
        std::process::exit(0);
    })
    .expect("注册 Ctrl+C 失败");

    println!("输入监测已启动，按住右键拖动测试，Ctrl+C 退出");

    loop {
        match 方向接收端.recv_timeout(Duration::from_millis(100)) {
            Ok(方向) => 打印方向(方向),
            Err(RecvTimeoutError::Timeout) => {
                if 停止.load(Ordering::Relaxed) {
                    break;
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn 打印方向(方向: 方向) {
    match 方向 {
        方向::开始 => println!("\x1b[35m● 开始\x1b[0m"),
        方向::结束 => println!("\x1b[90m● 结束\x1b[0m"),
        方向::上 => println!("\x1b[36m↑ 上\x1b[0m"),
        方向::下 => println!("\x1b[33m↓ 下\x1b[0m"),
        方向::左 => println!("\x1b[32m← 左\x1b[0m"),
        方向::右 => println!("\x1b[31m→ 右\x1b[0m"),
    }
}
