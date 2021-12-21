use crate::{
	entity::{self, ArcLockEntityWorld},
	network::storage::server::user,
};
use engine::{
	network::{self, event, mode, processor::Processor, LocalData},
	utility::VoidResult,
};
use std::sync::{Arc, RwLock, Weak};

pub fn register_bonus_processors(
	builder: &mut network::Builder,
	auth_cache: &user::pending::ArcLockCache,
	active_cache: &user::active::ArcLockCache,
	app_state: &crate::app::state::ArcLockMachine,
	entity_world: &ArcLockEntityWorld,
) {
	use event::Kind::*;

	for event in [Disconnected, Stop].iter() {
		builder.add_processor(
			event.clone(),
			vec![mode::Kind::Client].into_iter(),
			CloseClient {
				app_state: app_state.clone(),
			},
		);
	}
	builder.add_processor(
		Disconnected,
		mode::all().into_iter(),
		RemoveUser {
			auth_cache: auth_cache.clone(),
			active_cache: active_cache.clone(),
		},
	);
	builder.add_processor(
		Disconnected,
		vec![mode::Kind::Server].into_iter(),
		DestroyOwnedEntities {
			entity_world: Arc::downgrade(&entity_world),
		},
	);
}

// Client-Only: Perform operations when the client closes its connection.
#[derive(Clone)]
struct CloseClient {
	app_state: crate::app::state::ArcLockMachine,
}

impl Processor for CloseClient {
	fn process(
		&self,
		kind: &event::Kind,
		_data: &mut Option<event::Data>,
		_local_data: &LocalData,
	) -> VoidResult {
		use crate::app::state::State::*;
		if *kind == event::Kind::Disconnected || *kind == event::Kind::Stop {
			profiling::scope!("close-client-world");
			if let Ok(mut app_state) = self.app_state.write() {
				app_state.transition_to(MainMenu, None);
			}
		}
		Ok(())
	}
}

// Client or Server: Remove active user data from caches when any client disconnects.
#[derive(Clone)]
struct RemoveUser {
	auth_cache: user::pending::ArcLockCache,
	active_cache: user::active::ArcLockCache,
}

impl Processor for RemoveUser {
	fn process(
		&self,
		_kind: &event::Kind,
		data: &mut Option<event::Data>,
		_local_data: &LocalData,
	) -> VoidResult {
		if let Some(event::Data::Connection(connection)) = data {
			profiling::scope!("remove-user");
			if let Ok(mut auth_cache) = self.auth_cache.write() {
				let _ = auth_cache.remove(&connection.address);
			}
			if let Ok(mut active_cache) = self.active_cache.write() {
				let _ = active_cache.remove(&connection.address);
			}
		}
		Ok(())
	}
}

// Server-Only: Destroy entities owned by connections which drop from the network.
#[derive(Clone)]
struct DestroyOwnedEntities {
	entity_world: Weak<RwLock<entity::World>>,
}

impl Processor for DestroyOwnedEntities {
	fn process(
		&self,
		_kind: &event::Kind,
		data: &mut Option<event::Data>,
		_local_data: &LocalData,
	) -> VoidResult {
		use entity::component::net;
		if let Some(event::Data::Connection(connection)) = data {
			profiling::scope!("destroy-owned-entities");
			if let Some(arc_world) = self.entity_world.upgrade() {
				if let Ok(mut world) = arc_world.write() {
					// Iterate over the world and collect all of the entity ids which are owned by the connection
					let mut entities_to_remove = Vec::new();
					for (entity, net_owner) in world.query_mut::<&net::Owner>() {
						if *net_owner.address() == connection.address {
							entities_to_remove.push(entity);
						}
					}
					let id_list = entities_to_remove
						.iter()
						.map(|e| format!("{}", e.id()))
						.collect::<Vec<_>>()
						.join(", ");
					log::info!(
						target: "server",
						"{} has disconnected, despawning {} owned entities: [{}]",
						connection.address,
						entities_to_remove.len(),
						id_list
					);
					for entity in entities_to_remove.into_iter() {
						let _ = world.despawn(entity);
					}
				}
			}
		}
		Ok(())
	}
}
