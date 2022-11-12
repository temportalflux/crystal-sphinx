use crate::{
	entity::{self, component, ArcLockEntityWorld},
	graphics::voxel::camera,
};
use engine::{math::nalgebra::Point3, utility::ValueSet, EngineSystem};
use std::sync::{Arc, RwLock, Weak};

type QueryBundle<'c> = hecs::PreparedQuery<(
	&'c component::physics::linear::Position,
	&'c component::Orientation,
	&'c component::Camera,
)>;

#[derive(Clone)]
pub struct UpdateCamera {
	systems: Arc<ValueSet>,
}

impl UpdateCamera {
	pub fn new(systems: Arc<ValueSet>) -> Self {
		Self { systems }
	}

	pub fn update(&self) {
		profiling::scope!("subsystem:update_camera");

		let arc_world = self.systems.get_arclock::<entity::World>().unwrap();
		let arc_camera = self.systems.get_arclock::<camera::Camera>().unwrap();

		let world = arc_world.read().unwrap();
		let mut query_bundle = QueryBundle::new();
		let mut result = arc_camera.read().unwrap().clone();
		for (_entity, (position, orientation, camera)) in query_bundle.query(&world).iter() {
			result.chunk_coordinate = {
				// WARN: Casting i64 to f32 will result in data loss...
				// I'll find a way to address this on another day...
				let chunk = position.chunk();
				Point3::new(chunk[0] as f32, chunk[1] as f32, chunk[2] as f32)
			};

			let isometry = camera.view().get_isometry(orientation.orientation());
			result.position = *position.offset() + isometry.translation.vector;
			result.orientation = isometry.rotation;

			result.projection = *camera.projection();

			// TODO: support multiple camera components but only 1 active at a time
			break;
		}

		*arc_camera.write().unwrap() = result;
	}
}
