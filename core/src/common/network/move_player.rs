use anyhow::Result;
use chrono::{DateTime, Utc};
use engine::math::nalgebra::{UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use socknet::{connection::Connection, stream};
use std::sync::Weak;

mod identifier;
pub use identifier::*;
pub mod client;
pub mod server;

#[derive(Serialize, Deserialize, Clone)]
pub struct Datum {
	pub timestamp: DateTime<Utc>,
	pub server_entity: hecs::Entity,
	pub velocity: Vector3<f32>,
	pub orientation: UnitQuaternion<f32>,
}

impl Datum {
	pub fn send(self, connection: Weak<Connection>) -> Result<()> {
		let arc = Connection::upgrade(&connection)?;
		let log = <Identifier as stream::Identifier>::log_category("client", &arc);
		arc.spawn(log, async move {
			use stream::handler::Initiator;
			let mut stream = client::Sender::open(&connection)?.await?;
			stream.send_datum(self).await?;
			Ok(())
		});
		Ok(())
	}
}
