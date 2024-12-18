use {
	super::pick_bg_color,
	bevy::prelude::*
};

#[derive(Component)]
pub struct SpinBox {
	value: u8,
	min_value: u8,
	max_value: u8,
	value_entity: Entity
}

impl SpinBox {
	pub fn value(&self) -> u8 {self.value}
}

pub trait SpinBoxExt {
	fn spin_box(&mut self, commands: &mut Commands, label: &str, value: u8, min_value: u8, max_value: u8, enabled: bool) -> &mut Self;
}
impl SpinBoxExt for ChildBuilder {
	fn spin_box(&mut self, commands: &mut Commands, label: &str, value: u8, min_value: u8, max_value: u8, enabled: bool) -> &mut Self {
		let mut root: EntityCommands = self.spawn(Node {
			flex_direction: FlexDirection::Row,
			justify_content: JustifyContent::FlexStart,
			align_items: AlignItems::FlexStart,
			display: if enabled {Display::Flex} else {Display::None},
			..default()
		});
		let root_id: Entity = root.id();
		root.with_children(|parent| {
			//label
			parent.spawn((
				Node {margin: UiRect::right(Val::Px(5.)), ..default()},
				Text::new(label)
			));
			//decrement button
			spin_box_button(parent, "<")
				.observe(|trigger: Trigger<Pointer<Click>>, mut spin_boxes: Query<&mut SpinBox>, mut texts: Query<&mut Text>| {
					if trigger.button != PointerButton::Primary {return;}
					let mut spin_box: Mut<SpinBox> = spin_boxes.get_mut(trigger.entity()).unwrap();
					if spin_box.value > spin_box.min_value {
						spin_box.value -= 1;
						**texts.get_mut(spin_box.value_entity).unwrap() = spin_box.value.to_string();
					}
				});
			//value
			commands.entity(root_id).insert(SpinBox {
				value, min_value, max_value,
				value_entity: parent.spawn((
					Node {margin: UiRect::horizontal(Val::Px(5.)), ..default()},
					Text::new(value.to_string()),
				)).id()
			});
			//increment button
			spin_box_button(parent, ">")
				.observe(|trigger: Trigger<Pointer<Click>>, mut spin_boxes: Query<&mut SpinBox>, mut texts: Query<&mut Text>| {
					if trigger.button != PointerButton::Primary {return;}
					let mut spin_box: Mut<SpinBox> = spin_boxes.get_mut(trigger.entity()).unwrap();
					if spin_box.value < spin_box.max_value {
						spin_box.value += 1;
						**texts.get_mut(spin_box.value_entity).unwrap() = spin_box.value.to_string();
					}
				});
		});
		self
	}
}

///Returns the EntityCommands of the button so you can chain an observe function like spin_box_button(...).observe(...)
fn spin_box_button(builder: &mut ChildBuilder, label: impl Into<Option<String>>) -> EntityCommands {
	let mut root: EntityCommands = builder.spawn((
		Node {
			display: Display::None,
			margin: UiRect::horizontal(Val::Px(5.)),
			width: Val::Px(30.),
			height: Val::Px(30.),
			border: UiRect::all(Val::Px(2.5)),
			justify_content: JustifyContent::Center,
			..default()
		},
		BorderColor(Color::srgb_u8(216, 216, 216)),
		BackgroundColor(Color::srgb_u8(0, 64, 0)),
		Button::default()
	));
	if let Some(label) = label {
		root.with_children(|parent| {
			parent.spawn((
				Text::new(label),
				TextFont::from_font_size(30.)
			));
		});
	}
	root
		.observe(pick_bg_color::<Out>(Color::srgb_u8(216, 216, 216)))
		.observe(pick_bg_color::<Over>(Color::srgb_u8(0, 60, 0)))
		.observe(pick_bg_color::<Down>(Color::srgb_u8(0, 55, 0)))
		.observe(pick_bg_color::<Up>(Color::srgb_u8(0, 60, 0)));
	root
}