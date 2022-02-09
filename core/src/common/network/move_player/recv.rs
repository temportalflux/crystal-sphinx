use std::sync::Arc;

use crate::common::network::move_player::Datum;

use super::Builder;
use engine::socknet::{
	connection::Connection,
	stream::{
		self,
		kind::recv::{self, Datagram},
	},
};

type Context = stream::Context<Builder, recv::Datagram>;

pub struct Handler {
	#[allow(dead_code)]
	context: Arc<Builder>,
	connection: Arc<Connection>,
	recv: recv::Datagram,
}

impl From<Context> for Handler {
	fn from(context: Context) -> Self {
		Self {
			context: context.builder,
			connection: context.connection,
			recv: context.stream,
		}
	}
}

impl stream::handler::Receiver for Handler {
	type Builder = Builder;
	fn receive(mut self) {
		self.connection.clone().spawn(async move {
			use stream::kind::Read;
			use crate::entity::component::{physics::linear, Orientation};
			let data = self.recv.read::<Datum>().await?;

			let arc_world = match self.context.entity_world.upgrade() {
				Some(arc) => arc,
				None => return Ok(()),
			};

			// TODO: Analyze the timestamp and only accept the data if its newer than the last received move update.

			let mut world = arc_world.write().unwrap();
			if let Ok(entity_ref) = world.entity(data.server_entity) {
				if let Some(mut velocity) = entity_ref.get_mut::<linear::Velocity>() {
					**velocity = data.velocity;
				}
				if let Some(mut orientation) = entity_ref.get_mut::<Orientation>() {
					**orientation = data.orientation;
				}
			}

			Ok(())
		});
	}
}
