pub static LOG: &'static str = "account";

pub type Id = uuid::Uuid;

mod account;
pub use account::*;
mod client_registry;
pub use client_registry::*;
mod manager;
pub use manager::*;

pub mod key;
