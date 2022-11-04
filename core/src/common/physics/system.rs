use super::component;
use crate::entity;
use engine::{channels::mpsc, EngineSystem};
use nalgebra::vector;
use rand::Rng;
use rapier3d::prelude::{ChannelEventCollector, RigidBodyType, SharedShape};
use std::{
	sync::{Arc, RwLock, Weak},
	time::Duration,
};

mod object_id;
pub use object_id::*;
mod phase_add_objects;
use phase_add_objects::*;
mod phase_copy_comp_to_phys;
use phase_copy_comp_to_phys::*;
mod phase_copy_phys_to_comp;
use phase_copy_phys_to_comp::*;
mod phase_simulate;
use phase_simulate::*;

pub struct System {
	world: Weak<RwLock<entity::World>>,
	state: Arc<super::Physics>,
	update_objects: AddPhysicsObjects,
	simulation: StepSimulation,
	phys_to_world: CopyPhysicsToComponents,
}

impl System {
	pub fn new(world: &Arc<RwLock<entity::World>>) -> Self {
		Self::init_demo(&mut *world.write().unwrap());
		let (event_handler, phys_to_world) = {
			let (send_collisions, recv_collisions) = mpsc::unbounded();
			let (send_contact_forces, recv_contact_forces) = mpsc::unbounded();
			let event_handler = ChannelEventCollector::new(send_collisions, send_contact_forces);
			let phys_to_world = CopyPhysicsToComponents {
				recv_collisions,
				recv_contact_forces,
			};
			(event_handler, phys_to_world)
		};
		Self {
			world: Arc::downgrade(world),
			state: Arc::new(super::Physics::default()),
			update_objects: AddPhysicsObjects::new(),
			simulation: StepSimulation::new(event_handler),
			phys_to_world,
		}
	}

	fn init_demo(world: &mut entity::World) {
		let mut transaction = hecs::CommandBuffer::default();

		transaction.spawn(
			hecs::EntityBuilder::default()
				.add(component::Collider::new(SharedShape::cuboid(8.0, 0.5, 8.0)))
				.add(component::Position::default().with_point(vector![8.0, 6.0, 8.0].into()))
				.build(),
		);

		let mut rng = rand::thread_rng();
		let balls = vec![
			(vector![5.0, (rng.gen::<f32>() * 20.0 + 10.0), 8.0], 0.1),
			(vector![11.0, (rng.gen::<f32>() * 20.0 + 10.0), 8.0], 0.3),
			(vector![8.0, (rng.gen::<f32>() * 20.0 + 10.0), 8.0], 0.5),
			(vector![8.0, (rng.gen::<f32>() * 20.0 + 10.0), 5.0], 0.7),
			(vector![8.0, (rng.gen::<f32>() * 20.0 + 10.0), 11.0], 0.9),
		];
		for (position, bounciness) in balls.into_iter() {
			transaction.spawn(
				hecs::EntityBuilder::default()
					.add(component::Position::default().with_point(position.into()))
					.add(
						component::Collider::new(SharedShape::ball(0.5))
							.with_restitution(bounciness),
					)
					.add(component::RigidBody::new(RigidBodyType::Dynamic))
					.build(),
			);
		}

		//let cyl_col = ColliderBuilder::cylinder(0.85, 0.4)
		//	.translation(vector![5.0, 10.0, 5.0])
		//	.build();
		//colliders.insert(cyl_col);

		transaction.run_on(world);
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	pub fn state(&self) -> &Arc<super::Physics> {
		&self.state
	}
}

impl EngineSystem for System {
	fn update(&mut self, delta_time: Duration, _: bool) {
		profiling::scope!("subsystem:physics");

		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			None => return,
		};
		let mut world = {
			profiling::scope!("lock-world");
			arc_world.write().unwrap()
		};

		let mut state = self.state.write();
		self.update_objects.execute(&mut state, &mut world);
		CopyComponentsToPhysics::execute(&mut state, &mut world);
		self.simulation.execute(&mut state, delta_time);
		self.phys_to_world.execute(&mut state, &mut world);
	}
}
