use std::mem;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::ColorImage;
use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, HDC,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect, IsWindowVisible};

use crate::config::冻结缓存延迟毫秒;

const PW_RENDERFULLCONTENT: u32 = 2;

#[link(name = "user32")]
extern "system" {
    fn PrintWindow(hwnd: HWND, hdc: HDC, flags: u32) -> BOOL;
}

/// 动作后延迟重新截屏；会话开始与每次方向触发均走此路径。
/// 代际号用于连切时丢弃过期的截屏任务。
pub fn 调度延迟刷新缓存(
    状态: super::state::Overlay共享状态,
    重绘信号: super::repaint::Overlay重绘信号,
    排除句柄: Option<isize>,
) {
    let 代际 = 状态
        .lock()
        .map(|mut 状态| 状态.推进缓存刷新代际())
        .unwrap_or(0);

    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(冻结缓存延迟毫秒));

        if 状态
            .lock()
            .ok()
            .is_some_and(|状态| 状态.缓存刷新代际 != 代际)
        {
            return;
        }

        let 排除 = 排除句柄.map(|句柄| HWND(句柄 as *mut core::ffi::c_void));
        let Some(条目) = 截取前台窗口(排除) else {
            return;
        };

        if let Ok(mut 状态) = 状态.lock() {
            if 状态.缓存刷新代际 != 代际 {
                return;
            }
            状态.缓存冻结帧 = Some(Arc::new(条目));
        }
        super::repaint::请求重绘(&重绘信号);
    });
}

/// 用 PrintWindow 截取前台窗口一帧，不含 overlay 自身。
pub fn 截取前台窗口(排除句柄: Option<HWND>) -> Option<super::state::冻结缓存条目> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        if 排除句柄.is_some_and(|排除| 排除 == hwnd) {
            return None;
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return None;
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return None;
        }

        let 宽 = rect.right - rect.left;
        let 高 = rect.bottom - rect.top;
        if 宽 <= 0 || 高 <= 0 {
            return None;
        }

        let hdc_屏幕 = GetDC(None);
        if hdc_屏幕.is_invalid() {
            return None;
        }

        let hdc_内存 = CreateCompatibleDC(hdc_屏幕);
        if hdc_内存.is_invalid() {
            ReleaseDC(None, hdc_屏幕);
            return None;
        }

        let 位图 = CreateCompatibleBitmap(hdc_屏幕, 宽, 高);
        if 位图.is_invalid() {
            let _ = DeleteDC(hdc_内存);
            ReleaseDC(None, hdc_屏幕);
            return None;
        }

        let 旧对象 = SelectObject(hdc_内存, HGDIOBJ(位图.0));

        let 成功 = PrintWindow(hwnd, hdc_内存, PW_RENDERFULLCONTENT).as_bool();
        if !成功 {
            SelectObject(hdc_内存, 旧对象);
            let _ = DeleteObject(HGDIOBJ(位图.0));
            let _ = DeleteDC(hdc_内存);
            ReleaseDC(None, hdc_屏幕);
            return None;
        }

        let mut 信息 = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 宽,
                biHeight: -高,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        let 像素数 = (宽 * 高) as usize;
        let mut 像素 = vec![0u8; 像素数 * 4];

        let 行数 = GetDIBits(
            hdc_内存,
            位图,
            0,
            高 as u32,
            Some(像素.as_mut_ptr().cast()),
            &mut 信息,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_内存, 旧对象);
        let _ = DeleteObject(HGDIOBJ(位图.0));
        let _ = DeleteDC(hdc_内存);
        ReleaseDC(None, hdc_屏幕);

        if 行数 == 0 {
            return None;
        }

        for 块 in 像素.chunks_exact_mut(4) {
            块.swap(0, 2);
        }

        Some(super::state::冻结缓存条目 {
            图像: Arc::new(ColorImage::from_rgba_unmultiplied(
                [宽 as usize, 高 as usize],
                &像素,
            )),
            屏幕左: rect.left,
            屏幕上: rect.top,
        })
    }
}