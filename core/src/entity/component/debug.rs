/// Trait implemented by components which allows them to
/// display information in the [`Entity Inspector`](crate::debug::EntityInspector).
pub trait EguiInformation {
	fn render(&self, ui: &mut egui::Ui);
}

pub struct Registration {
	render_inspector: Box<dyn Fn(&hecs::EntityRef<'_>, &mut egui::Ui)>,
}
impl Registration {
	pub(crate) fn from<T>() -> Self
	where
		T: super::Component + EguiInformation,
	{
		Self {
			render_inspector: Box::new(|e: &hecs::EntityRef<'_>, ui: &mut egui::Ui| {
				if let Some(component) = e.get::<T>() {
					(*component).render(ui);
				}
			}),
		}
	}

	pub(crate) fn render(&self, entity_ref: &hecs::EntityRef<'_>, ui: &mut egui::Ui) {
		(self.render_inspector)(entity_ref, ui)
	}
}
