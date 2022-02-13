mod identifier;
use std::sync::Weak;

pub use identifier::*;
use socknet::connection::Connection;

pub mod update;

pub mod client;
pub mod server;

pub fn spawn(connection: Weak<Connection>, channel: update::Receiver) -> anyhow::Result<()> {
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
