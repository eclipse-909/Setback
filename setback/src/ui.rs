use {
	crate::{app::App, layout::Node, style::Style},
	taffy::{Rect, Size},
	vello::{
		kurbo::RoundedRectRadii,
		peniko::{Brush, Color},
	},
};

#[inline(always)]
pub fn init_ui(app: &mut App) {
	let left = app.ui_tree.new_node(
		&app.static_arena,
		Style {
			size: Size::length(250.),
			margin: Rect::length(25.),
			padding: Rect::length(25.),
			border: Rect::length(2.),
			background: Brush::Solid(Color::from_rgb8(0x00, 0x00, 0xff)),
			border_radius: RoundedRectRadii::from_single_radius(10.),
			border_color: Color::WHITE,
			..Default::default()
		},
	);

	let right = app.ui_tree.new_node(
		&app.static_arena,
		Style {
			size: Size::length(250.),
			margin: Rect::length(25.),
			padding: Rect::length(25.),
			background: Brush::Solid(Color::from_rgb8(0x00, 0x00, 0xff)),
			..Default::default()
		},
	);

	let root_node = Node::from_id_mut(app.root_node);

	root_node.style = Style {
		size: Size::from_lengths(1500., 500.),
		background: Brush::Solid(Color::from_rgb8(0xff, 0x00, 0x00)),
		..Default::default()
	};

	root_node.push_child(left);
	root_node.push_child(right);
}
