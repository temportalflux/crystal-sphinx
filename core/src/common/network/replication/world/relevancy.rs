//! Stream used to replicate world-relevancy updates to a specific client.
//! Specifically this replicates the location and radius of relevancy triggers
//! to the owning client so the client knows both
//! what chunks to discard locally (were previously relevant and are no longer relevant)
//! and what chunks to expect in the chunk replication streams (no previously relevant and are now relevant).
//!
//! See [Identifier] for stream graph.
use crate::common::network::replication::world::{RecvUpdate, SendChunks};
use socknet::connection::Connection;
use std::sync::Weak;

#[doc(hidden)]
mod identifier;
pub use identifier::*;

/// Context & Handler for the client/receiver.
pub mod client;
/// Context & Handler for the server/sender.
pub mod server;

/// Creates a world relevancy stream for the provided connection,
/// given the proper channels for cross-thread communication.
pub fn spawn(
	connection: Weak<Connection>,
	channel: RecvUpdate,
	send_chunks: SendChunks,
) -> anyhow::Result<()> {
	use socknet::stream;
	let arc = Connection::upgrade(&connection)?;
	let log = <Identifier as stream::Identifier>::log_category("server", &arc);
	arc.spawn(log, async move {
		use stream::handler::Initiator;
		let mut stream = server::Sender::open(&connection)?.await?;
		stream.send_until_closed(channel, send_chunks).await?;
		Ok(())
	});
	Ok(())
}
