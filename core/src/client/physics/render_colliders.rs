use crate::{
	client::graphics::{line, SectionedBuffer},
	common::physics::{
		self,
		component::{ColliderHandle, RigidBodyIsActive},
		Physics, System,
	},
	entity,
	graphics::voxel::camera::{self, Camera},
	CrystalSphinx, InGameSystems, SystemsContext,
};
use anyhow::Context;
use engine::{
	channels::mpsc::{self, Receiver, Sender},
	graphics::{
		buffer,
		chain::{operation::RequiresRecording, Operation},
		command, flags,
		resource::ColorBuffer,
		utility::NamedObject,
		Chain, Drawable, GpuOperationBuilder, Uniform,
	},
	Application, EngineSystem,
};
use nalgebra::{Matrix4, Point3, Vector3, Vector4};
use rapier3d::prelude::ShapeType;
use std::{
	collections::HashMap,
	sync::{Arc, RwLock, Weak},
	time::Duration,
};

static LOG: &'static str = "render-colliders";
static ID: &'static str = "RenderColliders";

type InstanceBuffer = SectionedBuffer<ShapeType, rapier3d::prelude::ColliderHandle, line::Instance>;

#[profiling::function]
pub fn create_collider_systems(
	ctx: &SystemsContext,
	in_game: &InGameSystems,
) -> anyhow::Result<(
	Arc<RwLock<GatherRenderableColliders>>,
	Arc<RwLock<RenderColliders>>,
)> {
	let client_ctx = ctx.client.as_ref().unwrap();

	let instance_buffer = Arc::new({
		let allocator = {
			let arc_chain = client_ctx.chain();
			let chain = arc_chain.read().unwrap();
			chain.allocator()?
		};
		let name = format!("{ID}.InstanceBuffer");
		let result = InstanceBuffer::new(name, &allocator, 500);
		result.context(format!("create {ID} instance buffer"))?
	});

	let gather_renderable_colliders =
		GatherRenderableColliders::new(&ctx.world, &in_game.physics, instance_buffer.clone());

	let render_colliders = RenderColliders::new(
		&*client_ctx.chain().read().unwrap(),
		client_ctx.camera.clone(),
		instance_buffer,
	)
	.context("creating render colliders operation")?;

	Ok((
		gather_renderable_colliders.arclocked(),
		render_colliders.arclocked(),
	))
}

/// Component-flag indicating if an entity with a physics collider has been registered as a renderable collider.
pub struct RenderCollider {
	handle: rapier3d::prelude::ColliderHandle,
	on_drop: Sender<rapier3d::prelude::ColliderHandle>,
}
impl Drop for RenderCollider {
	fn drop(&mut self) {
		let _ = self.on_drop.send(self.handle);
	}
}
impl crate::entity::component::Component for RenderCollider {
	fn unique_id() -> &'static str {
		"crystal_sphinx::client::physics::RenderCollider"
	}

	fn display_name() -> &'static str {
		"RenderCollider"
	}
}

/// Analyzes the existing physics collider-set to copy relevant data to the renderer for collision shapes.
pub struct GatherRenderableColliders {
	world: Weak<RwLock<entity::World>>,
	physics_state: Arc<Physics>,
	instance_buffer: Arc<InstanceBuffer>,
	dropped_colliders: (
		Sender<rapier3d::prelude::ColliderHandle>,
		Receiver<rapier3d::prelude::ColliderHandle>,
	),
}

impl GatherRenderableColliders {
	#[profiling::function]
	pub fn new(
		world: &Arc<RwLock<entity::World>>,
		physics: &Arc<RwLock<System>>,
		instance_buffer: Arc<InstanceBuffer>,
	) -> Self {
		let physics_state = physics.read().unwrap().state().clone();
		let dropped_colliders = mpsc::unbounded();
		Self {
			world: Arc::downgrade(&world),
			physics_state,
			instance_buffer,
			dropped_colliders,
		}
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}
}

impl EngineSystem for GatherRenderableColliders {
	#[profiling::function]
	fn update(&mut self, _delta_time: Duration, _: bool) {
		profiling::scope!("subsystem:render-colliders::gather");

		// Non-blocking read, if something currently as write access, we skip this update.
		let state = self.physics_state.read();
		let arc_world = match self.world.upgrade() {
			Some(arc) => arc,
			_ => return,
		};
		let mut world = match arc_world.try_write() {
			Ok(world) => world,
			_ => return,
		};

		// INSERT: In order to detect when a collider is added, we need to detect when an entity in ecs has a collider handle but no RenderCollider component.
		// When thats the case, we know that we need to make a collider-render component for that entity, and add it to the instance buffer.
		self.add_render_components(&state, &mut world);

		// REMOVE: To remove old instances, the collider-render component needs a channel to send a signal to when its Dropped. That signal can be received by this system,
		// which will remove instances with a particular collider handle when the drop signal is processed.
		self.remove_dropped_entities();

		// UPDATE: To update instances, we can send a signal from the physics system saying "these objects moved this step", and use that information
		// to gather the set of all entities which have moved. From there, we can regenerate their instances and write those to the buffer.
		self.update_instances(&state, &world);
	}
}
impl GatherRenderableColliders {
	#[profiling::function]
	fn add_render_components(&self, state: &physics::State, world: &mut entity::World) {
		let mut transaction = hecs::CommandBuffer::new();
		for (entity, handle) in world
			.query::<&ColliderHandle>()
			.without::<&RenderCollider>()
			.iter()
		{
			let (shape_type, instance) = {
				let collider = state.collider(*handle.inner()).unwrap();
				let shape_type = collider.shared_shape().shape_type();
				(shape_type, self.make_instance(collider))
			};
			self.instance_buffer
				.insert(handle.inner(), instance, &shape_type);
			transaction.insert_one(
				entity,
				RenderCollider {
					handle: *handle.inner(),
					on_drop: self.dropped_colliders.0.clone(),
				},
			);
		}
		transaction.run_on(world);
	}

	#[profiling::function]
	fn remove_dropped_entities(&self) {
		while let Ok(handle) = self.dropped_colliders.1.try_recv() {
			self.instance_buffer.remove(&handle);
		}
	}

	#[profiling::function]
	fn update_instances(&self, state: &physics::State, world: &entity::World) {
		for (_entity, handle) in world
			.query::<&ColliderHandle>()
			.with::<(&RenderCollider, &RigidBodyIsActive)>()
			.iter()
		{
			let collider = state.collider(*handle.inner()).unwrap();
			let instance = self.make_instance(collider);
			self.instance_buffer.update(handle.inner(), instance);
		}
	}
}

impl GatherRenderableColliders {
	#[profiling::function]
	fn make_instance(&self, collider: &rapier3d::prelude::Collider) -> line::Instance {
		let use_model_color = Vector4::new(1.0, 1.0, 1.0, 1.0);
		let cuboid_base_extents = Vector3::<f32>::new(0.5, 0.5, 0.5);

		let mut color = Vector4::new(0.0, 0.3, 0.6, 1.0);
		let mut isometry = collider.position().clone();
		let scaling;
		match collider.shape().shape_type() {
			ShapeType::Cuboid => {
				let cuboid = collider.shape().as_cuboid().unwrap();
				isometry.translation.y += cuboid.half_extents.y;

				let scale = cuboid.half_extents.component_div(&cuboid_base_extents);
				scaling = Matrix4::new_nonuniform_scaling(&scale);
			}
			ShapeType::Ball => {
				let radius_scaled = collider.shape().as_ball().unwrap().radius / 0.5f32;
				let scale = Vector3::new(1.0, 1.0, 1.0) * radius_scaled;
				scaling = Matrix4::new_nonuniform_scaling(&scale);
				color = Vector4::new(1.0, 1.0, 0.0, 1.0);
			}
			_ => {
				let aabb = collider.compute_aabb();
				isometry.translation.vector = aabb.center().coords;
				let scale = aabb.half_extents().component_div(&cuboid_base_extents);
				scaling = Matrix4::new_nonuniform_scaling(&scale);
				color = use_model_color;
			}
		}

		// First scale the model, then apply rotation, then translate it in world space.
		let transform_matrix = isometry.to_homogeneous() * scaling;

		// TODO: Convert the physics f32 position into a chunk position (using logic similar to the f32 in Position component and the block::Point::align).
		//let offset = isometry.transform_point(&Point3::origin());

		line::Instance {
			chunk_coordinate: Vector3::new(0.0, 0.0, 0.0).into(),
			model_matrix: transform_matrix.into(),
			color: color.into(),
		}
	}
}

pub struct RenderColliders {
	drawable: Drawable,

	models: HashMap<ShapeType, line::ModelSubset>,
	vertex_buffer: Arc<buffer::Buffer>,
	index_buffer: Arc<buffer::Buffer>,
	instance_buffer: Arc<InstanceBuffer>,
	/// Reference to `instance_buffer` for each frame which has used it and may still be in flight.
	/// This is required so that when instance_buffer needs to expand, the old one isn't
	/// immediately dropped if its was used in the last `n` frames.
	instance_buffer_per_frame: Vec<Arc<buffer::Buffer>>,

	camera: Arc<RwLock<Camera>>,
	camera_uniform: Uniform,
}

impl RenderColliders {
	#[profiling::function]
	pub fn new(
		chain: &Chain,
		camera: Arc<RwLock<Camera>>,
		instance_buffer: Arc<InstanceBuffer>,
	) -> anyhow::Result<Self> {
		log::trace!(target: LOG, "Creating renderer");

		// TODO: Load shaders in async process before renderer is created
		let mut drawable = Drawable::default().with_name(ID);
		drawable.add_shader(&CrystalSphinx::get_asset_id(
			"shaders/debug/colliders/vertex",
		))?;
		drawable.add_shader(&CrystalSphinx::get_asset_id(
			"shaders/debug/colliders/fragment",
		))?;

		let mut vertices = Vec::<line::Vertex>::new();
		let mut indices = Vec::<u32>::new();
		let models = Self::construct_shape_models(&mut vertices, &mut indices);

		log::trace!(target: LOG, "Creating buffers");

		let vertex_buffer = buffer::Buffer::create_gpu(
			format!("{ID}.VertexBuffer"),
			&chain.allocator()?,
			flags::BufferUsage::VERTEX_BUFFER,
			vertices.len() * std::mem::size_of::<line::Vertex>(),
			None,
			false,
		)
		.context(format!("create {ID} vertex buffer"))?;

		GpuOperationBuilder::new(format!("Write({})", vertex_buffer.name()), chain)?
			.begin()?
			.stage(&vertices[..])?
			.copy_stage_to_buffer(&vertex_buffer)
			.send_signal_to(chain.signal_sender())?
			.end()?;

		let index_buffer = buffer::Buffer::create_gpu(
			format!("{ID}.IndexBuffer"),
			&chain.allocator()?,
			flags::BufferUsage::INDEX_BUFFER,
			indices.len() * std::mem::size_of::<u32>(),
			Some(flags::IndexType::UINT32),
			false,
		)
		.context(format!("create {ID} index buffer"))?;

		GpuOperationBuilder::new(format!("Write({})", index_buffer.name()), chain)?
			.begin()?
			.stage(&indices[..])?
			.copy_stage_to_buffer(&index_buffer)
			.send_signal_to(chain.signal_sender())?
			.end()?;

		let camera_uniform = Uniform::new::<camera::UniformData>(
			format!("{ID}.Camera"),
			&chain.logical()?,
			&chain.allocator()?,
			chain.persistent_descriptor_pool(),
			chain.view_count(),
		)?;

		log::trace!(target: LOG, "Finalizing construction");
		Ok(Self {
			drawable,
			models,
			vertex_buffer,
			index_buffer,
			instance_buffer,
			instance_buffer_per_frame: Vec::new(),
			camera_uniform,
			camera,
		})
	}

	pub fn arclocked(self) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(self))
	}

	fn construct_shape_models(
		vertices: &mut Vec<line::Vertex>,
		indices: &mut Vec<u32>,
	) -> HashMap<ShapeType, line::ModelSubset> {
		let mut models = HashMap::new();
		for shape in Self::all_shapes().into_iter() {
			let subset = Self::make_subset(vertices, indices, Self::make_segments(shape));
			models.insert(shape, subset);
		}
		models
	}

	fn all_shapes() -> Vec<ShapeType> {
		use ShapeType::*;
		vec![
			Ball,
			Cuboid,
			Capsule,
			Segment,
			Triangle,
			TriMesh,
			Polyline,
			HalfSpace,
			HeightField,
			Compound,
			ConvexPolyhedron,
			Cylinder,
			Cone,
			RoundCuboid,
			RoundTriangle,
			RoundCylinder,
			RoundCone,
			RoundConvexPolyhedron,
			Custom,
		]
	}

	fn make_segments(shape: ShapeType) -> line::Segments {
		match shape {
			ShapeType::Cuboid => {
				let r = 0.5f32;
				#[rustfmt::skip]
				let points = vec![
					// X-Axis corners
					((-r,  r,  r), (r,  r,  r)),
					((-r,  r, -r), (r,  r, -r)),
					((-r, -r,  r), (r, -r,  r)),
					((-r, -r, -r), (r, -r, -r)),
					// Y-Axis corners
					(( r, -r,  r), ( r, r,  r)),
					(( r, -r, -r), ( r, r, -r)),
					((-r, -r,  r), (-r, r,  r)),
					((-r, -r, -r), (-r, r, -r)),
					// Z-Axis corners
					(( r,  r, -r), ( r,  r, r)),
					(( r, -r, -r), ( r, -r, r)),
					((-r,  r, -r), (-r,  r, r)),
					((-r, -r, -r), (-r, -r, r)),
				];
				let mut segments = line::Segments::new();
				for line in points.into_iter() {
					segments.push(line.into());
				}
				segments
			}
			ShapeType::Ball => {
				// Icosphere!
				// https://twitter.com/FreyaHolmer/status/1321205757669498880
				// https://twitter.com/FreyaHolmer/status/1321178630895132672
				// http://blog.andreaskahler.com/2009/06/creating-icosphere-mesh-in-code.html
				let t = (1.0 + 5.0f32.sqrt()) / 2.0;
				#[rustfmt::skip]
				let major_points: Vec<Point3<f32>> = vec![
					Point3::new(-1.0,    t,  0.0),
					Point3::new( 1.0,    t,  0.0),
					Point3::new(-1.0,   -t,  0.0),
					Point3::new( 1.0,   -t,  0.0),
					Point3::new( 0.0, -1.0,    t),
					Point3::new( 0.0,  1.0,    t),
					Point3::new( 0.0, -1.0,   -t),
					Point3::new( 0.0,  1.0,   -t),
					Point3::new(   t,  0.0, -1.0),
					Point3::new(   t,  0.0,  1.0),
					Point3::new(  -t,  0.0, -1.0),
					Point3::new(  -t,  0.0,  1.0),
				];
				// Indices into major_points for each of the major faces
				let major_face_indices = vec![
					// 5 faces around point 0
					[0, 11, 5],
					[0, 5, 1],
					[0, 1, 7],
					[0, 7, 10],
					[0, 10, 11],
					// 5 adjacent faces
					[1, 5, 9],
					[5, 11, 4],
					[11, 10, 2],
					[10, 7, 6],
					[7, 1, 8],
					// 5 faces around point 3
					[3, 9, 4],
					[3, 4, 2],
					[3, 2, 6],
					[3, 6, 8],
					[3, 8, 9],
					// 5 adjacent faces
					[4, 9, 5],
					[2, 4, 11],
					[6, 2, 10],
					[8, 6, 7],
					[9, 8, 1],
				];
				// Indices into major_points for each edge along major faces.
				let major_edges = vec![
					// Tent pitch around "upper" pentagon
					(1, 0),
					(1, 5),
					(1, 7),
					(1, 8),
					(1, 9),
					// "upper" pentagon
					(0, 7),
					(7, 8),
					(8, 9),
					(9, 5),
					(5, 0),
					// Middle Ring
					(0, 11),
					(11, 5),
					(5, 4),
					(4, 9),
					(9, 3),
					(3, 8),
					(8, 6),
					(6, 7),
					(7, 10),
					(10, 0),
					// "lower" pentagon
					(6, 3),
					(3, 4),
					(4, 11),
					(11, 10),
					(10, 6),
					// Ten pitch around "lower" pentagon
					(2, 3),
					(2, 4),
					(2, 6),
					(2, 10),
					(2, 11),
				];

				// All of the major edges split in half, plus 3 edges per major face (one subdivision).
				let total_edge_count = major_edges.len() * 2 + major_face_indices.len() * 3;
				let mut edges = Vec::with_capacity(total_edge_count);
				for [i1, i2, i3] in major_face_indices.iter() {
					let p1 = major_points[*i1].coords;
					let p2 = major_points[*i2].coords;
					let p3 = major_points[*i3].coords;
					let p12 = (p1 * 0.5) + (p2 * 0.5);
					let p23 = (p2 * 0.5) + (p3 * 0.5);
					let p31 = (p3 * 0.5) + (p1 * 0.5);
					// Subdivide each edge
					edges.push((p1, p12));
					edges.push((p12, p2));
					edges.push((p2, p23));
					edges.push((p23, p3));
					edges.push((p3, p31));
					edges.push((p31, p1));
					// Add face subdivision
					edges.push((p12, p23));
					edges.push((p23, p31));
					edges.push((p31, p12));
				}

				let center = Vector3::<f32>::new(0.5, 0.5, 0.5);
				let mut segments = line::Segments::new();
				for (p1, p2) in edges.iter() {
					let p1 = p1.normalize() * 0.5 + center;
					let p2 = p2.normalize() * 0.5 + center;
					segments.push((p1, p2).into());
				}
				segments
			}
			_ => {
				Self::make_segments(ShapeType::Cuboid).with_color(Vector4::new(1.0, 0.0, 1.0, 1.0))
			}
		}
	}

	fn make_subset(
		vertices: &mut Vec<line::Vertex>,
		indices: &mut Vec<u32>,
		segments: line::Segments,
	) -> line::ModelSubset {
		let (mut new_vertices, mut new_indices) = segments.into_vertices();
		let subset = line::ModelSubset {
			index_start: indices.len(),
			index_count: new_indices.len(),
			vertex_start: vertices.len(),
		};
		vertices.append(&mut new_vertices);
		indices.append(&mut new_indices);
		subset
	}
}

impl Drop for RenderColliders {
	fn drop(&mut self) {
		log::info!(target: LOG, "Dropping from subpass.",);
	}
}

impl Operation for RenderColliders {
	#[profiling::function]
	fn initialize(&mut self, chain: &Chain) -> anyhow::Result<()> {
		self.drawable.create_shaders(&chain.logical()?)?;
		self.camera_uniform
			.write_descriptor_sets(&*chain.logical()?);
		Ok(())
	}

	#[profiling::function]
	fn construct(&mut self, chain: &Chain, subpass_index: usize) -> anyhow::Result<()> {
		use engine::graphics::pipeline::{state::*, Pipeline};

		self.instance_buffer_per_frame = (0..chain.view_count())
			.map(|_| self.instance_buffer.buffer())
			.collect();

		let sample_count = {
			let arc = chain.resources().get::<ColorBuffer>()?;
			let color_buffer = arc.read().unwrap();
			color_buffer.sample_count()
		};

		self.drawable.create_pipeline(
			&chain.logical()?,
			vec![self.camera_uniform.layout()],
			Pipeline::builder()
				.with_vertex_layout(
					vertex::Layout::default()
						.with_object::<line::Vertex>(0, flags::VertexInputRate::VERTEX)
						.with_object::<line::Instance>(1, flags::VertexInputRate::INSTANCE),
				)
				.set_viewport_state(Viewport::from(*chain.extent()))
				.with_topology(
					Topology::default().with_primitive(flags::PrimitiveTopology::LINE_LIST),
				)
				.with_multisampling(Multisampling::default().with_sample_count(sample_count))
				.set_color_blending(
					color_blend::ColorBlend::default()
						.add_attachment(color_blend::Attachment::default()),
				)
				.with_depth_stencil(
					DepthStencil::default()
						.with_depth_test()
						.with_depth_compare_op(flags::CompareOp::LESS),
				),
			chain.render_pass(),
			subpass_index,
		)?;
		Ok(())
	}

	fn deconstruct(&mut self, _chain: &Chain) -> anyhow::Result<()> {
		self.drawable.destroy_pipeline()?;
		Ok(())
	}

	#[profiling::function]
	fn prepare_for_submit(
		&mut self,
		chain: &Chain,
		frame_image: usize,
	) -> anyhow::Result<RequiresRecording> {
		let data = self
			.camera
			.read()
			.unwrap()
			.as_uniform_data(&chain.resolution());
		self.camera_uniform.write_data(frame_image, &data)?;

		match self
			.instance_buffer
			.submit_changes(chain, chain.signal_sender())
		{
			Ok(true) => Ok(RequiresRecording::AllFrames),
			Ok(false) => Ok(RequiresRecording::NotRequired),
			Err(err) => {
				log::error!(
					target: LOG,
					"Failed to submit instance buffer changes: {err:?}"
				);
				Ok(RequiresRecording::NotRequired)
			}
		}
	}

	#[profiling::function]
	fn record(&mut self, buffer: &mut command::Buffer, buffer_index: usize) -> anyhow::Result<()> {
		buffer.begin_label("Draw:Debug", engine::graphics::debug::LABEL_COLOR_DRAW);
		{
			// TODO: Add a mode/pipeline for rendering opaque faces on each model, instead of only the wireframe.

			self.drawable.bind_pipeline(buffer);
			self.drawable.bind_descriptors(
				buffer,
				vec![&self.camera_uniform.get_set(buffer_index).unwrap()],
			);

			let (instance_buffer, instance_sections) = self.instance_buffer.submitted();
			buffer.bind_vertex_buffers(0, vec![&self.vertex_buffer], vec![0]);
			buffer.bind_vertex_buffers(1, vec![&instance_buffer], vec![0]);
			buffer.bind_index_buffer(&self.index_buffer, 0);

			for (shape, instance_range) in instance_sections.into_iter() {
				let model = self.models.get(&shape).unwrap();
				let instance_count = instance_range.end - instance_range.start;
				if instance_count > 0 {
					buffer.begin_label(
						format!("{shape:?}"),
						engine::graphics::debug::LABEL_COLOR_DRAW,
					);
					buffer.draw(
						model.index_count,
						model.index_start,
						instance_count,
						instance_range.start,
						model.vertex_start,
					);
					buffer.end_label();
				}
			}

			// Ensure the instance buffer is not dropped (due to possible reallocation) if its being used by a frame.
			self.instance_buffer_per_frame[buffer_index] = instance_buffer;
		}
		buffer.end_label();

		Ok(())
	}
}
