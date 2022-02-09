use chrono::{DateTime, Utc};
use engine::{
	math::nalgebra::{UnitQuaternion, Vector3},
	socknet::{connection::Connection, stream},
	utility::Result,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Weak};

mod builder;
pub use builder::*;
mod send;
use send::*;
mod recv;
use recv::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct Datum {
	pub timestamp: DateTime<Utc>,
	pub server_entity: hecs::Entity,
	pub velocity: Vector3<f32>,
	pub orientation: UnitQuaternion<f32>,
}

impl Datum {
	pub fn send(self, connection: Weak<Connection>) -> Result<()> {
		Connection::upgrade(&connection)?.spawn(async move {
			use stream::handler::Initiator;
			let mut stream = Sender::open(&connection)?.await?;
			stream.initiate().await?;
			stream.send_datum(self).await?;
			Ok(())
		});
		Ok(())
	}
}
