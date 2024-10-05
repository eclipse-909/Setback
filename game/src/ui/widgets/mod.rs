mod button;
mod spinbox;
mod slider;
mod setup_player;

use {
	crate::ui::scenes::RootUINode,
	slider::SliderPlugin,
	bevy::{
		prelude::*,
		ecs::system::IntoObserverSystem
	}
};

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((SliderPlugin));
	}
}

pub struct UiBuilder<'a, T> {
	commands: Commands<'a, 'a>,
	context: T
}

impl<'a, T> UiBuilder<'a, T> {
	pub fn context(&self) -> &T {&self.context}
	pub fn commands(&mut self) -> &mut Commands<'a, 'a> {&mut self.commands}
}

impl UiBuilder<'_, RootUINode> {
	pub fn spawn(&mut self, bundle: impl Bundle) -> UiBuilder<Entity> {
		self.commands().ui_builder(self.commands().spawn(bundle).id())
	}
}

impl UiBuilder<'_, Entity> {
	pub fn id(&self) -> Entity {
		self.context().get_id()
	}
	
	pub fn entity_commands(&mut self) -> EntityCommands {
		self.commands().entity(self.id())
	}
	
	/// This allows for using the `EntityCommands` of the builder, and also returning the UiBuilder with context
	/// intact for further processing
	pub fn entity_commands_inplace(&mut self, entity_commands_fn: impl FnOnce(&mut EntityCommands)) -> &mut Self {
		entity_commands_fn(&mut self.entity_commands());
		self
	}
	
	pub fn spawn(&mut self, bundle: impl Bundle) -> UiBuilder<Entity> {
		let mut new_entity: Entity = Entity::PLACEHOLDER;
		let entity: Entity = self.id();
		self.commands().entity(entity).with_children(|parent| {
			new_entity = parent.spawn(bundle).id();
		});
		self.commands().ui_builder(new_entity)
	}
	
	pub fn insert(&mut self, bundle: impl Bundle) -> &mut Self {
		self.entity_commands().insert(bundle);
		self
	}
	
	pub fn observe<E: Event, B: Bundle, M>(&mut self, system: impl IntoObserverSystem<E, B, M>) -> &mut Self {
		self.entity_commands().observe(system);
		self
	}
}

pub trait UiBuilderExt {
	fn ui_builder<T>(&mut self, context: T) -> UiBuilder<T>;
}

impl UiBuilderExt for Commands<'_, '_> {
	fn ui_builder<T>(&mut self, context: T) -> UiBuilder<T> {
		UiBuilder {
			commands: self.reborrow(),
			context,
		}
	}
}

impl UiBuilderExt for EntityCommands<'_> {
	fn ui_builder<T>(&mut self, context: T) -> UiBuilder<T> {
		UiBuilder {
			commands: self.commands(),
			context,
		}
	}
}