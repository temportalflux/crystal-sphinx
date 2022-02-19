//! Stream used to replicate entities and their components to relevant clients.
//!
//! See [Identifier] for stream graph.
use socknet::connection::Connection;
use std::sync::Weak;

#[doc(hidden)]
mod identifier;
pub use identifier::*;

#[doc(hidden)]
mod update;
pub use update::*;

/// Context & Handler for the client/receiver.
pub mod client;
/// Context & Handler for the server/sender.
pub mod server;

/// Spawns an entity replication stream, which persists until either the provided connection or channel are closed (whichever comes first).
pub fn spawn(connection: Weak<Connection>, channel: RecvUpdate) -> anyhow::Result<()> {
	use socknet::stream;
	let arc = Connection::upgrade(&connection)?;
	let log = <Identifier as stream::Identifier>::log_category("server", &arc);
	arc.spawn(log, async move {
		use stream::handler::Initiator;
		let mut stream = server::Sender::open(&connection)?.await?;
		stream.send_until_closed(channel).await?;
		Ok(())
	});
	Ok(())
}
