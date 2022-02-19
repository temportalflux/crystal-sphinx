use crate::{
	app::state,
	common::network::connection,
	common::network::Storage,
	entity::{self},
};
use bus::BusReader;
use engine::{Engine, EngineSystem};
use std::{
	collections::HashSet,
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

static LOG: &'static str = "subsystem:OwnedByConnection";

/// System run on (integrated or dedicated) servers to
/// remove entities from the world when they are owned by
/// a connection which gets dropped (user disconnects).
///
/// This does not handle updating the [`entity-world`](entity::World)
/// when the application leaves the [`InGame`](state::State::InGame) state.
/// See [`entity::add_state_listener`](entity::add_state_listener) for that functionality.
pub struct OwnedByConnection {
	world: Weak<RwLock<entity::World>>,
	receiver: BusReader<connection::Event>,
}

impl OwnedByConnection {
	pub fn add_state_listener(
		app_state: &Arc<RwLock<state::Machine>>,
		arc_storage: Weak<RwLock<Storage>>,
		arc_world: Weak<RwLock<entity::World>>,
	) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_storage = arc_storage.clone();
		let callback_world = arc_world.clone();
		Storage::<Arc<RwLock<Self>>>::default()
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				use crate::common::network::mode;
				profiling::scope!("init-subsystem", LOG);

				// This system should only be active/present while
				// in-game on the (integrated or dedicated) server.
				if !mode::get().contains(mode::Kind::Server) {
					return Ok(None);
				}

				log::info!(target: LOG, "Initializing");

				let world = callback_world.clone();
				let receiver = match callback_storage.upgrade() {
					Some(arc_storage) => {
						let arc_connection_list = {
							let storage = arc_storage.read().unwrap();
							storage.connection_list().clone()
						};
						let mut connection_list = arc_connection_list.write().unwrap();
						connection_list.add_recv()
					}
					None => {
						log::error!(target: LOG, "Failed to find storage");
						return Ok(None);
					}
				};

				let arc_self = Arc::new(RwLock::new(Self { world, receiver }));

				if let Ok(mut engine) = Engine::get().write() {
					engine.add_weak_system(Arc::downgrade(&arc_self));
				}

				return Ok(Some(arc_self));
			});
	}
}

impl EngineSystem for OwnedByConnection {
	fn update(&mut self, _delta_time: std::time::Duration, _has_focus: bool) {
		profiling::scope!(LOG);

		let disconnected = self.poll_receiver();
		if disconnected.is_empty() {
			return;
		}

		let entities = self.gather_owned_entities(disconnected);
		if entities.is_empty() {
			return;
		}

		self.remove_entities(entities);
	}
}

type QueryBundle<'c> = hecs::PreparedQuery<&'c entity::component::OwnedByConnection>;

impl OwnedByConnection {
	#[profiling::function]
	fn poll_receiver(&mut self) -> HashSet<SocketAddr> {
		use connection::Event;
		use std::sync::mpsc::TryRecvError;
		let mut dropped_connections = HashSet::new();
		'poll: loop {
			match self.receiver.try_recv() {
				Ok(Event::Dropped(address)) => {
					dropped_connections.insert(address);
				}
				// NO-OP: We dont care about new connections (neither when created or authenticated)
				Ok(Event::Created(_, _, _)) => {}
				Ok(Event::Authenticated(_, _)) => {}
				Err(TryRecvError::Empty) => {
					// the receiver is empty, we can return the gathered changes
					break 'poll;
				}
				Err(TryRecvError::Disconnected) => {
					// The receiver has no sender,
					// it shouldn't be long before this system is dropped too.
					break 'poll;
				}
			}
		}
		dropped_connections
	}

	#[profiling::function]
	fn gather_owned_entities(
		&self,
		owners: HashSet<SocketAddr>,
	) -> Vec<(hecs::Entity, SocketAddr)> {
		let mut entities = Vec::new();
		let mut query_bundle = QueryBundle::new();
		let arc_world = self.world.upgrade().unwrap();
		let world = arc_world.read().unwrap();
		for (entity, net_owner) in query_bundle.query(&world).iter() {
			let address = *net_owner.address();
			if owners.contains(&address) {
				entities.push((entity, address));
			}
		}
		entities
	}

	#[profiling::function]
	fn remove_entities(&self, entities: Vec<(hecs::Entity, SocketAddr)>) {
		let arc_world = self.world.upgrade().unwrap();
		let mut world = arc_world.write().unwrap();
		for (entity, address) in entities.into_iter() {
			match world.despawn(entity) {
				Ok(_) => {
					log::trace!(
						target: LOG,
						"Successfully despawned entity({}) because its owner({}) disconnected.",
						entity.id(),
						address
					);
				}
				Err(err) => {
					log::error!(
						target: LOG,
						"Failed to despawn entity({}) when its owner({}) disconnected, {:?}",
						entity.id(),
						address,
						err
					);
				}
			}
		}
	}
}
