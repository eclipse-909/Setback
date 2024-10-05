mod scenes;
mod widgets;

use {
	crate::ui::{
		scenes::ScenesPlugin,
		widgets::WidgetsPlugin
	},
	bevy::prelude::*
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ScenesPlugin, WidgetsPlugin));
	}
}