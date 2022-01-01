use crate::graphics::voxel::Face;
use engine::math::nalgebra::Vector4;
use enumset::EnumSet;

pub struct Flags {
	pub faces: EnumSet<Face>,
}

impl Flags {
	pub fn build(&self) -> Vector4<f32> {
		let mut flags = Vector4::default();

		let mut faces_enabled_bitfield: u32 = 0;
		for face in self.faces {
			faces_enabled_bitfield |= face.model_bit();
		}
		// Convert the bits of the face flag int to the f32 for the shader
		flags[0] = unsafe { std::mem::transmute(faces_enabled_bitfield) };

		flags
	}
}

impl From<Vector4<f32>> for Flags {
	fn from(flags: Vector4<f32>) -> Self {
		let faces_enabled_bitfield = unsafe { std::mem::transmute(flags[0]) };
		Self {
			faces: Face::parse_model_bit(faces_enabled_bitfield),
		}
	}
}
