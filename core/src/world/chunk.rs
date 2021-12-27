//! Contains all world chunk structures around submitting chunk tickets, data contained in a chunk, and how chunks are loaded.

use engine::math::nalgebra::Vector3;
pub static SIZE_I: Vector3<usize> = Vector3::new(16, 16, 16);
pub static SIZE: Vector3<f32> = Vector3::new(16.0, 16.0, 16.0);

mod cache;
pub use cache::*;

mod chunk;
pub use chunk::*;

mod level;
pub use level::*;

pub(crate) mod ticket;
pub use ticket::Ticket;

/// Structures & Functions used internally to handle the loading of chunks on a thread.
pub(crate) mod thread;
