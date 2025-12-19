use {
	crate::{
		app::App,
		cursor::MouseButtonInput,
		keyboard::{Keybind, TextInput},
	},
	std::collections::HashSet,
	taffy::NodeId,
	winit::dpi::LogicalPosition,
};

pub type ScrollNotification = fn(
	node: NodeId,
	app: &mut App,
	scroll_delta: LogicalPosition<f64>,
	mouse_pos: LogicalPosition<f64>,
);

pub type MouseButtonNotification = fn(
	node: NodeId,
	app: &mut App,
	button_events: &[MouseButtonInput],
	mouse_pos: LogicalPosition<f64>,
);

pub type KeyNotification =
	fn(node: NodeId, app: &mut App, key_events: &[TextInput], pressed_keys: &HashSet<Keybind>);
