use crate::{client::graphics::line, common::world::chunk, graphics::voxel::camera, CrystalSphinx};
use anyhow::Result;
use engine::{
	asset,
	graphics::{
		self, buffer,
		chain::{operation::RequiresRecording, Chain, Operation},
		command, flags,
		resource::ColorBuffer,
		utility::NamedObject,
		Drawable, GpuOperationBuilder, Uniform,
	},
	input,
	math::nalgebra::{Matrix4, Point3, Translation3, Vector4},
	world, Application, Engine, EngineSystem,
};
use enumset::{EnumSet, EnumSetType};
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

static ID: &'static str = "render-chunk-boundary";

struct BoundaryControl {
	kind: Type,
	weak_action: input::action::WeakLockState,
}
#[derive(Debug, EnumSetType, Hash)]
enum Type {
	None,
	Column,
	Cube,
	FaceGrid,
}
impl BoundaryControl {
	#[profiling::function]
	fn create(kind: Type, weak_action: input::action::WeakLockState) -> Arc<RwLock<Self>> {
		log::trace!(target: ID, "Creating action listener");
		let control = Arc::new(RwLock::new(Self { kind, weak_action }));
		if let Ok(mut engine) = Engine::get().write() {
			engine.add_weak_system(Arc::downgrade(&control));
		}
		control
	}
}
impl Type {
	fn rendered_kinds(&self) -> Vec<Self> {
		match self {
			Self::None => vec![],
			Self::Column => vec![Self::Column],
			Self::Cube => vec![Self::Column, Self::Cube],
			Self::FaceGrid => vec![Self::Column, Self::Cube, Self::FaceGrid],
		}
	}

	fn line_segments(&self) -> line::Segments {
		let w_x = chunk::SIZE.x;
		let h_y = chunk::SIZE.y;
		let l_z = chunk::SIZE.z;
		let bound_h = [0.0, h_y];
		let mut segments = line::Segments::new();
		match self {
			Self::None => {}
			Self::Column => {
				let line_height = /*16 chunks*/ 16.0 * chunk::SIZE[1];
				let h1 = line_height / 2.0 * -1.0;
				let h2 = line_height / 2.0;
				segments.push(((0.0, h1, 0.0), (0.0, h2, 0.0)).into());
				segments.push(((w_x, h1, 0.0), (w_x, h2, 0.0)).into());
				segments.push(((w_x, h1, l_z), (w_x, h2, l_z)).into());
				segments.push(((0.0, h1, l_z), (0.0, h2, l_z)).into());
			}
			Self::Cube => {
				for &y in bound_h.iter() {
					segments.push(((0.0, y, 0.0), (w_x, y, 0.0)).into());
					segments.push(((w_x, y, 0.0), (w_x, y, l_z)).into());
					segments.push(((0.0, y, 0.0), (0.0, y, l_z)).into());
					segments.push(((0.0, y, l_z), (w_x, y, l_z)).into());
				}
			}
			Self::FaceGrid => {
				let bound_w = [0.0, w_x];
				let bound_l = [0.0, l_z];
				let inner_w = (1..chunk::SIZE_I[0]).into_iter().map(|i| i as f32);
				let inner_h = (1..chunk::SIZE_I[1]).into_iter().map(|i| i as f32);
				let inner_l = (1..chunk::SIZE_I[2]).into_iter().map(|i| i as f32);

				// Y-Faces (Up/Down)
				for &y in bound_h.iter() {
					for x in inner_w.clone() {
						segments.push(((x, y, 0.0), (x, y, l_z)).into());
					}
					for z in inner_l.clone() {
						segments.push(((0.0, y, z), (w_x, y, z)).into());
					}
				}
				// Z-Faces (Back/Front)
				for &z in bound_l.iter() {
					for x in inner_w.clone() {
						segments.push(((x, 0.0, z), (x, h_y, z)).into());
					}
					for y in inner_h.clone() {
						segments.push(((0.0, y, z), (w_x, y, z)).into());
					}
				}
				// X-Faces (Left/Right)
				for &x in bound_w.iter() {
					for y in inner_h.clone() {
						segments.push(((x, y, 0.0), (x, y, l_z)).into());
					}
					for z in inner_l.clone() {
						segments.push(((x, 0.0, z), (x, h_y, z)).into());
					}
				}
			}
		}
		segments
	}
}

impl EngineSystem for BoundaryControl {
	fn update(&mut self, _delta_time: std::time::Duration, _has_focus: bool) {
		if let Some(arc_state) = self.weak_action.upgrade() {
			if let Ok(state) = arc_state.read() {
				if state.on_button_pressed() {
					self.kind = match self.kind {
						Type::None => Type::Column,
						Type::Column => Type::Cube,
						Type::Cube => Type::FaceGrid,
						Type::FaceGrid => Type::None,
					};
				}
			}
		}
	}
}

#[derive(Debug, Hash, Eq, PartialEq)]
enum RenderType {
	ChunkBoundary(Type),
	OrientationGadget,
}
impl From<Type> for RenderType {
	fn from(kind: Type) -> Self {
		Self::ChunkBoundary(kind)
	}
}
impl RenderType {
	fn all() -> Vec<Self> {
		let mut all = Vec::new();
		all.push(Self::OrientationGadget);
		for boundary in EnumSet::<Type>::all().into_iter() {
			all.push(Self::ChunkBoundary(boundary));
		}
		all
	}

	fn line_segments(&self) -> line::Segments {
		match self {
			Self::ChunkBoundary(boundary) => boundary.line_segments(),
			Self::OrientationGadget => {
				let red = Vector4::new(1.0, 0.0, 0.0, 1.0);
				let green = Vector4::new(0.0, 1.0, 0.0, 1.0);
				let blue = Vector4::new(0.0, 0.1, 1.0, 1.0);
				let flags = Vector4::new(1.0, 0.0, 0.0, 0.0);
				let start = Point3::<f32>::new(0.0, 0.0, 0.0);
				let axis_length = 0.01f32;

				let mut segments = line::Segments::new();
				segments.push(line::Segment {
					pos1: start,
					pos2: start + (*world::global_right() * axis_length),
					color: red,
					flags,
				});
				segments.push(line::Segment {
					pos1: start,
					pos2: start + (*world::global_up() * axis_length),
					color: green,
					flags,
				});
				segments.push(line::Segment {
					pos1: start,
					pos2: start + (*world::global_forward() * axis_length),
					color: blue,
					flags,
				});
				segments
			}
		}
	}

	fn instance(&self) -> line::Instance {
		match self {
			Self::ChunkBoundary(Type::None) => line::Instance {
				chunk_coordinate: Point3::origin().into(),
				model_matrix: Matrix4::identity().into(),
				color: Vector4::new(1.0, 1.0, 1.0, 1.0).into(),
			},
			Self::ChunkBoundary(Type::Column) => line::Instance {
				chunk_coordinate: Point3::origin().into(),
				model_matrix: Matrix4::identity().into(),
				color: Vector4::new(0.0, 1.0, 0.0, 1.0).into(),
			},
			Self::ChunkBoundary(Type::Cube) => line::Instance {
				chunk_coordinate: Point3::origin().into(),
				model_matrix: Matrix4::identity().into(),
				color: Vector4::new(1.0, 0.0, 0.0, 1.0).into(),
			},
			Self::ChunkBoundary(Type::FaceGrid) => line::Instance {
				chunk_coordinate: Point3::origin().into(),
				model_matrix: Matrix4::identity().into(),
				color: Vector4::new(0.0, 0.0, 1.0, 1.0).into(),
			},
			Self::OrientationGadget => {
				let transform = Translation3::<f32>::new(0.0, 0.0, -0.25);
				line::Instance {
					chunk_coordinate: Point3::origin().into(),
					model_matrix: transform.to_homogeneous().into(),
					color: Vector4::new(1.0, 1.0, 1.0, 1.0).into(),
				}
			}
		}
	}
}

pub type ArcLockRender = Arc<RwLock<Render>>;
pub struct Render {
	drawable: Drawable,

	control: Arc<RwLock<BoundaryControl>>,
	recorded_kind: Vec<Type>,
	type_settings: HashMap<RenderType, line::BufferDrawSubset>,
	vertex_buffer: Arc<buffer::Buffer>,
	index_buffer: Arc<buffer::Buffer>,
	instance_buffer: Arc<buffer::Buffer>,

	camera: Arc<RwLock<camera::Camera>>,
	camera_uniform: Uniform,
}

impl Render {
	fn subpass_id() -> asset::Id {
		CrystalSphinx::get_asset_id("render_pass/subpass/debug")
	}

	#[profiling::function]
	pub fn new(
		chain: &Chain,
		camera: Arc<RwLock<camera::Camera>>,
		weak_action: input::action::WeakLockState,
	) -> Result<Self> {
		log::trace!(target: ID, "Creating renderer");

		// TODO: Load shaders in async process before renderer is created
		let mut drawable = Drawable::default().with_name(ID);
		drawable.add_shader(&CrystalSphinx::get_asset_id(
			"shaders/debug/chunk_boundary/vertex",
		))?;
		drawable.add_shader(&CrystalSphinx::get_asset_id(
			"shaders/debug/chunk_boundary/fragment",
		))?;

		let mut type_settings = HashMap::new();
		let mut vertices = Vec::new();
		let mut indices = Vec::new();
		let mut instances = Vec::new();
		for render_type in RenderType::all().into_iter() {
			let segments = render_type.line_segments();
			let instance = render_type.instance();

			let (mut kind_vertices, mut kind_indices) = segments.into_vertices();

			type_settings.insert(
				render_type,
				line::BufferDrawSubset {
					model: line::ModelSubset {
						index_start: indices.len(),
						index_count: kind_indices.len(),
						vertex_start: vertices.len(),
					},
					instance: line::InstanceSubset {
						start: instances.len(),
						count: 1,
					},
				},
			);

			vertices.append(&mut kind_vertices);
			indices.append(&mut kind_indices);
			instances.push(instance);
		}

		log::trace!(target: ID, "Creating buffers");

		let vertex_buffer = buffer::Buffer::create_gpu(
			format!("ChunkBoundary.VertexBuffer"),
			&chain.allocator()?,
			flags::BufferUsage::VERTEX_BUFFER,
			vertices.len() * std::mem::size_of::<line::Vertex>(),
			None,
			false,
		)?;

		GpuOperationBuilder::new(format!("Write({})", vertex_buffer.name()), chain)?
			.begin()?
			.stage(&vertices[..])?
			.copy_stage_to_buffer(&vertex_buffer)
			.send_signal_to(chain.signal_sender())?
			.end()?;

		let index_buffer = buffer::Buffer::create_gpu(
			format!("ChunkBoundary.IndexBuffer"),
			&chain.allocator()?,
			flags::BufferUsage::INDEX_BUFFER,
			indices.len() * std::mem::size_of::<u32>(),
			Some(flags::IndexType::UINT32),
			false,
		)?;

		GpuOperationBuilder::new(format!("Write({})", index_buffer.name()), chain)?
			.begin()?
			.stage(&indices[..])?
			.copy_stage_to_buffer(&index_buffer)
			.send_signal_to(chain.signal_sender())?
			.end()?;

		let instance_buffer = buffer::Buffer::create_gpu(
			format!("ChunkBoundary.InstanceBuffer"),
			&chain.allocator()?,
			flags::BufferUsage::VERTEX_BUFFER,
			instances.len() * std::mem::size_of::<line::Instance>(),
			None,
			false,
		)?;

		GpuOperationBuilder::new(format!("Write({})", instance_buffer.name()), chain)?
			.begin()?
			.stage(&instances[..])?
			.copy_stage_to_buffer(&instance_buffer)
			.send_signal_to(chain.signal_sender())?
			.end()?;

		let camera_uniform = Uniform::new::<camera::UniformData>(
			"ChunkBoundary.Camera",
			&chain.logical()?,
			&chain.allocator()?,
			chain.persistent_descriptor_pool(),
			chain.view_count(),
		)?;

		let control = BoundaryControl::create(Type::None, weak_action);

		log::trace!(target: ID, "Finalizing construction");
		Ok(Self {
			drawable,
			control,
			recorded_kind: Vec::new(),
			type_settings,
			vertex_buffer,
			index_buffer,
			instance_buffer,
			camera_uniform,
			camera,
		})
	}

	pub fn arclocked(self) -> ArcLockRender {
		Arc::new(RwLock::new(self))
	}
}

impl Drop for Render {
	fn drop(&mut self) {
		log::info!(target: ID, "Dropping from subpass({}).", Self::subpass_id());
	}
}

impl Operation for Render {
	#[profiling::function]
	fn initialize(&mut self, chain: &Chain) -> anyhow::Result<()> {
		self.drawable.create_shaders(&chain.logical()?)?;
		self.camera_uniform
			.write_descriptor_sets(&*chain.logical()?);

		let control_kind = self.control.read().unwrap().kind;
		self.recorded_kind = vec![control_kind; chain.view_count()];
		Ok(())
	}

	#[profiling::function]
	fn construct(&mut self, chain: &Chain, subpass_index: usize) -> anyhow::Result<()> {
		use graphics::pipeline::{state::*, Pipeline};

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

		let control_kind = self.control.read().unwrap().kind;
		if self.recorded_kind[frame_image] != control_kind {
			self.recorded_kind[frame_image] = control_kind;
			Ok(RequiresRecording::CurrentFrame)
		} else {
			Ok(RequiresRecording::NotRequired)
		}
	}

	#[profiling::function]
	fn record(&mut self, buffer: &mut command::Buffer, buffer_index: usize) -> anyhow::Result<()> {
		use graphics::debug;

		buffer.begin_label("Draw:Debug", debug::LABEL_COLOR_DRAW);
		{
			self.drawable.bind_pipeline(buffer);
			self.drawable.bind_descriptors(
				buffer,
				vec![&self.camera_uniform.get_set(buffer_index).unwrap()],
			);

			buffer.bind_vertex_buffers(0, vec![&self.vertex_buffer], vec![0]);
			buffer.bind_vertex_buffers(1, vec![&self.instance_buffer], vec![0]);
			buffer.bind_index_buffer(&self.index_buffer, 0);

			let mut render_types = self.recorded_kind[buffer_index]
				.rendered_kinds()
				.into_iter()
				.map(RenderType::from)
				.collect::<Vec<_>>();
			render_types.push(RenderType::OrientationGadget);
			for render_type in render_types.into_iter() {
				if let Some(settings) = self.type_settings.get(&render_type) {
					buffer.draw(
						settings.model.index_count,
						settings.model.index_start,
						settings.instance.count,
						settings.instance.start,
						settings.model.vertex_start,
					);
				}
			}
		}
		buffer.end_label();

		Ok(())
	}
}
