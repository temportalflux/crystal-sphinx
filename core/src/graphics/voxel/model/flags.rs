use crate::graphics::voxel::Face;
use engine::{graphics::types::Vec4, math::nalgebra::Vector4};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Flags {
	pub face: Face,
	pub biome_color_enabled: bool,
	pub biome_color_masked: bool,
}

impl Into<Vector4<f32>> for Flags {
	fn into(self) -> Vector4<f32> {
		let mut flag1 = 0u32;

		// Face mask - bits (0..6) - 0b0xxxxxx
		flag1 |= self.face.model_bit() << 0;
		// Is Colorizing enabled - bit 6 - 0bx00000
		flag1 |= (self.biome_color_enabled as u32) << 6;
		// Does colorizing use a mask - bit 7 - 0bx000000
		flag1 |= (self.biome_color_masked as u32) << 7;

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
