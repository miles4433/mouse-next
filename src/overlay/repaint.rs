use std::sync::{Arc, Mutex};

use eframe::egui;

pub type Overlay重绘信号 = Arc<Mutex<Option<egui::Context>>>;

pub fn 新建重绘信号() -> Overlay重绘信号 {
    Arc::new(Mutex::new(None))
}

pub fn 请求重绘(信号: &Overlay重绘信号) {
    if let Ok(守卫) = 信号.lock() {
        if let Some(上下文) = 守卫.as_ref() {
            上下文.request_repaint();
        }
    }
}

pub fn 注册重绘上下文(信号: &Overlay重绘信号, 上下文: &egui::Context) {
    if let Ok(mut 守卫) = 信号.lock() {
        if 守卫.is_none() {
            *守卫 = Some(上下文.clone());
        }
    }
}
