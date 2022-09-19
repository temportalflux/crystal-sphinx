use crate::client::model::DescriptorId;
use engine::{
	channels::mpsc::Sender,
	graphics::{
		alloc::{self, Object},
		buffer, command, flags, pipeline,
		types::{Mat4, Vec3},
		utility::NamedObject,
		vertex_object, GpuOpContext, GpuOperationBuilder,
	},
	math::nalgebra::{Isometry3, Point3, Translation3, UnitQuaternion},
};
use std::sync::Arc;

pub struct InstanceBuilder {
	chunk: Point3<i64>,
	offset: Point3<f32>,
	orientation: UnitQuaternion<f32>,
}

impl InstanceBuilder {
	pub fn new() -> Self {
		Self {
			chunk: Point3::new(0, 0, 0),
			offset: Point3::new(0.0, 0.0, 0.0),
			orientation: UnitQuaternion::identity(),
		}
	}

	pub fn with_chunk(mut self, coord: Point3<i64>) -> Self {
		self.chunk = coord;
		self
	}

	pub fn with_offset(mut self, offset: Point3<f32>) -> Self {
		self.offset = offset;
		self
	}

	pub fn with_orientation(mut self, rotation: UnitQuaternion<f32>) -> Self {
		self.orientation = rotation;
		self
	}

	pub fn build(self) -> Instance {
		// The model matrix transforms points in model space (relative to the origin) to move them into world space.
		// To do so, points on the model are first rotated in model space, then every point is also translated into the location in the world.
		// This is equivalent to translating the model into the world, and then rotating relative to that world location.
		let translation = Translation3::from(self.offset.coords.cast::<f32>());
		let transform = Isometry3::from_parts(translation, self.orientation);
		let model_matrix = transform.to_homogeneous().into();
		Instance {
			chunk_coordinate: self.chunk.coords.cast::<f32>().into(),
			model_matrix,
		}
	}
}

#[vertex_object]
#[derive(Clone, Debug, Default)]
pub struct Instance {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub chunk_coordinate: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	#[vertex_span(4)]
	pub model_matrix: Mat4,
}

impl Instance {
	pub fn builder() -> InstanceBuilder {
		InstanceBuilder::new()
	}
}

/*
Instances need to be grouped by their model ids so we can batch draw calls.
At this point, it makes more sense to abstract the voxel instance local buffer with different ids
		 |      Voxel      |    Entity
Item Id  | v3<i64>+v3<u8>  | hecs::Entity
Category | block::LookupId | asset::Id
Data     | voxel::Instance | model::Instance

except the above isn't the whole picture. unlike blocks, entities can have the same model and different textures.
Draw call criteria contains:
  all draws bind the same vertex+index+instance buffers
  model_id => what subset of the index buffer to draw
  tex_id => what descriptor set to bind
  instance list => what subset of the instance buffer to draw
to that end, instances should be grouped by model and then by texture
so only 1 draw call is needed for every set of entities which have the same model and texture.

What is shared between voxels and entities is that there are categories, which exist in a particular order.
Instances can be dropped or added to categories, forcing the surrounding categories to have minor shifts
in their start index or length in the overall buffer. In some cases, instances can even change categories directly.
Any mutations to the category slices or data itself must be marked and mutated data must be prepared for submission to GPU.
*/

pub struct Buffer {
	pending: Option<(Vec<DescriptorId>, Vec<Instance>)>,
	submitted: Vec<DescriptorId>,
	buffer: Arc<buffer::Buffer>,
}

impl Buffer {
	pub fn new(
		allocator: &Arc<alloc::Allocator>,
		instance_buffer_size: usize,
	) -> anyhow::Result<Self> {
		let buffer = buffer::Buffer::create_gpu(
			Some(format!("RenderModel.InstanceBuffer")),
			allocator,
			flags::BufferUsage::VERTEX_BUFFER,
			instance_buffer_size,
			None,
		)?;
		Ok(Self {
			pending: None,
			submitted: Vec::new(),
			buffer,
		})
	}

	/// This is EXTREMELY inefficient and causes every frame to reupload the entity instance buffer.
	pub fn set_pending(&mut self, descriptors: Vec<DescriptorId>, instances: Vec<Instance>) {
		self.pending = Some((descriptors, instances));
	}

	pub fn buffer(&self) -> &Arc<buffer::Buffer> {
		&self.buffer
	}

	pub fn submitted(&self) -> &Vec<DescriptorId> {
		&self.submitted
	}

	pub fn submit(
		&mut self,
		context: &impl GpuOpContext,
		signal_sender: &Sender<Arc<command::Semaphore>>,
	) -> anyhow::Result<bool> {
		let (descriptors, instances) = match self.pending.take() {
			Some(changes) => changes,
			None => return Ok(false),
		};

		let mut ranges = Vec::with_capacity(1);

		let mut task = {
			profiling::scope!("prepare-task");
			let mut task = GpuOperationBuilder::new(
				self.buffer.wrap_name(|v| format!("Write({})", v)),
				context,
			)?
			.begin()?;
			task.stage_start(self.buffer.size())?;
			task
		};

		{
			profiling::scope!("gather-instances");
			task.staging_memory()?.write_slice(&instances)?;
			ranges.push(command::CopyBufferRange {
				start_in_src: 0,
				start_in_dst: 0,
				size: self.buffer.size(),
			});
		}

		{
			profiling::scope!("run-task");
			task.copy_stage_to_buffer_ranges(&self.buffer, ranges)
				.send_signal_to(signal_sender)?
				.end()?;
		}

		self.submitted = descriptors;

		Ok(true)
	}
}
