mod executor;
mod matcher;
mod preview;
mod registry;
mod types;

pub use preview::会话预览;
pub use registry::{动作表条目, 动作条目, 默认动作列表};

pub use executor::启动执行器;
pub use matcher::启动匹配器;
pub use types::{待执行动作, 虚拟键操作};
