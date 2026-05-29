mod executor;
mod matcher;
mod panel_table;
mod types;

pub use panel_table::{
    全部槽位, 处理进入槽, 命中槽, 槽位中心, 宫格事件, 宫格槽, 层, 槽显示状态, 槽离开需截屏, 匹配触碰动作,
    匹配松手动作, 同槽重复动作, 动作是_tab, 格显示, 格符号, 显示级别,
};

pub use executor::启动执行器;
pub use matcher::启动匹配器;
pub use types::待执行动作;
