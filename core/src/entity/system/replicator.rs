use crate::{
	app::state,
	common::network::connection,
	common::network::Storage,
	common::utility::MultiSet,
	entity::{
		self,
		component::{self, binary, network},
		ArcLockEntityWorld,
	},
	server::world::chunk::{self, Chunk},
};
use anyhow::Result;
use bus::BusReader;
use engine::{math::nalgebra::Point3, Engine, EngineSystem};
use multimap::MultiMap;
use socknet::connection::Connection;
use std::{
	collections::{HashMap, HashSet},
	net::SocketAddr,
	sync::{Arc, RwLock, Weak},
};

static LOG: &'static str = "subsystem:replicator";

mod handle;
use handle::*;

pub mod relevancy;

mod instigator;
use instigator::*;

/// Replicates entities on the Server to connected Clients while they are net-relevant.
pub struct Replicator {
	world: Weak<RwLock<entity::World>>,
	chunk_cache: chunk::cache::WeakLock,
	connection_recv: BusReader<connection::Event>,
	connection_handles: HashMap<SocketAddr, Handle>,
	entities_relevant: MultiSet<hecs::Entity, SocketAddr>,
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
				profiling::scope!("init-subsystem", LOG);

				// This system should only be active/present while
				// in-game on the (integrated or dedicated) server.
				if !mode::get().contains(mode::Kind::Server) {
					return Ok(None);
				}

				log::info!(target: LOG, "Initializing");

				let arc_storage = match callback_storage.upgrade() {
					Some(arc_storage) => arc_storage,
					None => {
						log::error!(target: LOG, "Failed to find storage");
						return Ok(None);
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

				let chunk_cache = Arc::downgrade(&server.read().unwrap().chunk_cache());
				let world = callback_world.clone();
				let arc_self = Arc::new(RwLock::new(Self {
					chunk_cache,
					world,
					connection_recv,
					connection_handles: HashMap::new(),
					entities_relevant: MultiSet::default(),
				}));

				if let Ok(mut engine) = Engine::get().write() {
					engine.add_weak_system(Arc::downgrade(&arc_self));
				}

				return Ok(Some(arc_self));
			});
	}
}

#[derive(Default)]
struct OperationGroup {
	socket_ops: MultiMap<SocketAddr, (EntityOperation, hecs::Entity)>,
	entity_ops: MultiMap<hecs::Entity, (EntityOperation, SocketAddr)>,
}
impl OperationGroup {
	fn insert(&mut self, operation: EntityOperation, address: SocketAddr, entity: hecs::Entity) {
		self.socket_ops.insert(address, (operation, entity));
		self.entity_ops.insert(entity, (operation, address));
	}
}

impl EngineSystem for Replicator {
	fn update(&mut self, _delta_time: std::time::Duration, _has_focus: bool) {
		profiling::scope!(LOG);

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};

		let chunk_cache = match self.chunk_cache.upgrade() {
			Some(arc) => arc,
			None => return,
		};

		// Look for any new network connections so their replication streams can be set up.
		let _new_connections = self.poll_connections();

		// Query the world for any updates to entities. This can include but is not limited to entities being:
		// - spawned
		// - data changed (e.g. moved position)
		// - destroyed
		let updates = EntityUpdates::new(&self.entities_relevant);
		let updates = updates.query(&arc_world);
		let updates = updates.collect_chunks(&chunk_cache, &mut self.connection_handles);

		// Entity updates are turned into operations on a given set of connections.
		// This can result in multiple of the same operation for different connections
		// depending on what entities are relevant to which connections.
		let operations =
			updates.as_operations(&mut self.entities_relevant, &self.connection_handles);

		for (address, updates) in updates.into_items().into_iter() {
			let handle = self.get_handle_mut(&address);
			handle.send_relevance_updates(updates);
		}

		// Sends the operations to each connection's handle/input stream
		self.send_entity_updates(&arc_world, operations);
	}
}

#[derive(Default)]
struct RelevanceByConnection(HashMap<SocketAddr, relevancy::PairedRelevance>);
impl RelevanceByConnection {
	fn get_or_insert_mut(&mut self, address: &SocketAddr) -> &mut relevancy::PairedRelevance {
		if !self.0.contains_key(&address) {
			self.0
				.insert(address.clone(), relevancy::PairedRelevance::default());
		}
		self.0.get_mut(&address).unwrap()
	}

	fn into_inner(self) -> HashMap<SocketAddr, relevancy::PairedRelevance> {
		self.0
	}
}

struct GatherEntity<'c> {
	entity: hecs::Entity,
	components: GatherComponents<'c>,
}

use hecs::Query;
#[derive(Query)]
struct GatherComponents<'c> {
	position: &'c mut component::physics::linear::Position,
	owner: Option<&'c component::OwnedByConnection>,
	relevancy: Option<&'c component::chunk::Relevancy>,
	// The `Replicated` component here acts as a flag indicating what entities should get replicated to clients.
	replicated: Option<&'c component::network::Replicated>,
}

impl<'c> GatherEntity<'c> {
	fn query_mut(world: &'c mut hecs::World) -> impl std::iter::Iterator<Item = GatherEntity<'c>> {
		world
			.query_mut::<GatherComponents>()
			.into_iter()
			.map(|(entity, components)| Self { entity, components })
	}

	fn chunk(&self) -> Point3<i64> {
		*self.components.position.chunk()
	}

	fn push_relevance(&self, relevance: &mut RelevanceByConnection) {
		let owner = match self.components.owner {
			Some(comp) => comp,
			None => return,
		};
		let relevancy = match self.components.relevancy {
			Some(comp) => comp,
			None => return,
		};

		let relevance = relevance.get_or_insert_mut(owner.address());
		relevance
			.chunk
			.push(relevancy::Area::new(self.chunk(), relevancy.radius()));
		relevance.entity.push(relevancy::Area::new(
			self.chunk(),
			relevancy.entity_radius(),
		));
	}

	fn is_entity_replicated(&self) -> bool {
		self.components.replicated.is_some()
	}

	fn get_update(&mut self) -> Option<(Option<SocketAddr>, UpdatedEntity)> {
		// If the entity is marked for replication and its position has changed
		// (either it was never acknowledged or it has actually changed),
		// then this will be Some(UpdatedEntity).
		match UpdatedEntity::acknowledged(&self.entity, self.components.position) {
			Some(update) => {
				let address = self.components.owner.map(|owner| *owner.address());
				Some((address, update))
			}
			None => None,
		}
	}
}

struct EntityUpdates {
	relevance: RelevanceByConnection,
	updates: MultiMap<Option<SocketAddr>, UpdatedEntity>,
	destroyed: HashSet<hecs::Entity>,
	new_chunks: MultiMap<SocketAddr, Weak<RwLock<Chunk>>>,
}

impl EntityUpdates {
	fn new(relevant_entities: &MultiSet<hecs::Entity, SocketAddr>) -> Self {
		Self {
			relevance: RelevanceByConnection::default(),
			updates: MultiMap::new(),
			destroyed: relevant_entities.keys().cloned().collect::<HashSet<_>>(),
			new_chunks: MultiMap::new(),
		}
	}

	fn collect_chunks(
		mut self,
		arc_chunk_cache: &chunk::cache::ArcLock,
		connection_handles: &mut HashMap<SocketAddr, Handle>,
	) -> Self {
		let chunk_cache = arc_chunk_cache.read().unwrap();
		for (handle_addr, handle) in connection_handles.iter_mut() {
			let prev_relevance = handle.chunk_relevance();
			let next_relevance = match self.relevance.0.get(handle_addr) {
				Some(relevance) => &relevance.chunk,
				None => continue,
			};
			if *prev_relevance == *next_relevance {
				continue;
			}

			log::debug!(
				"{}: {:?} -> {:?}",
				handle_addr,
				prev_relevance,
				next_relevance
			);

			let mut pending_chunks = next_relevance.difference(&prev_relevance);
			for coordinate in handle.take_pending_chunks().into_iter() {
				if next_relevance.is_relevant(&coordinate) {
					pending_chunks.insert(coordinate);
				}
			}
			let mut pending_chunks = pending_chunks.into_iter().collect::<Vec<_>>();
			next_relevance.sort_vec_by_sig_dist(&mut pending_chunks);
			for coordinate in pending_chunks.into_iter() {
				// If the chunk is in the cache, then the server has it loaded (to some degree).
				// If not, it needs to go back on the component for the next update cycle.
				match chunk_cache.find(&coordinate) {
					Some(weak_chunk) => {
						self.new_chunks
							.insert(handle_addr.clone(), weak_chunk.clone());
					}
					None => {
						handle.insert_pending_chunk(coordinate);
					}
				}
			}
		}
		self
	}

	fn query(mut self, arc_world: &Arc<RwLock<hecs::World>>) -> Self {
		profiling::scope!("query-entity-updates");
		let mut world = arc_world.write().unwrap();
		for mut entity_query in GatherEntity::query_mut(&mut world) {
			entity_query.push_relevance(&mut self.relevance);
			if entity_query.is_entity_replicated() {
				// Prune all entities from `destroyed_entities` that still exist,
				// (leaving it only containing the entities which do not still exist).
				self.destroyed.remove(&entity_query.entity);
				if let Some((address, update)) = entity_query.get_update() {
					self.updates.insert(address, update);
				}
			}
		}
		self
	}

	#[profiling::function]
	fn as_operations(
		&self,
		relevant_entities: &mut MultiSet<hecs::Entity, SocketAddr>,
		connection_handles: &HashMap<SocketAddr, Handle>,
	) -> OperationGroup {
		let mut operations = OperationGroup::default();
		self.gather_destroyed_operations(relevant_entities, &mut operations);
		self.gather_relevancy_diffs(&connection_handles, &mut operations);
		operations
	}

	#[profiling::function]
	fn gather_destroyed_operations(
		&self,
		relevant_entities: &mut MultiSet<hecs::Entity, SocketAddr>,
		operations: &mut OperationGroup,
	) {
		for entity in self.destroyed.iter() {
			if let Some(addresses) = relevant_entities.remove_key(&entity) {
				for address in addresses.into_iter() {
					operations.insert(EntityOperation::Destroyed, address, *entity);
				}
			}
		}
	}

	fn gather_relevancy_diffs(
		&self,
		connection_handles: &HashMap<SocketAddr, Handle>,
		operations: &mut OperationGroup,
	) {
		profiling::scope!(
			"gather_relevancy_diffs",
			&format!("connections={}", connection_handles.len())
		);
		for (_address, updated_entities) in self.updates.iter_all() {
			let _address_id = match _address {
				Some(addr) => addr.to_string(),
				None => "server".to_string(),
			};
			for updated_entity in updated_entities.iter() {
				profiling::scope!(
					"scan-entity",
					&format!(
						"owner={} entity={}",
						_address_id,
						updated_entity.entity.id()
					)
				);
				for (handle_addr, handle) in connection_handles.iter() {
					let was_relevant = match updated_entity.old_chunk {
						Some(old_chunk) => handle.entity_relevance().is_relevant(&old_chunk),
						None => false,
					};
					let is_relevant = match self.relevance.0.get(handle_addr) {
						Some(relevance) => relevance.entity.is_relevant(&updated_entity.new_chunk),
						None => false,
					};
					match (was_relevant, is_relevant) {
						// NO-OP: entity wasn't relevant and still isn't relevant
						(false, false) => {}
						// Is newly relevant with this set of updates
						(false, true) => {
							operations.insert(
								EntityOperation::Relevant,
								*handle_addr,
								updated_entity.entity,
							);
						}
						// Is no longer relevant with this set of updates
						(true, false) => {
							operations.insert(
								EntityOperation::Irrelevant,
								*handle_addr,
								updated_entity.entity,
							);
						}
						// Is still relevant and addr needs entity updates
						(true, true) => {
							operations.insert(
								EntityOperation::Update,
								*handle_addr,
								updated_entity.entity,
							);
						}
					}
				}
			}
		}
	}

	fn into_items(mut self) -> HashMap<SocketAddr, Vec<relevancy::Update>> {
		use relevancy::{Update::*, WorldUpdate};
		let relevance = self.relevance.into_inner();
		let mut items = HashMap::with_capacity(relevance.len());
		for (address, relevance) in relevance.into_iter() {
			let mut updates = Vec::new();
			updates.push(Entity(relevance.entity));
			updates.push(World(WorldUpdate::Relevance(relevance.chunk)));
			if let Some(new_chunks) = self.new_chunks.remove(&address) {
				updates.push(World(WorldUpdate::Chunks(new_chunks)));
			}
			items.insert(address, updates);
		}
		items
	}
}

impl Replicator {
	#[profiling::function]
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
						self.add_connection(address.clone(), &connection);
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

	fn add_connection(&mut self, address: SocketAddr, connection: &Weak<Connection>) {
		let handle = Handle::new(address.clone(), &connection);
		self.connection_handles.insert(address, handle);
	}

	fn remove_connection(&mut self, address: &SocketAddr) {
		// Dropping the stream handler will allow it to finalize any currently
		// transmitting data until the client has fully acknowledged it.
		// The stream will be dropped then, or when the connection is closed (whichever is sooner).
		self.connection_handles.remove(&address);
		self.entities_relevant.remove_value(&address);
	}

	fn get_handle_mut(&mut self, address: &SocketAddr) -> &mut Handle {
		self.connection_handles.get_mut(&address).unwrap()
	}

	#[profiling::function]
	fn send_entity_updates(&mut self, arc_world: &ArcLockEntityWorld, operations: OperationGroup) {
		// Serialize entities which are being replicated for one or more connections
		let entity_data = {
			let world = arc_world.read().unwrap();
			let entities = operations.entity_ops.keys().cloned().collect();
			self.serialize_entities(&world, entities)
		};
		// Update relevancy cache
		for (entity, operations) in operations.entity_ops.into_iter() {
			for (operation, address) in operations.into_iter() {
				match operation {
					EntityOperation::Relevant => {
						self.entities_relevant.insert(&entity, address);
					}
					// NO-OP: Entity has not changed relevancy
					EntityOperation::Update => {}
					EntityOperation::Irrelevant => {
						self.entities_relevant.remove(&entity, &address);
					}
					// NO-OP, addresses for dropped are gathered by removing them from the `entities_relevant` map
					EntityOperation::Destroyed => {}
				}
			}
		}
		// Send operations to relevant connections
		for (address, operations) in operations.socket_ops.into_iter() {
			if let Some(handle) = self.connection_handles.get(&address) {
				handle.send_entity_operations(operations, &entity_data);
			}
		}
	}

	fn serialize_entities(
		&self,
		world: &entity::World,
		entities: HashSet<hecs::Entity>,
	) -> HashMap<hecs::Entity, binary::SerializedEntity> {
		let count = entities.len();
		profiling::scope!("serialize_entities", &format!("count={}", count));
		let mut serialized_entities = HashMap::with_capacity(count);

		let registry = component::Registry::read();
		for entity in entities.into_iter() {
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

		serialized_entities
	}
}

impl Replicator {
	fn serialize_entity(
		&self,
		registry: &component::Registry,
		entity_ref: hecs::EntityRef<'_>,
	) -> Result<binary::SerializedEntity> {
		profiling::scope!(
			"serialize_entity",
			&format!("entity={}", entity_ref.entity().id())
		);
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
