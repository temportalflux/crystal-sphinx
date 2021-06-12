mod config;
pub use config::*;
mod manager;
pub use manager::*;
mod plugin;
pub use plugin::*;

pub static LOG: &'static str = "plugin";
