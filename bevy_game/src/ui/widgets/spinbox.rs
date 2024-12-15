use {
	crate::ui::widgets::UiBuilder,
	bevy::prelude::*
};

#[derive(Component)]
struct SpinBox {
	value: u8,
	min_value: u8,
	max_value: u8,
	value_label: Entity
}

pub trait SpinBoxExt {
	fn spin_box(&mut self, label: &str, value: u8, min_value: u8, max_value: u8, enabled: bool) -> UiBuilder<Entity>;
}
impl SpinBoxExt for UiBuilder<Entity> {
	fn spin_box(&mut self, label: &str, value: u8, min_value: u8, max_value: u8, enabled: bool) -> UiBuilder<Entity> {
		let mut spin_box: UiBuilder<Entity> = self.spawn((Name::new("SpinBox"), NodeBundle {
			style: Style {
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
				display: if enabled {Display::Flex} else {Display::None},
				..default()
			},
			..default()
		}));
		//Label
		spin_box.spawn(TextBundle::from_section(label, TextStyle::default()).with_style(Style {
			margin: UiRect::right(Val::Px(5.)),
			..default()
		}));
		//Decrement button
		spin_box_button(&mut spin_box, "<")
			.observe(|trigger: Trigger<Pointer<Click>>, mut spin_boxes: Query<&mut SpinBox>, mut texts: Query<&mut Text>| {
				if trigger.button != PointerButton::Primary {return;}
				let mut spin_box: Mut<SpinBox> = spin_boxes.get_mut(trigger.entity()).expect("Observer should be attached to a spin box");
				if spin_box.value > spin_box.min_value {
					spin_box.value -= 1;
				}
				texts.get_mut(spin_box.value_label).expect("Text should have been updated").sections[0].value = spin_box.value.to_string();
			});
		//Value
		let value_label: Entity = spin_box.spawn(TextBundle::from_section(value.to_string(), TextStyle::default()).with_style(Style {
			margin: UiRect::horizontal(Val::Px(5.)),
			..default()
		})).id();
		spin_box.insert(SpinBox {value, min_value, max_value, value_label});
		//Increment button
		spin_box_button(&mut spin_box, ">")
			.observe(|trigger: Trigger<Pointer<Click>>, mut spin_boxes: Query<&mut SpinBox>, mut texts: Query<&mut Text>| {
				if trigger.button != PointerButton::Primary {return;}
				let mut spin_box: Mut<SpinBox> = spin_boxes.get_mut(trigger.entity()).expect("Observer should be attached to a spin box");
				if spin_box.value < spin_box.max_value {
					spin_box.value += 1;
				}
				texts.get_mut(spin_box.value_label).expect("Text should have been updated").sections[0].value = spin_box.value.to_string();
			});
		spin_box
	}
}

fn spin_box_button(builder: &mut UiBuilder<Entity>, label: impl Into<Option<String>>) -> UiBuilder<Entity> {
	let mut button: UiBuilder<Entity> = builder.spawn(ButtonBundle {
		style: Style {
			display: Display::None,
			margin: UiRect::horizontal(Val::Px(5.)),
			width: Val::Px(30.),
			height: Val::Px(30.),
			border: UiRect::all(Val::Px(2.5)),
			justify_content: JustifyContent::Center,
			..default()
		},
		border_color: BorderColor(Color::srgb_u8(216, 216, 216)),
		background_color: BackgroundColor(Color::srgb_u8(0, 64, 0)),
		..default()
	});
	if let Some(label) = label {
		button.spawn(TextBundle::from_section(label, TextStyle {font_size: 30., ..default()}));
	}
	button
		.observe(|trigger: Trigger<Pointer<Out>>, mut bg_colors: Query<&mut BackgroundColor>| {
			*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(216, 216, 216);
		})
		.observe(|trigger: Trigger<Pointer<Over>>, mut bg_colors: Query<&mut BackgroundColor>| {
			*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 60, 0);
		})
		// .observe(|trigger: Trigger<Pointer<Move>>, mut bg_colors: Query<&mut BackgroundColor>| {
		// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 60, 0);
		// })
		// .observe(|trigger: Trigger<Pointer<Up>>, mut bg_colors: Query<&mut BackgroundColor>| {
		// 	if trigger.button != PointerButton::Primary {return;}
		// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 60, 0);
		// })
		.observe(|trigger: Trigger<Pointer<Click>>, mut bg_colors: Query<&mut BackgroundColor>| {
			if trigger.button != PointerButton::Primary {return;}
			*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0, 55, 0);
		});
	button
}