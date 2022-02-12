use crate::{
	common::account,
	entity::{
		self,
		component::{self, debug},
		ArcLockEntityWorld,
	},
};
use anyhow::Result;
use engine::ui::egui::Element;
use enumset::{EnumSet, EnumSetType};
use std::{
	collections::HashSet,
	sync::{Arc, RwLock, Weak},
};

#[derive(EnumSetType)]
enum Selector {
	LocalOwner,
	ProvidedId,
}

impl std::fmt::Display for Selector {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::LocalOwner => write!(f, "Local Owner"),
			Self::ProvidedId => write!(f, "Provide Id"),
		}
	}
}

impl Selector {
	fn render<T>(&mut self, ui: &mut egui::Ui, label: T) -> bool
	where
		T: ToString + std::hash::Hash + Clone,
	{
		let mut has_changed = false;
		ui.label(label.clone());
		egui::ComboBox::from_id_source(label)
			.selected_text(format!("{}", self))
			.show_ui(ui, |ui| {
				for value in EnumSet::<Self>::all().into_iter() {
					if ui
						.selectable_value(self, value, value.to_string())
						.changed()
					{
						has_changed = true;
					}
				}
			});
		has_changed
	}
}

/// In-Game debug window for examining information about an entity (like the local player).
pub struct EntityInspector {
	entity_world: Weak<RwLock<entity::World>>,
	is_open: bool,
	selector: Selector,
	provided_entity_id: u32,
	components_to_show: HashSet<std::any::TypeId>,
}

impl EntityInspector {
	pub fn new(entity_world: &ArcLockEntityWorld) -> Self {
		Self {
			entity_world: Arc::downgrade(&entity_world),
			is_open: false,
			selector: Selector::LocalOwner,
			provided_entity_id: 0,
			components_to_show: HashSet::new(),
		}
	}
}

impl EntityInspector {
	fn local_account_id() -> Result<account::Id> {
		crate::client::account::Manager::read()
			.unwrap()
			.active_account()
			.map(|account| account.id())
	}

	fn find_entity(&self) -> Option<hecs::Entity> {
		use entity::component::OwnedByAccount;
		let arc_world = self.entity_world.upgrade().unwrap();
		let world = arc_world.read().unwrap();
		match self.selector {
			Selector::LocalOwner => {
				let local_id = match Self::local_account_id() {
					Ok(id) => id,
					Err(_) => return None,
				};
				for (entity, user) in world.query::<&OwnedByAccount>().iter() {
					if *user.id() == local_id {
						return Some(entity);
					}
				}
				None
			}
			Selector::ProvidedId => world
				.iter()
				.find(|e| e.entity().id() == self.provided_entity_id)
				.map(|e| e.entity().clone()),
		}
	}
}

impl super::PanelWindow for EntityInspector {
	fn is_open_mut(&mut self) -> &mut bool {
		&mut self.is_open
	}
}

impl Element for EntityInspector {
	fn render(&mut self, ctx: &egui::CtxRef) {
		if !self.is_open {
			return;
		}
		let mut is_open = self.is_open;
		egui::Window::new("Entity Inspector")
			.open(&mut is_open)
			.show(ctx, |ui| {
				self.render_selector(ui);
				self.render_components(ui);
			});
		self.is_open = is_open;
	}
}

impl EntityInspector {
	fn render_selector(&mut self, ui: &mut egui::Ui) {
		ui.horizontal(|ui| {
			self.selector.render(ui, "Selector");
			if self.selector == Selector::ProvidedId {
				ui.label("Entity Id");
				ui.add(egui::DragValue::new(&mut self.provided_entity_id).speed(1));
			}
		});
	}

	fn render_components(&mut self, ui: &mut egui::Ui) {
		// TODO: show entity components that are only on the server even if the client is not CotoS?
		// TODO: ComboBox of component types on the entity. Can select multiple. Those selected are shown in the list, if they have a egui debug trait implemented.

		let entity = match self.find_entity() {
			Some(entity) => entity,
			None => {
				ui.label("No entity selected.");
				return;
			}
		};

		let arc_world = self.entity_world.upgrade().unwrap();
		let world = arc_world.read().unwrap();
		let entity_ref = match world.entity(entity) {
			Ok(entity_ref) => entity_ref,
			Err(_) => {
				ui.label("Entity not found in world.");
				return;
			}
		};

		let registry = component::Registry::read();

		ui.horizontal(|ui| {
			ui.label("Components");
			egui::ComboBox::from_id_source("Components")
				.selected_text(format!("{} components", self.components_to_show.len()))
				.show_ui(ui, |ui| {
					for type_id in entity_ref.component_types() {
						if let Some(registered) = registry.find(&type_id) {
							let is_showing = self.components_to_show.contains(&type_id);
							let can_be_displayed = registered.has_ext::<debug::Registration>();
							let label =
								egui::SelectableLabel::new(is_showing, registered.display_name());
							if ui.add_enabled(can_be_displayed, label).clicked() {
								match is_showing {
									true => self.components_to_show.remove(&type_id),
									false => self.components_to_show.insert(type_id),
								};
							}
						}
					}
				});
		});
		for type_id in self.components_to_show.iter() {
			let registered = registry.find(&type_id).unwrap();
			if let Some(debug_registration) = registered.get_ext::<debug::Registration>() {
				ui.label(registered.display_name());
				ui.indent(registered.id(), |ui| {
					debug_registration.render(&entity_ref, ui);
				});
			}
		}
	}
}
