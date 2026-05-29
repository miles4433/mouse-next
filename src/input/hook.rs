use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, PeekMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK,
    LLMHF_INJECTED, MSG, PM_REMOVE, WH_MOUSE_LL, WM_MOUSEMOVE, WM_QUIT, WM_RBUTTONDOWN,
    WM_RBUTTONUP,
};

use super::types::原始鼠标事件;

thread_local! {
    static 原始事件发送端: RefCell<Option<Sender<原始鼠标事件>>> = const { RefCell::new(None) };
}

pub fn 启动_hook(overlay发送端: Sender<原始鼠标事件>, 停止: Arc<AtomicBool>) {
    thread::spawn(move || {
        原始事件发送端.with(|槽| {
            *槽.borrow_mut() = Some(overlay发送端);
        });

        let hook = unsafe { 安装_hook() };
        let hook = match hook {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Hook 安装失败: {e}");
                return;
            }
        };

        let mut 消息 = MSG::default();
        while !停止.load(Ordering::Relaxed) {
            unsafe {
                while PeekMessageW(&mut 消息, None, 0, 0, PM_REMOVE).into() {
                    if 消息.message == WM_QUIT {
                        停止.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        unsafe {
            let _ = UnhookWindowsHookEx(hook);
        }
    });
}

unsafe fn 安装_hook() -> windows::core::Result<HHOOK> {
    SetWindowsHookExW(WH_MOUSE_LL, Some(鼠标_hook回调), HINSTANCE::default(), 0)
}

unsafe extern "system" fn 鼠标_hook回调(
    代码: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if 代码 != HC_ACTION as i32 {
        return CallNextHookEx(None, 代码, w_param, l_param);
    }

    let 信息 = &*(l_param.0 as *const windows::Win32::UI::WindowsAndMessaging::MSLLHOOKSTRUCT);

    if 信息.flags & LLMHF_INJECTED != 0 {
        return CallNextHookEx(None, 代码, w_param, l_param);
    }

    let x = 信息.pt.x;
    let y = 信息.pt.y;
    let 消息 = w_param.0 as u32;

    let (事件, 吞掉) = match 消息 {
        WM_RBUTTONDOWN => (Some(原始鼠标事件::右键按下 { x, y }), true),
        WM_RBUTTONUP => (Some(原始鼠标事件::右键抬起 { x, y }), true),
        WM_MOUSEMOVE => (Some(原始鼠标事件::鼠标移动 { x, y }), false),
        _ => (None, false),
    };

    if let Some(事件) = 事件 {
        原始事件发送端.with(|槽| {
            if let Some(发送端) = 槽.borrow().as_ref() {
                let _ = 发送端.send(事件);
            }
        });
    }

    if 吞掉 {
        LRESULT(1)
    } else {
        CallNextHookEx(None, 代码, w_param, l_param)
    }
}
