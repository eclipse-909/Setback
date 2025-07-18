use {
	crate::plugins::*,
	bevy::prelude::{Component, World, Image, Handle}
};

/// The UI for each scene is built on an entity with a RootNode component.
///When the scene is changed, the entity with the RootNode is deleted recursively with its children.
#[derive(Component)]
pub struct RootNode;

#[derive(Component)]
pub struct TextInputOnSubmitAction(pub Option<fn(&mut World)>);

#[derive(Component)]
pub struct CursorComponent {
	pub default: CursorStyle,
	pub pointer: CursorStyle
}

#[derive(Component)]
pub struct Url(pub String);