use crate::graphics::voxel::Face;
use engine::{graphics::types::Vec4, math::nalgebra::Vector4};

pub struct Flags {
	pub face: Face,
	pub use_biome_color: bool,
}

impl Into<Vector4<f32>> for Flags {
	fn into(self) -> Vector4<f32> {
		
		let mut flag1 = 0u32;

		// Face mask - bits (0..6) - 0b_xxxxxx
		flag1 |= self.face.model_bit() << 0;
		// Is Colorizing enabled - bit 6 - 0bx______
		flag1 |= (self.use_biome_color as u32) << 6; // 0b1000000
		
		// Convert the bits of the flag ints to the f32 for the shader
		let mut flags = Vector4::default();
		flags[0] = unsafe { std::mem::transmute(flag1) };
		flags
	}
}

impl Into<Vec4> for Flags {
	fn into(self) -> Vec4 {
		let data: Vector4<f32> = self.into();
		data.into()
	}
}
