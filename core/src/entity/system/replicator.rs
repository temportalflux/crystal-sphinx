use crate::entity::{
	self,
	component::{net, Position},
	ArcLockEntityWorld,
};
use engine::EngineSystem;
use std::sync::{Arc, RwLock, Weak};

/// Replicates entities on the Server to connected Clients while they are net-relevant.
pub struct Replicator {
	world: Weak<RwLock<entity::World>>,
}

impl Replicator {
	pub fn new(world: &ArcLockEntityWorld) -> Self {
		Self {
			world: Arc::downgrade(&world),
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for Replicator {
	fn update(&mut self, _delta_time: std::time::Duration) {
		profiling::scope!("subsystem:replicator");

		// TODO: more entity replication
		// - Replicate all entities & components which implement net::Replicated to net::Owner connections when they come within range of the owner
		// - Destroy entities from connections when they leave net relevance
		// - While relevant, replicate updates on net::Replicated components (net::Replicated should have a flag to indicate that it is dirty)

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		self.replicate_owned_entities(&arc_world);
	}
}

impl Replicator {
	#[profiling::function]
	fn replicate_owned_entities(&self, arc_world: &ArcLockEntityWorld) {
		use crate::network::packet::ReplicateEntity;
		use engine::network::{enums::*, packet::Packet, Network};

		let mut world = arc_world.write().unwrap();
		let mut entities_to_replicate = vec![];
		for (id, owner) in world.query_mut::<&mut net::Owner>() {
			if !owner.has_been_replicated() {
				entities_to_replicate.push((id, *owner.address()));
				owner.mark_as_replicated();
			}
		}

		let mut replications = Vec::new();
		let registry = net::Registry::read();
		for (entity, address) in entities_to_replicate.into_iter() {
			let scope_tag = format!("entity:{}", entity.id());
			profiling::scope!("serialize-entity", scope_tag.as_str());

			let entity_ref = world.entity(entity).unwrap();
			match registry.serialize_entity(entity_ref) {
				Ok(serialized) => {
					replications.push((address.clone(), serialized));
				}
				Err(err) => {
					log::error!(target: "entity-replicator", "Encountered error while serializing entity: {}", err)
				}
			}
		}

		for (address, serialized) in replications.into_iter() {
			let _ = Network::send_packets(
				Packet::builder()
					.with_address(address)
					.unwrap()
					// Integrated Client-Server should not sent to itself
					.ignore_local_address()
					.with_guarantee(Reliable + Unordered)
					.with_payload(&ReplicateEntity {
						entities: vec![serialized],
					}),
			);
		}
	}

	fn _owned_entities(
		&self,
		arc_world: &ArcLockEntityWorld,
	) -> Vec<(hecs::Entity, std::net::SocketAddr, Position)> {
		let world = arc_world.read().unwrap();
		let entities = world
			.query::<(&net::Owner, &Position)>()
			.iter()
			.map(|(entity, (&owner, &position))| (entity, *owner.address(), position))
			.collect::<Vec<_>>();
		entities
	}
}
