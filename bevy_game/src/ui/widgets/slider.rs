use {
	bevy::{
		prelude::*,
		ui::RelativeCursorPosition
	},
	super::pick_bg_color
};

#[derive(Component)]
struct SliderHandle {
	value: f32,
	value_entity: Entity
}

#[derive(Component)]
struct SliderBar;

#[derive(Component)]
struct Slider;

pub trait SliderExt {
	fn slider(&mut self, label: &str, value: f32, step: f32, precision: usize, min_value: f32, max_value: f32) -> &mut Self;
}
impl SliderExt for ChildBuilder {
	///Horizontal slider
	fn slider(&mut self, label: &str, value: f32, step: f32, precision: usize, min_value: f32, max_value: f32) -> &mut Self {
		let mut root: EntityCommands = self.spawn((
			Slider,
			Node {
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
				..default()
			}
		));
		root.with_children(|parent| {
			//Label
			parent.spawn((
				Node {
					margin: UiRect::right(Val::Px(5.)),
					..default()
				},
				Text::new(label)
			));
			//value
			let value_label: EntityCommands = parent.spawn((
				Node {
					margin: UiRect::right(Val::Px(5.)),
					..default()
				},
				Text::new(format!("{:.1$}", value, precision))
			));
			let value_entity: Entity = value_label.id();
			//slider bar
			let mut slider_bar: EntityCommands = parent.spawn((
				Node {
					height: Val::Px(7.5),
					width: Val::Percent(100.),
					align_items: AlignItems::Center,
					..default()
				},
				BackgroundColor(Color::srgb_u8(0xC0, 0xC0, 0xC0)),
				BorderRadius::all(Val::Percent(20.)),
				SliderBar,
				RelativeCursorPosition::default(),
			));
			slider_bar.with_children(|bar| {
				//slider handle
				bar.spawn((
					SliderHandle {value, value_entity},
					Button::default(),
					Node {
						height: Val::Px(30.),
						width: Val::Px(20.),
						position_type: PositionType::Absolute,
						left: Val::Percent(100. * ((value - min_value) / (max_value - min_value)).clamp(0., 1.)),
						..default()
					},
					BackgroundColor(Color::srgb_u8(0xFF, 0xCC, 0x00)),
					BorderRadius::all(Val::Percent(20.)),
				))
					.observe(pick_bg_color::<Out>(Color::srgb_u8(0xFF, 0xCC, 0x00)))
					.observe(pick_bg_color::<Over>(Color::srgb_u8(0xEE, 0xBB, 0x00)))
					.observe(pick_bg_color::<Down>(Color::srgb_u8(0xCC, 0xAA, 0x00)))
					.observe(pick_bg_color::<Up>(Color::srgb_u8(0xEE, 0xBB, 0x00)))
					.observe(|
						trigger: Trigger<Pointer<Drag>>,
						mut slider_handles: Query<(&mut SliderHandle, &mut Node, &Parent)>,
						mut slider_bars: Query<&RelativeCursorPosition, With<SliderBar>>,
						mut texts: Query<&mut Text>
					| {
						if trigger.button != PointerButton::Primary || trigger.distance.y == 0. {return;}
						let (mut handle, mut handle_node, Parent(parent)) = slider_handles.get_mut(trigger.entity()).unwrap();
						let RelativeCursorPosition {normalized: Some(bar_cursor_pos), ..} = slider_bars.get(*parent).unwrap() else {return;};
						handle.value = (((min_value + bar_cursor_pos.x.clamp(0., 1.) * (max_value - min_value)) / step).round() * step).clamp(min_value, max_value);
						**texts.get_mut(handle.value_entity).unwrap() = format!("{:.1$}", handle.value, precision).to_string();
						handle_node.left = Val::Percent(bar_cursor_pos.x.clamp(0., 1.) * 100.);
					});
			});
		});
		self
	}
}