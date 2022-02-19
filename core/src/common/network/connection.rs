mod event;
pub use event::*;

mod list;
pub use list::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Connection list is invalid")]
	InvalidList,
	#[error("Failed to read connection list")]
	FailedToReadList,
	#[error("Failed to write connection list")]
	FailedToWriteList,
}
