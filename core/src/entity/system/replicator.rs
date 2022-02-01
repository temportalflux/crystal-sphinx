use crate::{
	app::state,
	common::network::connection,
	entity::{
		self,
		component::{self, binary, network},
		ArcLockEntityWorld,
	},
	network::storage::Storage,
	server::world::chunk,
};
use bus::BusReader;
use engine::{
	math::nalgebra::Point3,
	network::{packet::PacketBuilder, socknet::connection::Connection},
	utility::Result,
	Engine, EngineSystem,
};
use multimap::MultiMap;
use std::{
	collections::{HashMap, HashSet},
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

static LOG: &'static str = "subsystem:replicator";

mod stream_handle;
use stream_handle::*;

/// Replicates entities on the Server to connected Clients while they are net-relevant.
pub struct Replicator {
	world: Weak<RwLock<entity::World>>,
	chunk_cache: chunk::cache::WeakLock,
	connection_recv: BusReader<connection::Event>,
	connections: HashMap<SocketAddr, StreamHandle>,

	// Mapping of Entity -> Address List to keep track of to what connections a given entity has been replicated to.
	entities_replicated_to: MultiMap<hecs::Entity, std::net::SocketAddr>,
}

impl Replicator {
	pub fn add_state_listener(
		app_state: &Arc<RwLock<state::Machine>>,
		storage: Weak<RwLock<Storage>>,
		world: Weak<RwLock<entity::World>>,
	) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_storage = storage.clone();
		let callback_world = world.clone();
		Storage::<Arc<RwLock<Self>>>::default()
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				use crate::common::network::mode;

				// This system should only be active/present while
				// in-game on the (integrated or dedicated) server.
				if !mode::get().contains(mode::Kind::Server) {
					return None;
				}

				log::info!(target: LOG, "Initializing");

				let arc_storage = match callback_storage.upgrade() {
					Some(arc_storage) => arc_storage,
					None => {
						log::error!(target: LOG, "Failed to find storage");
						return None;
					}
				};
				let (server, connection_recv) = {
					let storage = arc_storage.read().unwrap();
					let server = storage.server().as_ref().unwrap().clone();
					let connection_recv = {
						let arc_connection_list = storage.connection_list().clone();
						let mut connection_list = arc_connection_list.write().unwrap();
						connection_list.add_recv()
					};
					(server, connection_recv)
				};

				let chunk_cache = server.read().unwrap().chunk_cache();
				let world = callback_world.clone();
				let arc_self = Arc::new(RwLock::new(Self {
					chunk_cache,
					world,
					connection_recv,
					connections: HashMap::new(),
					entities_replicated_to: MultiMap::new(),
				}));

				if let Ok(mut engine) = Engine::get().write() {
					engine.add_weak_system(Arc::downgrade(&arc_self));
				}

				return Some(arc_self);
			});
	}
}

impl EngineSystem for Replicator {
	fn update(&mut self, _delta_time: std::time::Duration, _: bool) {
		profiling::scope!(LOG);

		let new_connections = self.poll_connections();

		type QueryBundle<'c> = hecs::PreparedQuery<(
			&'c component::physics::linear::Position,
			&'c mut component::OwnedByConnection,
			&'c mut component::chunk::Relevancy,
		)>;

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};

		let (updated_connections, chunk_packets) = {
			// its possible for a connection to have multiple owned entities (in theory),
			// so this needs to be a multimap where each address can have multiple chunk locations.
			let mut connections = MultiMap::new();
			let mut chunk_packets = Vec::with_capacity(10);
			let mut world = arc_world.write().unwrap();
			let mut query_bundle = QueryBundle::new();
			for (entity, (position, owner, relevancy)) in query_bundle.query_mut(&mut world) {
				// Get a list of all connections which have not been replicated to.
				// This marks each as having been replicated (because it is garunteed to happen in this update).
				let already_replicated = owner.has_been_replicated();
				if !already_replicated {
					connections.insert(
						*owner.address(),
						(None, *position.chunk(), relevancy.entity_radius()),
					);
					owner.mark_as_replicated();
				}

				// Replicate relevant chunks to connections based on the position of owner entities.
				{
					// Chunk coordinate of where the entity is now
					let current_chunk = *position.chunk();
					// Chunk coordinate of the last replication
					let previous_chunk = *relevancy.chunk();
					// If the current chunk is unchanged and there are no
					// chunks to replicate, then this can early out.
					if current_chunk != previous_chunk || !relevancy.has_replicated_all() {
						if already_replicated {
							connections.insert(
								*owner.address(),
								(
									Some(previous_chunk),
									current_chunk,
									relevancy.entity_radius(),
								),
							);
						}
						let mut packets =
							self.replicate_chunks_to(entity, owner, current_chunk, relevancy);
						chunk_packets.append(&mut packets);
					}
				}
			}
			(connections, chunk_packets)
		};

		let entity_packets = self.replicate_entities(&arc_world, updated_connections);

		/* TODO: Reimplement with new networking
		let _ = engine::network::Network::send_all_packets(entity_packets);
		let _ = engine::network::Network::send_all_packets(chunk_packets);
		*/

		// TODO: Replicate updates on net::BinarySerializable components
		// - (net::BinarySerializable should have a flag to indicate that it is dirty)
	}
}

impl Replicator {
	fn poll_connections(&mut self) -> HashSet<SocketAddr> {
		use connection::Event;
		use std::sync::mpsc::TryRecvError;
		let mut new_connections = HashSet::new();
		'poll: loop {
			match self.connection_recv.try_recv() {
				Ok(Event::Created(address, connection, is_local)) => {
					// We do not create a replication stream for "local" connections,
					// where the defn of local in this context is the same application,
					// aka an Integrated Server / Client-on-top-of-Server situation.
					// Since a CotoS has a shared world between client and server,
					// there is no point in wasting cycles pretending to replicate data.
					if !is_local {
						self.add_connection(address.clone(), connection);
						new_connections.insert(address);
					}
				}
				Ok(Event::Dropped(address)) => {
					self.remove_connection(&address);
				}
				Err(TryRecvError::Empty | TryRecvError::Disconnected) => {
					// NO-OP:
					// If empty, there is nothing to do.
					// If disconnected, then the appstate will transition
					// soon and the replicator will be destroyed.
					break 'poll;
				}
			}
		}
		new_connections
	}

	fn add_connection(&mut self, address: SocketAddr, connection: Weak<Connection>) {
		let handle = StreamHandle::new(address.clone(), connection);
		self.connections.insert(address, handle);
	}

	fn remove_connection(&mut self, address: &SocketAddr) {
		// Dropping the stream handler will allow it to finalize any currently
		// transmitting data until the client has fully acknowledged it.
		// The stream will be dropped then, or when the connection is closed (whichever is sooner).
		self.connections.remove(&address);
	}
}

enum EntityOperation {
	Replicate(hecs::Entity),
	Destroy(hecs::Entity),
}

impl Replicator {
	fn replicate_chunks_to(
		&self,
		entity: hecs::Entity,
		owner: &component::OwnedByConnection,
		current_chunk: Point3<i64>,
		relevancy: &mut component::chunk::Relevancy,
	) -> Vec<PacketBuilder> {
		use crate::network::packet::ReplicateWorld;
		use engine::network::{enums::*, packet::Packet};

		profiling::scope!(
			"replicate_chunks_to",
			&format!("entity={} address={}", entity.id(), owner.address())
		);

		let (new_chunks, old_chunks) = relevancy.get_chunk_diff(&current_chunk);
		// Server side, update the component directly
		relevancy.update_replicated_chunks(current_chunk, &old_chunks, &new_chunks);

		let old_chunks = {
			profiling::scope!("sort-old-chunks");
			let mut old_chunks = old_chunks.into_iter().collect::<Vec<_>>();
			old_chunks.sort_by(|a, b| {
				let a_dist = (a - current_chunk).cast::<f32>().magnitude_squared();
				let b_dist = (b - current_chunk).cast::<f32>().magnitude_squared();
				b_dist.partial_cmp(&a_dist).unwrap()
			});
			old_chunks
		};

		// Get all the coordinates that should be replicated once they are available/loaded.
		// This might be some number of updates after the user moves to a new chunk,
		// so there may be a list of pending chunks for some amount of time.
		let chunks_to_replicate = relevancy.take_pending_chunks(8);
		let mut updates = Vec::with_capacity(chunks_to_replicate.len());
		if let Ok(chunk_cache) = self.chunk_cache.upgrade().unwrap().read() {
			for coordinate in chunks_to_replicate.into_iter() {
				profiling::scope!(
					"create-chunk-packet",
					&format!("<{}, {}, {}>", coordinate.x, coordinate.y, coordinate.z)
				);
				// If the chunk is in the cache, then the server has it loaded (to some degree).
				// If not, it needs to go back on the component for the next update cycle.
				match chunk_cache.find(&coordinate) {
					Some(weak_chunk) => {
						let arc_chunk = weak_chunk.upgrade().unwrap();
						let server_chunk = arc_chunk.read().unwrap();
						// Conver the chunk into replication data and add it to the list of things to send.
						let mut packets = ReplicateWorld::create_chunk_packets(&server_chunk.chunk);
						updates.append(&mut packets);
						relevancy.mark_as_replicated(coordinate);
					}
					None => {
						relevancy.insert_pending(coordinate);
					}
				}
			}
		}

		let mut packets = Vec::with_capacity(2);
		/*
		{
			profiling::scope!("send-packets");
			packets.push(
				Packet::builder()
					.with_address(*owner.address())
					.unwrap()
					.with_guarantee(Reliable + Ordered)
					// Notify client what chunks are no longer relevant (can be dropped),
					// and what chunks will be incoming over the network shortly.
					.with_payloads(&ReplicateWorld::fragment_relevancy(entity, old_chunks)),
			);
			packets.push(
				Packet::builder()
					.with_address(*owner.address())
					.unwrap()
					.with_guarantee(Reliable + Ordered)
					// Send each chunk update in its own Reliably-Ordered packet,
					// which is garunteed to be received by clients after the initial update.
					.with_payloads(&updates[..]),
			);
		}
		*/
		packets
	}

	fn is_chunk_within_radius(origin: &Point3<i64>, coord: &Point3<i64>, radius: usize) -> bool {
		let origin_to_coord = coord.coords - origin.coords;
		origin_to_coord.dot(&origin_to_coord) <= (radius as i64).pow(2)
	}

	#[profiling::function]
	fn replicate_entities(
		&mut self,
		arc_world: &ArcLockEntityWorld,
		updated_connections: MultiMap<
			std::net::SocketAddr,
			(Option<Point3<i64>>, Point3<i64>, usize),
		>,
	) -> Vec<PacketBuilder> {
		use crate::network::packet::replicate_entity;
		use engine::network::{enums::*, packet::Packet};
		type QueryBundle<'c> = hecs::PreparedQuery<(
			&'c component::physics::linear::Position,
			&'c component::network::Replicated,
		)>;

		let mut world = arc_world.write().unwrap();

		// List of all entities that need to be serialized,
		// because they are in at least 1 `EntityOperation::Replicate` in `operations`.
		let mut additions = HashSet::new();
		let mut removals = MultiMap::new();
		// Map of address to the replication operations for adding/removing entities for that client.
		let mut operations = MultiMap::new();
		if !updated_connections.is_empty() {
			profiling::scope!(
				"gather-changes",
				&format!("changed-connections:{}", updated_connections.len())
			);
			let mut query_bundle = QueryBundle::new();
			for (entity, (position, _replicated)) in query_bundle.query_mut(&mut world) {
				for (address, areas_of_effect) in updated_connections.iter_all() {
					// true if the `entity` was relevant to this address for any of its "owned" areas
					let mut was_relevant = false;
					// true if the `entity` is relevant (still or newly) to this address for any of its "owned" areas
					let mut is_relevant = false;
					'owned_area_iter: for (prev_chunk, next_chunk, entity_radius) in
						areas_of_effect.iter()
					{
						// once we determine that the entity was relevant to some area, we don't need to check for other areas
						if !was_relevant {
							if let Some(coord) = prev_chunk {
								if Self::is_chunk_within_radius(
									&coord,
									position.chunk(),
									*entity_radius,
								) {
									was_relevant = true;
								}
							}
						}
						// same here, once it becomes relevant, that cant be turned off by any other area
						if !is_relevant {
							is_relevant = Self::is_chunk_within_radius(
								&next_chunk,
								position.chunk(),
								*entity_radius,
							);
						}
						// if there are no more flags to update, we dont need to continue iterating over the areas
						if was_relevant && is_relevant {
							break 'owned_area_iter;
						}
					}
					if was_relevant && !is_relevant {
						operations.insert(*address, EntityOperation::Destroy(entity));
						removals.insert(entity, *address);
					} else if !was_relevant && is_relevant {
						operations.insert(*address, EntityOperation::Replicate(entity));
						additions.insert(entity);
						// Record that the entity will be replicated to the client,
						// so that if/when the entity is despawned, we can purge it from that client.
						self.entities_replicated_to.insert(entity, *address);
					}
				}
			}
		}

		if !removals.is_empty() {
			// Remove addresses from the entity for which the entity is no longer relevant.
			self.entities_replicated_to
				.retain(|entity, address| match removals.get_vec(&entity) {
					Some(addresses) => !addresses.contains(&address),
					None => true,
				});
		}

		let mut serialized_entities = HashMap::new();
		if !additions.is_empty() {
			profiling::scope!(
				"serialize-relevant-entities",
				&format!("{} entities", additions.len())
			);
			let registry = component::Registry::read();
			for entity in additions.into_iter() {
				profiling::scope!("serialize-entity", &format!("entity:{}", entity.id()));

				let entity_ref = world.entity(entity).unwrap();
				assert!(entity_ref.has::<network::Replicated>());
				match self.serialize_entity(&registry, entity_ref) {
					Ok(serialized) => {
						serialized_entities.insert(entity, serialized);
					}
					Err(err) => {
						log::error!(target: "entity-replicator", "Encountered error while serializing entity: {}", err)
					}
				}
			}
		}

		// Purge any entities which have been despawned but were replicated to connections
		if !self.entities_replicated_to.is_empty() {
			let mut despawned_entities = Vec::new();
			for (entity, addresses) in self.entities_replicated_to.iter_all() {
				if !world.contains(*entity) {
					despawned_entities.push(*entity);
					for address in addresses.iter() {
						operations.insert(*address, EntityOperation::Destroy(*entity));
					}
				}
			}
			if !despawned_entities.is_empty() {
				self.entities_replicated_to
					.retain(|entity, _| !despawned_entities.contains(entity));
			}
		}

		let mut packets = Vec::with_capacity(operations.keys().count());
		for (address, operations) in operations.iter_all() {
			/*
			profiling::scope!("send-entity-packets", &format!("connection:{address}"));
			let operations = operations
				.iter()
				.map(|op| match op {
					EntityOperation::Replicate(eid) => replicate_entity::Operation::Replicate(
						serialized_entities.get(&eid).cloned().unwrap(),
					),
					EntityOperation::Destroy(eid) => replicate_entity::Operation::Destroy(*eid),
				})
				.collect();
			packets.push(
				Packet::builder()
					.with_address(address)
					.unwrap()
					// Integrated Client-Server should not sent to itself
					.ignore_local_address()
					.with_guarantee(Reliable + Unordered)
					.with_payload(&replicate_entity::Packet { operations }),
			);
			*/
		}
		packets
	}

	fn serialize_entity(
		&self,
		registry: &component::Registry,
		entity_ref: hecs::EntityRef<'_>,
	) -> Result<binary::SerializedEntity> {
		let mut serialized_components = Vec::new();
		for type_id in entity_ref.component_types() {
			if let Some(registered) = registry.find(&type_id) {
				// Skip any components that are not marked as network replicatable.
				match registered.get_ext::<network::Registration>() {
					None => continue,
					Some(_) => {}
				}
				let binary_registration = match registered.get_ext::<binary::Registration>() {
					Some(reg) => reg,
					None => {
						log::error!(
							target: "Replicator",
							"Failed to serialize type {}, missing binary serializable extension.",
							registered.id()
						);
						continue;
					}
				};
				// If `serializable` returns None, it means the component wasn't actually on that entity.
				// Since the type-id came from the entity itself, the component MUST exist on the entity_ref,
				// so it should be safe to unwrap directly.
				let serialized = binary_registration.serialize(&entity_ref)?.unwrap();
				serialized_components.push(serialized);
			}
		}
		Ok(binary::SerializedEntity {
			entity: entity_ref.entity(),
			components: serialized_components,
		})
	}
}
