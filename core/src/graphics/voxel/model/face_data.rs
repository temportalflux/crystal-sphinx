use crate::graphics::voxel::{atlas::AtlasTexCoord, model::Flags};

pub struct FaceData {
	pub main_tex: AtlasTexCoord,
	pub biome_color_tex: Option<AtlasTexCoord>,
	pub flags: Flags,
}

impl std::fmt::Debug for FaceData {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{} => (main_tex={:?}, biome_color=(enabled={}, mask={:?}))",
			self.flags.face, self.main_tex, self.flags.biome_color_enabled, self.biome_color_tex,
		)
	}
}
