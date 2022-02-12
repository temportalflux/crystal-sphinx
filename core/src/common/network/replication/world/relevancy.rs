mod identifier;
use std::sync::Weak;

pub use identifier::*;
use socknet::connection::Connection;

use crate::common::network::replication::world::{RecvUpdate, SendChunks};

pub mod client;
pub mod server;

pub fn spawn(
	connection: Weak<Connection>,
	channel: RecvUpdate,
	send_chunks: SendChunks,
) -> anyhow::Result<()> {
	let arc = Connection::upgrade(&connection)?;
	arc.spawn(async move {
		use socknet::stream::handler::Initiator;
		let mut stream = server::Sender::open(&connection)?.await?;
		stream.send_until_closed(channel, send_chunks).await?;
		Ok(())
	});
	Ok(())
}
