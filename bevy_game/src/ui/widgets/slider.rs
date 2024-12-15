use {
	crate::ui::widgets::UiBuilder,
	bevy::{
		prelude::*,
		ui::RelativeCursorPosition
	}
};

#[derive(Component)]
struct SliderHandle {
	value: f32,
	step: f32,
	min_value: f32,
	max_value: f32,
	value_label: Entity
}

#[derive(Component)]
struct SliderBar;

#[derive(Component)]
struct Slider;

pub trait SliderExt {
	fn slider(&mut self, label: &str, value: f32, step: f32, min_value: f32, max_value: f32) -> UiBuilder<Entity>;
}
impl SliderExt for UiBuilder<Entity> {
	///Only horizontal sliders are possible right now
	fn slider(&mut self, label: &str, value: f32, step: f32, min_value: f32, max_value: f32) -> UiBuilder<Entity> {
		let mut slider: UiBuilder<Entity> = self.spawn((Name::new("Slider"), Slider, NodeBundle {
			style: Style {
				flex_direction: FlexDirection::Row,
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
				..default()
			},
			..default()
		}));
		//Label
		slider.spawn(TextBundle::from_section(label, TextStyle::default()).with_style(Style {
			margin: UiRect::right(Val::Px(5.)),
			..default()
		}));
		//value
		let value_label: Entity = slider.spawn(TextBundle::from_section(value.to_string(), TextStyle::default()).with_style(Style {
			margin: UiRect::right(Val::Px(5.)),
			..default()
		})).id();
		//slider bar
		slider.spawn((SliderBar, RelativeCursorPosition::default(), NodeBundle {
			style: Style {
				height: Val::Px(7.5),
				width: Val::Percent(100.),
				align_items: AlignItems::Center,
				..default()
			},
			background_color: BackgroundColor(Color::srgb_u8(0xC0, 0xC0, 0xC0)),
			border_radius: BorderRadius::all(Val::Percent(20.)),
			..default()
		}))
			//slider handle
			.spawn((SliderHandle {value, step, min_value, max_value, value_label}, RelativeCursorPosition::default(), ButtonBundle {
				style: Style {
					height: Val::Px(30.),
					width: Val::Px(20.),
					position_type: PositionType::Absolute,
					left: Val::Percent(100. * ((value - min_value) / (max_value - min_value)).clamp(0., 1.)),
					..default()
				},
				background_color: BackgroundColor(Color::srgb_u8(0xFF, 0xCC, 0x00)),
				border_radius: BorderRadius::all(Val::Percent(20.)),
				..default()
			}))
				.observe(|trigger: Trigger<Pointer<Out>>, mut bg_colors: Query<&mut BackgroundColor>| {
					*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0xFF, 0xCC, 0x00);
				})
				.observe(|trigger: Trigger<Pointer<Over>>, mut bg_colors: Query<&mut BackgroundColor>| {
					*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0xEE, 0xBB, 0x00);
				})
				// .observe(|trigger: Trigger<Pointer<Move>>, mut bg_colors: Query<&mut BackgroundColor>| {
				// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0xEE, 0xBB, 0x00);
				// })
				// .observe(|trigger: Trigger<Pointer<Up>>, mut bg_colors: Query<&mut BackgroundColor>| {
				// 	if trigger.button != PointerButton::Primary {return;}
				// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0xEE, 0xBB, 0x00);
				// })
				// .observe(|trigger: Trigger<Pointer<DragEnd>>, mut bg_colors: Query<&mut BackgroundColor>| {
				// 	if trigger.button != PointerButton::Primary {return;}
				// 	*bg_colors.get_mut(trigger.entity()) = Color::srgb_u8(0xEE, 0xBB, 0x00);
				// })
				.observe(|
					trigger: Trigger<Pointer<Drag>>,
					mut slider_handles: Query<(&mut SliderHandle, &mut Style, &mut BackgroundColor, &Parent)>,
					mut slider_bars: Query<&RelativeCursorPosition, With<SliderBar>>,
					mut texts: Query<&mut Text>
				| {
					if trigger.button != PointerButton::Primary {return;}
					let (mut handle, mut handle_style, mut bg_color, Parent(parent)) = slider_handles
						.get_mut(trigger.entity())
						.expect("Entity should have SliderHandle and Style components");
					*bg_color = BackgroundColor(Color::srgb_u8(0xCC, 0xAA, 0x00));
					if trigger.distance.y == 0. {return;}
					let RelativeCursorPosition {normalized: Some(bar_cursor_pos), ..} = slider_bars
						.get(*parent)
						.expect("Parent slider bar should have style component") else {return;};
					todo!("Fix this equation");
					handle.value = min_value + (((min_value + bar_cursor_pos.x.clamp(0., 1.) * (max_value - min_value)) - min_value) / step).round() * step;
					texts.get_mut(handle.value_label).expect("Handle should hold a valid entity with a Text component").sections[0].value = handle.value.to_string();
					handle_style.left = Val::Percent(bar_cursor_pos.x.clamp(0., 1.) * 100.);
				});
		slider
	}
}