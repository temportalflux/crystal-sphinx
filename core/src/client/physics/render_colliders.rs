use crate::{
	app::state::{self, ArcLockMachine},
	client::graphics::line,
	common::physics::Physics,
	graphics::voxel::camera::{self, Camera},
	CrystalSphinx,
};
use crate::{InGameSystems, SystemsContext};
use anyhow::Context;
use engine::{
	graphics::{
		buffer,
		chain::{operation::RequiresRecording, Operation},
		command, flags,
		procedure::Phase,
		resource::ColorBuffer,
		utility::NamedObject,
		Chain, Drawable, GpuOperationBuilder, Uniform,
	},
	Application, EngineSystem,
};
use multimap::MultiMap;
use nalgebra::{Affine3, Matrix4, Point3, Similarity3, Vector3, Vector4};
use rapier3d::prelude::{Collider, ColliderSet, ShapeType};
use std::{
	collections::HashMap,
	sync::{Arc, Mutex, RwLock, Weak},
	time::Duration,
};

static LOG: &'static str = "render-colliders";
static ID: &'static str = "RenderColliders";

type PendingInstances = Arc<Mutex<Option<MultiMap<ShapeType, line::Instance>>>>;

#[profiling::function]
pub fn create_collider_systems(
	ctx: &SystemsContext,
	in_game: &InGameSystems,
) -> anyhow::Result<(
	Arc<RwLock<GatherRenderableColliders>>,
	Arc<RwLock<RenderColliders>>,
)> {
	let client_ctx = ctx.client.as_ref().unwrap();

	let pending_instances = Arc::new(Mutex::new(None));

	let gather_renderable_colliders =
		GatherRenderableColliders::new(&in_game.physics, pending_instances.clone()).arclocked();

	let render_colliders = RenderColliders::new(
		&*client_ctx.chain().read().unwrap(),
		client_ctx.camera.clone(),
		pending_instances,
	)
	.context("creating render colliders operation")?
	.arclocked();

	Ok((gather_renderable_colliders, render_colliders))
}

/// Analyzes the existing physics collider-set to copy relevant data to the renderer for collision shapes.
pub struct GatherRenderableColliders {
	colliders: Arc<RwLock<ColliderSet>>,
	pending_instances: PendingInstances,
}

impl GatherRenderableColliders {
	#[profiling::function]
	pub fn new(physics: &Arc<RwLock<Physics>>, pending_instances: PendingInstances) -> Self {
		let colliders = physics.read().unwrap().colliders().clone();
		Self {
			colliders,
			pending_instances,
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
		let colliders = match self.colliders.try_read() {
			Ok(colliders) => colliders,
			_ => return,
		};

		let mut collider_instances = MultiMap::new();
		for (_handle, collider) in colliders.iter() {
			profiling::scope!("insert-collider-instance");

			// Gather colliders as render Instances, by shape type. Store in multimap by shapetype
			// where each instances has the transform data to convert the base shape model into the shape for that collider.
			// Each instance should have a basic color, unless the model type is not yet supported, in which case ERROR-PINK (r1g0b1) and show aabb bounding box.
			collider_instances.insert(collider.shape().shape_type(), self.make_instance(collider));
		}

		// Save instances to a buffer, to be written to GPU on next frame (this can later be optimized to only write updates to the buffer,
		// and also is remarkably similar to the block type instance buffer, in that the entries are grouped by model type).
		*self.pending_instances.lock().unwrap() = Some(collider_instances);
	}
}
impl GatherRenderableColliders {
	#[profiling::function]
	fn make_instance(&self, collider: &Collider) -> line::Instance {
		let use_model_color = Vector4::new(1.0, 1.0, 1.0, 1.0);
		let cuboid_base_extents = Vector3::<f32>::new(0.5, 0.5, 0.5);

		let mut scaling = Matrix4::identity();
		let mut color = Vector4::new(0.0, 0.3, 0.6, 1.0);

		match collider.shape().shape_type() {
			ShapeType::Cuboid => {
				let half_extents = collider.shape().as_cuboid().unwrap().half_extents;
				let scale = half_extents.component_div(&cuboid_base_extents);
				scaling = Matrix4::new_nonuniform_scaling(&scale);
			}
			ShapeType::Ball => {
				let radius_scaled = collider.shape().as_ball().unwrap().radius / 0.5f32;
				let scale = Vector3::new(1.0, 1.0, 1.0) * radius_scaled;
				scaling = Matrix4::new_nonuniform_scaling(&scale);
				color = Vector4::new(1.0, 1.0, 0.0, 1.0); // TODO: color, ball does not have its own model yet
			}
			_ => {
				let half_extents = collider.compute_aabb().half_extents();
				let scale = half_extents.component_div(&cuboid_base_extents);
				scaling = Matrix4::new_nonuniform_scaling(&scale);
				color = use_model_color;
			}
		}

		// First scale the model, then apply rotation, then translate it in world space.
		let transform_matrix = collider.position().to_homogeneous() * scaling;

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

	pending_instances: PendingInstances,
	models: HashMap<ShapeType, line::BufferDrawSubset>,
	vertex_buffer: Arc<buffer::Buffer>,
	index_buffer: Arc<buffer::Buffer>,
	instance_buffer: Arc<buffer::Buffer>,
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
		pending_instances: PendingInstances,
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
		let mut instances = Vec::<line::Instance>::with_capacity(500);
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

		let instance_buffer = buffer::Buffer::create_gpu(
			format!("{ID}.InstanceBuffer"),
			&chain.allocator()?,
			flags::BufferUsage::VERTEX_BUFFER,
			instances.capacity() * std::mem::size_of::<line::Instance>(),
			None,
			false,
		)
		.context(format!("create {ID} instance buffer"))?;

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
			pending_instances,
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
	) -> HashMap<ShapeType, line::BufferDrawSubset> {
		let mut models = HashMap::new();
		for shape in Self::all_shapes().into_iter() {
			let subset = Self::make_subset(vertices, indices, Self::make_segments(shape));
			models.insert(
				shape,
				line::BufferDrawSubset {
					model: subset,
					instance: line::InstanceSubset { start: 0, count: 0 },
				},
			);
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
			.map(|_| self.instance_buffer.clone())
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

		let pending = self.pending_instances.lock().unwrap().take();
		let instance_count = pending.as_ref().map(|by_type| by_type.len()).unwrap_or(0);
		if instance_count > 0 {
			let required_size = instance_count * std::mem::size_of::<line::Instance>();
			// Allocate a new buffer if the current one is not large enough to hold all the instances.
			match self.instance_buffer.expand(required_size) {
				Some(Ok(new_buffer)) => {
					// The new buffer will have the original name, and the now-old buffer will be denoted as such.
					self.instance_buffer
						.rename(&format!("{} (<{frame_image})", self.instance_buffer.name()));
					// Save the new buffer as the primary buffer. The previous outdated buffer will
					// be dropped when there are no frames using it.
					self.instance_buffer = Arc::new(new_buffer);
				}
				Some(Err(err)) => {
					*self.pending_instances.lock().unwrap() = pending;
					return Err(err)?;
				}
				None => {}
			}
		}
		if let Some(pending) = pending {
			let mut all_instances = Vec::with_capacity(instance_count);
			for (shape, mut instances) in pending.into_iter() {
				let buffer_subset = self.models.get_mut(&shape).unwrap();
				buffer_subset.instance = line::InstanceSubset {
					start: all_instances.len(),
					count: instances.len(),
				};
				all_instances.append(&mut instances);
			}

			GpuOperationBuilder::new(format!("Write({})", self.instance_buffer.name()), chain)?
				.begin()?
				.stage(&all_instances[..])?
				.copy_stage_to_buffer(&self.instance_buffer)
				.send_signal_to(chain.signal_sender())?
				.end()?;

			return Ok(RequiresRecording::CurrentFrame);
		}

		Ok(RequiresRecording::NotRequired)
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

			self.instance_buffer_per_frame[buffer_index] = self.instance_buffer.clone();
			buffer.bind_vertex_buffers(0, vec![&self.vertex_buffer], vec![0]);
			buffer.bind_vertex_buffers(1, vec![&self.instance_buffer], vec![0]);
			buffer.bind_index_buffer(&self.index_buffer, 0);

			for (shape, draw_subset) in self.models.iter() {
				if draw_subset.instance.count > 0 {
					buffer.begin_label(
						format!("{shape:?}"),
						engine::graphics::debug::LABEL_COLOR_DRAW,
					);
					buffer.draw(
						draw_subset.model.index_count,
						draw_subset.model.index_start,
						draw_subset.instance.count,
						draw_subset.instance.start,
						draw_subset.model.vertex_start,
					);
					buffer.end_label();
				}
			}
		}
		buffer.end_label();

		Ok(())
	}
}
