use crate::graphics::voxel::Face;
use engine::{graphics::types::Vec4, math::nalgebra::Vector4};
use enumset::EnumSet;

pub struct Flags {
	pub faces: EnumSet<Face>,
}

impl Into<Vector4<f32>> for Flags {
	fn into(self) -> Vector4<f32> {
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

impl Into<Vec4> for Flags {
	fn into(self) -> Vec4 {
		let data: Vector4<f32> = self.into();
		data.into()
	}
}
