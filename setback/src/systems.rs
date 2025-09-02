use bevy::ecs::relationship::RelatedSpawnerCommands;
use {
	crate::{
		resources::*,
		constants::*,
		components::*
	},
	libsetback::Version,
	std::time::Duration,
	bevy::{
		prelude::*,
		winit::cursor::{CursorIcon, CustomCursor, CustomCursorImage}
	}
};

pub fn new_timer() -> Timer {
	Timer::new(Duration::from_secs_f32(1.0 / CURSOR_FPS), TimerMode::Once)
}

pub fn custom_cursor_icon(
	cursor_atlas: &mut CursorAtlas,
	image: Handle<Image>
) -> CursorIcon {
	cursor_atlas.current_animation = "";
	cursor_atlas.current_frame = 0;
	cursor_atlas.frame_timer = new_timer();
	CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
		handle: image,
		..default()
	}))
}

pub fn custom_cursor_anim(
	cursor_atlas: &mut CursorAtlas,
	animation: &'static str
) -> CursorIcon {
	cursor_atlas.current_animation = animation;
	cursor_atlas.current_frame = 0;
	cursor_atlas.frame_timer = new_timer();
	CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
		handle: cursor_atlas.texture.clone(),
		texture_atlas: Some(TextureAtlas {
			layout: cursor_atlas.layout.clone(),
			index: cursor_atlas.animations.get(animation).expect("Invalid animation name")[0]
		}),
		..default()
	}))
}

pub fn compare_version(current: Version, latest: Version) -> ClientUpdate {
	if current == latest {
		ClientUpdate::UpToDate
	} else if current.0 == latest.0 && current.1 == latest.1 {
		ClientUpdate::UpdateAvailable
	} else {
		ClientUpdate::UpdateRequired
	}
}

pub fn menu_background(commands: &mut Commands, times: Handle<Font>, with_children: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>)) {
	commands.spawn((
		Node {
			width: Val::Percent(100.),
			height: Val::Percent(100.),
			margin: UiRect::all(Val::Px(0.)),
			padding: UiRect::all(Val::Px(0.)),
			box_sizing: BoxSizing::BorderBox,
			min_height: Val::Vh(100.),
			display: Display::Flex,
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			overflow: Overflow::hidden(),
			..default()
		},
		BackgroundColor(PALLET.primary_green),
		RootNode
	)).with_children(|commands| {
		//Suit icons
		commands.spawn((
			Node {
				top: Val::Px(20.),
				left: Val::Px(20.),
				position_type: PositionType::Absolute,
				justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♠"),
				TextFont::from_font(times.clone()).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		commands.spawn((
			Node {
				top: Val::Px(20.),
				right: Val::Px(20.),
				position_type: PositionType::Absolute,
				justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♦"),
				TextFont::from_font(times.clone()).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		commands.spawn((
			Node {
				bottom: Val::Px(20.),
				left: Val::Px(20.),
				position_type: PositionType::Absolute,
				justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♣"),
				TextFont::from_font(times.clone()).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		commands.spawn((
			Node {
				bottom: Val::Px(20.),
				right: Val::Px(20.),
				position_type: PositionType::Absolute,
				justify_content: JustifyContent::Center,
				..default()
			},
			children![(
				Text::new("♥"),
				TextFont::from_font(times).with_font_size(60.),
				TextColor(PALLET.accent_gold.with_alpha(0.1)),
				TextLayout::new_with_justify(JustifyText::Center)
			)]
		));
		with_children(commands);
	});
}