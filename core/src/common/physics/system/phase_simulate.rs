use crate::common::physics::State;
use rapier3d::prelude::ChannelEventCollector;
use std::time::Duration;

/// Runs the update step in the physics simulation.
pub(in crate::common::physics) struct StepSimulation {
	pub duration_since_update: Duration,
	pub event_handler: ChannelEventCollector,
}
impl StepSimulation {
	pub fn new(event_handler: ChannelEventCollector) -> Self {
		Self {
			duration_since_update: Duration::from_millis(0),
			event_handler,
		}
	}

	pub fn execute(&mut self, ctx: &mut State, delta_time: Duration) {
		profiling::scope!("step-simulation");
		// If enough time has passed to run the next fixed-timestep-update, do so.
		let integration_dt = Duration::from_secs_f32(ctx.integration_parameters.dt);
		// Collect total delta_time since the last update
		self.duration_since_update += delta_time;
		while self.duration_since_update >= integration_dt {
			self.duration_since_update -= integration_dt;
			self.fixed_update(ctx);
		}
	}

	#[profiling::function]
	fn fixed_update(&self, ctx: &mut State) {
		let physics_hooks = ();
		ctx.physics_pipeline.step(
			&ctx.gravity,
			&ctx.integration_parameters,
			&mut ctx.islands,
			&mut ctx.broad_phase,
			&mut ctx.narrow_phase,
			&mut ctx.rigid_bodies,
			&mut ctx.colliders,
			&mut ctx.impulse_joints,
			&mut ctx.multibody_joints,
			&mut ctx.ccd_solver,
			&physics_hooks,
			&self.event_handler,
		);
		ctx.query_pipeline
			.update(&ctx.islands, &ctx.rigid_bodies, &ctx.colliders);
	}
}
