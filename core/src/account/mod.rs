pub static LOG: &'static str = "account";

pub type Id = String;

mod account;
pub use account::*;
mod manager;
pub use manager::*;

pub mod key;
