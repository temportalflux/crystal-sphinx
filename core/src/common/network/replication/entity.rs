mod identifier;
use std::sync::Weak;

pub use identifier::*;
use socknet::connection::Connection;

pub mod update;

pub mod client;
pub mod server;

pub fn spawn(connection: Weak<Connection>, channel: update::Receiver) -> anyhow::Result<()> {
	let arc = Connection::upgrade(&connection)?;
	arc.spawn(async move {
		use socknet::stream::handler::Initiator;
		let mut stream = server::Sender::open(&connection)?.await?;
		stream.send_until_closed(channel).await?;
		Ok(())
	});
	Ok(())
}
