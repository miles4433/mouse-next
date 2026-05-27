use std::thread;
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
    MOUSE_EVENT_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

const 补发前等待: Duration = Duration::from_millis(24);
const 按下保持: Duration = Duration::from_millis(48);

pub fn 调度补发右键单击(x: i32, y: i32) {
    thread::spawn(move || {
        thread::sleep(补发前等待);
        补发右键单击(x, y);
    });
}

fn 补发右键单击(x: i32, y: i32) {
    let (绝对_x, 绝对_y) = 屏幕坐标转绝对(x, y);
    let 移动 = mouse_input(
        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        绝对_x,
        绝对_y,
    );
    let 按下 = mouse_input(MOUSEEVENTF_RIGHTDOWN, 0, 0);
    unsafe {
        let _ = SendInput(&[移动, 按下], std::mem::size_of::<INPUT>() as i32);
    }
    thread::sleep(按下保持);
    let 抬起 = mouse_input(MOUSEEVENTF_RIGHTUP, 0, 0);
    unsafe {
        let _ = SendInput(&[抬起], std::mem::size_of::<INPUT>() as i32);
    }
}

fn mouse_input(标志: MOUSE_EVENT_FLAGS, dx: i32, dy: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                dwFlags: 标志,
                ..Default::default()
            },
        },
    }
}

fn 屏幕坐标转绝对(x: i32, y: i32) -> (i32, i32) {
    unsafe {
        let 屏宽 = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1);
        let 屏高 = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1);
        let 屏左 = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let 屏顶 = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let 绝对_x = ((x - 屏左) * 65535 / 屏宽) as i32;
        let 绝对_y = ((y - 屏顶) * 65535 / 屏高) as i32;
        (绝对_x, 绝对_y)
    }
}
