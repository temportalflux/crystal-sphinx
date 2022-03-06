use crate::{
	app::state::{self, ArcLockMachine},
	common::world::chunk,
	graphics::voxel::camera,
	CrystalSphinx,
};
use anyhow::Result;
use engine::{
	asset,
	graphics::{
		self, buffer, command, flags, pipeline, structs,
		types::{Vec3, Vec4},
		utility::NamedObject,
		vertex_object, ArcRenderChain, Drawable, GpuOperationBuilder, RenderChain,
		RenderChainElement, Uniform,
	},
	input,
	math::nalgebra::{Point3, Vector2, Vector4},
	Application, Engine, EngineSystem,
};
use enumset::{EnumSet, EnumSetType};
use std::{
	collections::HashMap,
	sync::{Arc, RwLock},
};

static ID: &'static str = "render-chunk-boundary";

struct LineSegment {
	pos1: Point3<f32>,
	pos2: Point3<f32>,
	color: Vector4<f32>,
}
impl From<((f32, f32, f32), (f32, f32, f32), Vector4<f32>)> for LineSegment {
	fn from(params: ((f32, f32, f32), (f32, f32, f32), Vector4<f32>)) -> Self {
		Self {
			pos1: Point3::new(params.0 .0, params.0 .1, params.0 .2),
			pos2: Point3::new(params.1 .0, params.1 .1, params.1 .2),
			color: params.2,
		}
	}
}

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

	fn line_segments(&self) -> Vec<LineSegment> {
		let w_x = chunk::SIZE.x;
		let h_y = chunk::SIZE.y;
		let l_z = chunk::SIZE.z;
		let bound_h = [0.0, h_y];
		let mut segments = Vec::new();
		match self {
			Self::None => {}
			Self::Column => {
				let line_height = /*16 chunks*/ 16.0 * chunk::SIZE[1];
				let h1 = line_height / 2.0 * -1.0;
				let h2 = line_height / 2.0;
				let color = Vector4::new(0.0, 1.0, 0.0, 1.0);
				segments.push(((0.0, h1, 0.0), (0.0, h2, 0.0), color).into());
				segments.push(((w_x, h1, 0.0), (w_x, h2, 0.0), color).into());
				segments.push(((w_x, h1, l_z), (w_x, h2, l_z), color).into());
				segments.push(((0.0, h1, l_z), (0.0, h2, l_z), color).into());
			}
			Self::Cube => {
				let color = Vector4::new(1.0, 0.0, 0.0, 1.0);
				for &y in bound_h.iter() {
					segments.push(((0.0, y, 0.0), (w_x, y, 0.0), color).into());
					segments.push(((w_x, y, 0.0), (w_x, y, l_z), color).into());
					segments.push(((0.0, y, 0.0), (0.0, y, l_z), color).into());
					segments.push(((0.0, y, l_z), (w_x, y, l_z), color).into());
				}
			}
			Self::FaceGrid => {
				let color = Vector4::new(0.0, 0.0, 1.0, 1.0);
				let bound_w = [0.0, w_x];
				let bound_l = [0.0, l_z];
				let inner_w = (1..chunk::SIZE_I[0]).into_iter().map(|i| i as f32);
				let inner_h = (1..chunk::SIZE_I[1]).into_iter().map(|i| i as f32);
				let inner_l = (1..chunk::SIZE_I[2]).into_iter().map(|i| i as f32);

				// Y-Faces (Up/Down)
				for &y in bound_h.iter() {
					for x in inner_w.clone() {
						segments.push(((x, y, 0.0), (x, y, l_z), color).into());
					}
					for z in inner_l.clone() {
						segments.push(((0.0, y, z), (w_x, y, z), color).into());
					}
				}
				// Z-Faces (Back/Front)
				for &z in bound_l.iter() {
					for x in inner_w.clone() {
						segments.push(((x, 0.0, z), (x, h_y, z), color).into());
					}
					for y in inner_h.clone() {
						segments.push(((0.0, y, z), (w_x, y, z), color).into());
					}
				}
				// X-Faces (Left/Right)
				for &x in bound_w.iter() {
					for y in inner_h.clone() {
						segments.push(((x, y, 0.0), (x, y, l_z), color).into());
					}
					for z in inner_l.clone() {
						segments.push(((x, 0.0, z), (x, h_y, z), color).into());
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

#[vertex_object]
#[derive(Debug, Default, Clone)]
pub struct Vertex {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub position: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub color: Vec4,

	// If a given dimension is 0, the vertex is rendered in world space.
	// If it is 1, the vertex is rendered in chunk space.
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub chunk_space_mask: Vec3,
}

struct Segments(Vec<LineSegment>);
impl Segments {
	fn prepare(self) -> (Vec<Vertex>, Vec<u32>) {
		let mut vertices = Vec::new();
		let mut indices = Vec::new();
		for segment in self.0.into_iter() {
			for pos in [segment.pos1, segment.pos2].iter() {
				let i = vertices.len() as u32;
				vertices.push(Vertex {
					position: (*pos).into(),
					color: segment.color.into(),
					chunk_space_mask: Vec3::default(), // segment.chunkSpaceMask
				});
				indices.push(i);
			}
		}
		(vertices, indices)
	}
}

struct TypeSettings {
	index_start: usize,
	index_count: usize,
	vertex_start: usize,
}

pub type ArcLockRender = Arc<RwLock<Render>>;
pub struct Render {
	drawable: Drawable,

	control: Arc<RwLock<BoundaryControl>>,
	recorded_kind: Vec<Type>,
	type_settings: HashMap<Type, TypeSettings>,
	vertex_buffer: Arc<buffer::Buffer>,
	index_buffer: Arc<buffer::Buffer>,

	camera: Arc<RwLock<camera::Camera>>,
	camera_uniform: Uniform,

	pending_gpu_signals: Vec<Arc<command::Semaphore>>,
}

impl Render {
	fn subpass_id() -> asset::Id {
		CrystalSphinx::get_asset_id("render_pass/subpass/debug")
	}

	pub fn add_state_listener(
		app_state: &ArcLockMachine,
		render_chain: &ArcRenderChain,
		camera: &Arc<RwLock<camera::Camera>>,
		arc_user: &input::ArcLockUser,
	) {
		use state::{
			storage::{Event::*, Storage},
			State::*,
			Transition::*,
			*,
		};

		let callback_render_chain = Arc::downgrade(&render_chain);
		let callback_camera = Arc::downgrade(&camera);
		let callback_action =
			input::User::get_action_in(&arc_user, crate::input::ACTION_TOGGLE_CHUNK_BOUNDARIES)
				.unwrap();
		Storage::<ArcLockRender>::default()
			// On Enter InGame => create Self and hold ownership in `storage`
			.with_event(Create, OperationKey(None, Some(Enter), Some(InGame)))
			// On Exit InGame => drop the renderer from storage, thereby removing it from the render-chain
			.with_event(Destroy, OperationKey(Some(InGame), Some(Exit), None))
			.create_callbacks(&app_state, move || {
				profiling::scope!("init-render", ID);
				log::trace!(target: ID, "Received Enter(InGame) transition");
				let arc_render_chain = callback_render_chain.upgrade().unwrap();
				let arc_camera = callback_camera.upgrade().unwrap();
				Ok(
					match Self::create(arc_render_chain, arc_camera, callback_action.clone()) {
						Ok(arclocked) => Some(arclocked),
						Err(err) => {
							log::error!(target: ID, "{}", err);
							None
						}
					},
				)
			});
	}

	fn create(
		render_chain: Arc<RwLock<RenderChain>>,
		camera: Arc<RwLock<camera::Camera>>,
		weak_action: input::action::WeakLockState,
	) -> Result<ArcLockRender> {
		log::info!(target: ID, "Initializing");
		let render_chunks = {
			let render_chain = render_chain.read().unwrap();
			Self::new(&render_chain, camera, weak_action)?.arclocked()
		};

		let subpass_id = Self::subpass_id();
		let element = render_chunks.clone();
		engine::task::spawn(ID.to_owned(), async move {
			log::trace!(target: ID, "Adding to render chain");
			let mut render_chain = render_chain.write().unwrap();
			render_chain.add_render_chain_element(Some(subpass_id.as_string()), &element)?;
			Ok(())
		});

		Ok(render_chunks)
	}

	fn new(
		render_chain: &RenderChain,
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

		let mut pending_gpu_signals = Vec::new();

		let mut type_settings = HashMap::new();
		let mut vertices = Vec::new();
		let mut indices = Vec::new();
		for kind in EnumSet::<Type>::all().into_iter() {
			let (mut kind_vertices, mut kind_indices) = Segments(kind.line_segments()).prepare();
			type_settings.insert(
				kind,
				TypeSettings {
					index_start: indices.len(),
					index_count: kind_indices.len(),
					vertex_start: vertices.len(),
				},
			);
			vertices.append(&mut kind_vertices);
			indices.append(&mut kind_indices);
		}

		log::trace!(target: ID, "Creating buffers");

		let vertex_buffer = buffer::Buffer::create_gpu(
			Some(format!("ChunkBoundary.VertexBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::VERTEX_BUFFER,
			vertices.len() * std::mem::size_of::<Vertex>(),
			None,
		)?;

		GpuOperationBuilder::new(
			vertex_buffer.wrap_name(|v| format!("Write({})", v)),
			render_chain,
		)?
		.begin()?
		.stage(&vertices[..])?
		.copy_stage_to_buffer(&vertex_buffer)
		.add_signal_to(&mut pending_gpu_signals)
		.end()?;

		let index_buffer = buffer::Buffer::create_gpu(
			Some(format!("ChunkBoundary.IndexBuffer")),
			&render_chain.allocator(),
			flags::BufferUsage::INDEX_BUFFER,
			indices.len() * std::mem::size_of::<u32>(),
			Some(flags::IndexType::UINT32),
		)?;

		GpuOperationBuilder::new(
			index_buffer.wrap_name(|v| format!("Write({})", v)),
			render_chain,
		)?
		.begin()?
		.stage(&indices[..])?
		.copy_stage_to_buffer(&index_buffer)
		.add_signal_to(&mut pending_gpu_signals)
		.end()?;

		let camera_uniform =
			Uniform::new::<camera::UniformData, &str>("ChunkBoundary.Camera", &render_chain)?;

		let control = BoundaryControl::create(Type::None, weak_action);

		log::trace!(target: ID, "Finalizing construction");
		Ok(Self {
			drawable,
			control,
			recorded_kind: Vec::new(),
			type_settings,
			vertex_buffer,
			index_buffer,
			pending_gpu_signals,
			camera_uniform,
			camera,
		})
	}

	fn arclocked(self) -> ArcLockRender {
		Arc::new(RwLock::new(self))
	}
}

impl Drop for Render {
	fn drop(&mut self) {
		log::info!(target: ID, "Dropping from subpass({}).", Self::subpass_id());
	}
}

impl RenderChainElement for Render {
	fn name(&self) -> &'static str {
		ID
	}

	fn initialize_with(
		&mut self,
		render_chain: &mut RenderChain,
	) -> Result<Vec<Arc<command::Semaphore>>> {
		let gpu_signals = Vec::new();

		self.drawable.create_shaders(render_chain)?;
		self.camera_uniform.write_descriptor_sets(render_chain);

		let control_kind = self.control.read().unwrap().kind;
		self.recorded_kind = vec![control_kind; render_chain.frame_count()];

		Ok(gpu_signals)
	}

	fn on_render_chain_constructed(
		&mut self,
		render_chain: &RenderChain,
		resolution: &Vector2<f32>,
		subpass_id: &Option<String>,
	) -> Result<()> {
		use graphics::pipeline::{state::*, Pipeline};
		Ok(self.drawable.create_pipeline(
			render_chain,
			vec![self.camera_uniform.layout()],
			Pipeline::builder()
				.with_vertex_layout(
					vertex::Layout::default()
						.with_object::<Vertex>(0, flags::VertexInputRate::VERTEX),
				)
				.set_viewport_state(Viewport::from(structs::Extent2D {
					width: resolution.x as u32,
					height: resolution.y as u32,
				}))
				.with_topology(
					Topology::default().with_primitive(flags::PrimitiveTopology::LINE_LIST),
				)
				.with_multisampling(
					Multisampling::default()
						.with_sample_count(render_chain.max_common_sample_count()),
				)
				.set_color_blending(
					color_blend::ColorBlend::default()
						.add_attachment(color_blend::Attachment::default()),
				)
				.with_depth_stencil(
					DepthStencil::default()
						.with_depth_test()
						.with_depth_compare_op(flags::CompareOp::LESS),
				),
			subpass_id,
		)?)
	}

	fn destroy_render_chain(&mut self, render_chain: &RenderChain) -> Result<()> {
		self.drawable.destroy_pipeline(render_chain)?;
		Ok(())
	}

	#[profiling::function]
	fn prerecord_update(
		&mut self,
		_render_chain: &graphics::RenderChain,
		_buffer: &command::Buffer,
		frame: usize,
		resolution: &Vector2<f32>,
	) -> Result<bool> {
		let data = self.camera.read().unwrap().as_uniform_data(resolution);
		self.camera_uniform.write_data(frame, &data)?;

		let control_kind = self.control.read().unwrap().kind;
		let has_changed_kind = self.recorded_kind[frame] != control_kind;
		if has_changed_kind {
			self.recorded_kind[frame] = control_kind;
		}

		Ok(has_changed_kind)
	}

	#[profiling::function]
	fn record_to_buffer(&self, buffer: &mut command::Buffer, frame: usize) -> Result<()> {
		use graphics::debug;

		buffer.begin_label("Draw:Debug", debug::LABEL_COLOR_DRAW);
		{
			self.drawable.bind_pipeline(buffer);
			self.drawable
				.bind_descriptors(buffer, vec![&self.camera_uniform.get_set(frame).unwrap()]);

			buffer.bind_vertex_buffers(0, vec![&self.vertex_buffer], vec![0]);
			buffer.bind_index_buffer(&self.index_buffer, 0);

			for kind in self.recorded_kind[frame].rendered_kinds().into_iter() {
				if let Some(settings) = self.type_settings.get(&kind) {
					buffer.draw(
						settings.index_count,
						settings.index_start,
						1,
						0,
						settings.vertex_start,
					);
				}
			}
		}
		buffer.end_label();

		Ok(())
	}

	fn take_gpu_signals(&mut self) -> Vec<Arc<command::Semaphore>> {
		self.pending_gpu_signals.drain(..).collect()
	}
}
