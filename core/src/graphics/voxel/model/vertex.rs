use engine::graphics::{
	flags, pipeline,
	types::{Vec3, Vec4},
	vertex_object,
};

#[vertex_object]
#[derive(Debug, Default, Clone)]
pub struct Vertex {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub position: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub tex_coord: Vec4,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub model_flags: Vec4,
}
