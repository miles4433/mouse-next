mod executor;
mod matcher;
mod registry;
mod types;

pub use executor::启动执行器;
pub use matcher::启动匹配器;
pub use types::{待执行动作, 虚拟键操作};
