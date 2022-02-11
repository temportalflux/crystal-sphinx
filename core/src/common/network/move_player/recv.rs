use std::sync::Arc;
use crate::common::network::move_player::{Datum, Builder};
use engine::socknet::{
	connection::{Connection, Active},
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
			use crate::entity::component::{physics::linear, Orientation};
			use stream::kind::Read;
			let data = self.recv.read::<Datum>().await?;

			// Analyze the timestamp and only accept the data if its newer than the last received move update.
			if let Ok(mut sequencer) = self.context.sequencer.write() {
				let address = self.connection.remote_address();
				if let Some(prev_timestamp) = sequencer.get(&address) {
					if data.timestamp <= *prev_timestamp {
						return Ok(());
					}
				}
				sequencer.insert(address, data.timestamp);
			}

			let arc_world = match self.context.entity_world.upgrade() {
				Some(arc) => arc,
				None => return Ok(()),
			};

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
