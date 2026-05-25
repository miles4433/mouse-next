#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 原始鼠标事件 {
    右键按下 { x: i32, y: i32 },
    右键抬起 { x: i32, y: i32 },
    鼠标移动 { x: i32, y: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum 方向 {
    开始,
    结束,
    上,
    下,
    左,
    右,
}
