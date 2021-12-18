mod cache;
pub use cache::*;

mod chunk;
pub use chunk::*;

mod level;
pub use level::*;

mod ticket;
pub use ticket::*;

pub(crate) mod thread;

pub type LoadRequestSender = crossbeam_channel::Sender<std::sync::Weak<Ticket>>;
pub type LoadRequestReceiver = crossbeam_channel::Receiver<std::sync::Weak<Ticket>>;
