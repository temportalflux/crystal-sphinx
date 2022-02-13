use socknet::{connection::Connection, stream};
use std::sync::Weak;

use crate::common::network::replication::world::RecvChunks;

mod identifier;
pub use identifier::*;

pub mod client;
pub mod server;

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
