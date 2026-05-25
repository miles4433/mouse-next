use std::sync::mpsc::Receiver;
use std::thread;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
};

use crate::action::types::{待执行动作, 虚拟键操作};

pub fn 启动执行器(执行接收端: Receiver<待执行动作>) {
    thread::spawn(move || {
        while let Ok(动作) = 执行接收端.recv() {
            执行动作(动作);
        }
    });
}

fn 执行动作(按键序列: 待执行动作) {
    if 按键序列.is_empty() {
        return;
    }

    let 输入: Vec<INPUT> = 按键序列
        .into_iter()
        .map(|操作| {
            let (虚拟键, 抬起) = match 操作 {
                虚拟键操作::按下(键) => (键, false),
                虚拟键操作::抬起(键) => (键, true),
            };
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 虚拟键,
                        dwFlags: if 抬起 {
                            KEYEVENTF_KEYUP
                        } else {
                            KEYBD_EVENT_FLAGS(0)
                        },
                        ..Default::default()
                    },
                },
            }
        })
        .collect();

    unsafe {
        SendInput(&输入, std::mem::size_of::<INPUT>() as i32);
    }
}
