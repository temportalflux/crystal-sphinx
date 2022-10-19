use crate::{
	client::model::{
		blender,
		instance::{self, Instance},
		texture, PlayerModel,
	},
	entity::{self, component},
};
use engine::{asset, Engine, EngineSystem, math::nalgebra::UnitQuaternion, world};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex, RwLock, Weak};

static LOG: &'static str = "subsystem:GatherEntitiesToRender";

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::physics::linear::Position,
	&'c component::Orientation,
	Option<&'c blender::Component>,
	Option<&'c PlayerModel>,
)>;

pub struct GatherEntitiesToRender {
	world: Weak<RwLock<entity::World>>,
	instance_buffer: Weak<RwLock<instance::Buffer>>,
	texture_cache: Weak<Mutex<texture::Cache>>,
}

impl GatherEntitiesToRender {
	pub fn create(
		world: Weak<RwLock<entity::World>>,
		instance_buffer: &Arc<RwLock<instance::Buffer>>,
		texture_cache: &Arc<Mutex<texture::Cache>>,
	) -> Arc<RwLock<Self>> {
		let arclocked = Arc::new(RwLock::new(Self {
			world,
			instance_buffer: Arc::downgrade(&instance_buffer),
			texture_cache: Arc::downgrade(&texture_cache),
		}));

		if let Ok(mut engine) = Engine::get().write() {
			engine.add_weak_system(Arc::downgrade(&arclocked));
		}

		arclocked
	}
}

impl EngineSystem for GatherEntitiesToRender {
	fn update(&mut self, _delta_time: std::time::Duration, _: bool) {
		profiling::scope!(LOG);

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let instance_buffer = match self.instance_buffer.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let texture_cache = self.texture_cache.upgrade();

		let mut entities = Vec::new();
		let mut active_textures = Vec::new();

		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (entity, (position, orientation, basic_model, player_model)) in
			query_bundle.query(&world).iter()
		{
			let forward_xz = {
				let mut forward = *orientation.forward();
				// Flatten so the rotation is only around the y-axis, never tilting the body forwards/backwards.
				forward.y = 0.0;
				// UnitQuaternion::face_towards uses +z as forward
				forward.x *= -1.0;
				forward.z *= -1.0;
				forward
			};
			let body_rotation = UnitQuaternion::face_towards(&forward_xz, &*world::global_up());

			let instance = Instance::builder()
				.with_chunk(*position.chunk())
				.with_offset(*position.offset())
				.with_orientation(body_rotation)
				.build();

			let descriptor_id = match (basic_model, player_model) {
				(Some(presentation), None) => presentation.descriptor().clone(),
				(None, Some(presentation)) => presentation.active_model().clone(),
				_ => continue,
			};

			active_textures.push(descriptor_id.texture_id.clone());
			entities.push((entity, descriptor_id, instance));
		}

		if !active_textures.is_empty() {
			if let Some(arctex) = &texture_cache {
				if let Ok(mut cache) = arctex.lock() {
					for texture_id in active_textures.into_iter() {
						cache.mark_required(&texture_id);
					}
				}
			}
		}

		// TODO: This is EXTREMELY inefficient and causes every frame to reupload the entity instance buffer.
		if let Ok(mut buffer) = instance_buffer.write() {
			buffer.set_pending(entities);
		}; // semi-colon here drops the `buffer` guard
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptorId {
	pub model_id: asset::Id,
	pub texture_id: asset::Id,
}
