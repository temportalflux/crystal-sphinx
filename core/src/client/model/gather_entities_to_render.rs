use crate::{
	entity::{self, component, ArcLockEntityWorld},
	graphics::voxel::camera,
};
use engine::{
	asset,
	math::nalgebra::{Point3, UnitQuaternion},
	Engine, EngineSystem,
};
use std::sync::{Arc, RwLock, Weak};

use super::blender;

static LOG: &'static str = "subsystem:GatherEntitiesToRender";

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::physics::linear::Position,
	&'c component::Orientation,
	&'c blender::Component,
)>;

pub struct GatherEntitiesToRender {
	world: Weak<RwLock<entity::World>>,
}

impl GatherEntitiesToRender {
	pub fn create(world: Weak<RwLock<entity::World>>) -> Arc<RwLock<Self>> {
		let arclocked = Arc::new(RwLock::new(Self { world }));

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
		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		for (entity, (position, orientation, presentation)) in query_bundle.query(&world).iter() {
			entity;
			let instance = Instance {
				chunk: *position.chunk(),
				offset: *position.offset(),
				orientation: *orientation.orientation(),
			};
			let descriptor_id = DescriptorId {
				model_id: presentation.model().clone(),
				texture_id: presentation.texture().clone(),
			};
			// TODO: Insert the entity->instance pairing and entity->descriptor pairing into the relevant caches for rendering
		}
		// TODO: Remove any entities from the caches that no longer exist
	}
}

struct Instance {
	chunk: Point3<i64>,
	offset: Point3<f32>,
	orientation: UnitQuaternion<f32>,
}
struct DescriptorId {
	model_id: asset::Id,
	texture_id: asset::Id,
}
