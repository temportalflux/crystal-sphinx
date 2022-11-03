use crate::common::physics::State;
use std::time::Duration;

/// Runs the update step in the physics simulation.
pub(in crate::common::physics) struct StepSimulation {
	pub(in crate::common::physics) duration_since_update: Duration,
}
impl StepSimulation {
	pub fn execute(&mut self, ctx: &mut State, delta_time: Duration) {
		profiling::scope!("step-simulation");
		// Collect total delta_time since the last update
		self.duration_since_update += delta_time;
		// If enough time has passed to run the next fixed-timestep-update, do so.
		let integration_dt = Duration::from_secs_f32(ctx.integration_parameters.dt);
		while self.duration_since_update >= integration_dt {
			self.duration_since_update -= integration_dt;
			self.fixed_update(ctx);
		}
	}

	#[profiling::function]
	fn fixed_update(&self, ctx: &mut State) {
		let physics_hooks = ();
		let event_handler = ();
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
			&event_handler,
		);
		ctx.query_pipeline
			.update(&ctx.islands, &ctx.rigid_bodies, &ctx.colliders);
	}
}
