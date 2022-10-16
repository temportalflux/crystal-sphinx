use crate::{
	client::model::DescriptorId,
	entity::component::{self, debug, Registration},
};
use engine::{asset, ecs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
	descriptor_id: DescriptorId,
}

impl Component {
	pub fn new(model_id: asset::Id, texture_id: asset::Id) -> Self {
		Self {
			descriptor_id: DescriptorId {
				model_id,
				texture_id,
			},
		}
	}

	pub fn descriptor(&self) -> &DescriptorId {
		&self.descriptor_id
	}
}

impl ecs::Component for Component {
	type Storage = ecs::VecStorage<Self>;
}

impl component::Component for Component {
	fn unique_id() -> &'static str {
		"crystal_sphinx::model::blender::Component"
	}

	fn display_name() -> &'static str {
		"Blender Model"
	}

	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		use debug::Registration as debug;
		Registration::<Self>::default().with_ext(debug::from::<Self>())
	}
}

impl std::fmt::Display for Component {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"blender::Component(model={}, texture={})",
			self.descriptor_id.model_id, self.descriptor_id.texture_id
		)
	}
}

impl debug::EguiInformation for Component {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label(format!("Model Id: {}", self.descriptor_id.model_id));
		ui.label(format!("Texture Id: {}", self.descriptor_id.texture_id));
	}
}
