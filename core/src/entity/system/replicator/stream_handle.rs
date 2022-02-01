use crate::common::network::replication::entity::send;
use engine::{
	network::socknet::{connection::Connection, stream},
	task::JoinHandle,
	utility::Result,
};
use std::{net::SocketAddr, sync::Weak};

/// The connective tissue between the [`replicator`](super::Replicator) system
/// and the async task which dispatches entity replication data to a given client.
/// Its lifetime is owned by the replicator system.
pub struct StreamHandle {
	task_handle: JoinHandle<()>,
}

impl StreamHandle {
	pub fn new(address: SocketAddr, connection: Weak<Connection>) -> Self {
		let task_handle = engine::task::spawn(send::Handler::log_target(&address), async move {
			use stream::handler::Initiator;
			let stream = send::Handler::open(&connection)?.await?;
			let stream = stream.initiate().await?;
			// TODO: Dispatch updates to this connection until the handle is dropped
			Ok(())
		});
		Self { task_handle }
	}
}

impl Drop for StreamHandle {
	fn drop(&mut self) {
		// Aborting the task also means dropping the stream handler
		self.task_handle.abort();
	}
}
