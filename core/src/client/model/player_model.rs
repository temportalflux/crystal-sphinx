use crate::{
	client::model::DescriptorId,
	entity::component::{self, debug, Perspective, Registration},
};
use engine::ecs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerModel {
	first_person: DescriptorId,
	third_person: DescriptorId,
	perspective: Perspective,
}

impl PlayerModel {
	pub fn new(first_person: DescriptorId, third_person: DescriptorId) -> Self {
		Self {
			first_person,
			third_person,
			perspective: Perspective::ThirdPerson,
		}
	}

	pub fn active_model(&self) -> &DescriptorId {
		match self.perspective {
			Perspective::FirstPerson => &self.first_person,
			Perspective::ThirdPerson => &self.third_person,
		}
	}

	pub fn set_perspective(&mut self, perspective: Perspective) {
		self.perspective = perspective;
	}
}

impl ecs::Component for PlayerModel {
	type Storage = ecs::VecStorage<Self>;
}

impl component::Component for PlayerModel {
	fn unique_id() -> &'static str {
		"crystal_sphinx::model::PlayerModel"
	}

	fn display_name() -> &'static str {
		"Player Model"
	}

	fn registration() -> Registration<Self>
	where
		Self: Sized,
	{
		use debug::Registration as debug;
		Registration::<Self>::default().with_ext(debug::from::<Self>())
	}
}

impl std::fmt::Display for PlayerModel {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"PlayerModel(first_person=(model={}, texture={}), third_person=(model={}, texture={}))",
			self.first_person.model_id,
			self.first_person.texture_id,
			self.third_person.model_id,
			self.third_person.texture_id
		)
	}
}

impl debug::EguiInformation for PlayerModel {
	fn render(&self, ui: &mut egui::Ui) {
		ui.label("Third Person");
		ui.indent("third", |ui| {
			ui.label(format!("Model Id: {}", self.third_person.model_id));
			ui.label(format!("Texture Id: {}", self.third_person.texture_id));
		});
		ui.label("First Person");
		ui.indent("first", |ui| {
			ui.label(format!("Model Id: {}", self.first_person.model_id));
			ui.label(format!("Texture Id: {}", self.first_person.texture_id));
		});
	}
}
