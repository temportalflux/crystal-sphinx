//! Stream type used to replicate chunk data to a client.
//! There is a fixed-size pool of chunk replication streams created when a client is authenticated.
//!
//! See [Identifier] for stream graph.
use crate::common::network::replication::world::RecvChunks;
use socknet::{connection::Connection, stream};
use std::sync::Weak;

#[doc(hidden)]
mod identifier;
pub use identifier::*;

/// Context & Handler for the client/receiver.
pub mod client;
/// Context & Handler for the server/sender.
pub mod server;

/// Creates a chunk replication stream for the provided connection,
/// given the proper channel for cross-thread communication.
pub fn spawn(
	connection: Weak<Connection>,
	index: usize,
	recv_chunks: RecvChunks,
) -> anyhow::Result<()> {
	let arc = Connection::upgrade(&connection)?;
	let log = format!(
		"{}[{}]",
		<Identifier as stream::Identifier>::log_category("server", &arc),
		index
	);
	arc.spawn(log, async move {
		use stream::handler::Initiator;
		let mut stream = server::Sender::open(&connection)?.await?;
		stream.send_until_closed(index, recv_chunks).await?;
		Ok(())
	});
	Ok(())
}
