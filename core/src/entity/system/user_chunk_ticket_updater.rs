use crate::entity::{self, component, ArcLockEntityWorld};
use engine::EngineSystem;
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::physics::linear::Position,
	&'c mut component::chunk::TicketOwner,
)>;

pub struct UserChunkTicketUpdater {
	world: Weak<RwLock<entity::World>>,
}

impl UserChunkTicketUpdater {
	pub fn new(world: &ArcLockEntityWorld) -> Self {
		Self {
			world: Arc::downgrade(&world),
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for UserChunkTicketUpdater {
	fn update(&mut self, _delta_time: std::time::Duration, _: bool) {
		profiling::scope!("subsystem:update-user-chunk-tickets");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = arc_world.write().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (_entity, (position, chunk_loader)) in query_bundle.query_mut(&mut world) {
			// The coordinate of the chunk the entity is in
			let current_chunk = *position.chunk();
			// The coordinate of the chunk the loader's ticket is for
			let ticket_chunk = chunk_loader.ticket_coordinate();
			if ticket_chunk.is_none() || ticket_chunk.unwrap() != current_chunk {
				chunk_loader.submit_ticket(current_chunk);
			}
		}
	}
}
