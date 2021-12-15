use engine::graphics::{
	flags, pipeline,
	types::{Mat4, Vec3, Vec4},
	vertex_object,
};

#[vertex_object]
#[derive(Debug, Default)]
pub struct Instance {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub chunk_coordinate: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	#[vertex_span(4)]
	pub model_matrix: Mat4,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub instance_flags: Vec4,
}
