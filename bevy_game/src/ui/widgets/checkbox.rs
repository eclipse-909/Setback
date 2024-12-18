use {
	bevy::prelude::*,
	crate::components::EntityPointer,
	super::pick_bg_color
};

#[derive(Component)]
pub struct Checkbox(bool);

impl Checkbox {
	pub fn checked(&self) -> bool {self.0}
}

pub trait CheckboxExt {
	fn checkbox(&mut self, asset_server: &AssetServer, label: &str, checked: bool) -> &mut Self;
}

impl CheckboxExt for ChildBuilder {
	fn checkbox(&mut self, asset_server: &AssetServer, label: &str, checked: bool) -> &mut Self {
		let mut root: EntityCommands = self.spawn((
			Node {
				..default()
			},
			Checkbox(checked)
		));
		let root_id: Entity = root.id();
		root.with_children(|parent| {
			//label
			parent.spawn((
				Node {
					..default()//TODO style
				},
				Text::new(label),
			));
			//button
			parent.spawn((
				Node {
					..default()//TODO style
				},
				Button::default()
			)).with_children(|button| {
				//checkmark
				button.spawn((
					Node {
						display: if checked {Display::Flex} else {Display::None},
						..default()//TODO style
					},
					Text::new('\u{E5CA}'),
					TextFont::from_font(asset_server.load("fonts/MaterialIcons-Regular.ttf")),
					EntityPointer(root_id),
					BackgroundColor(Color::srgb_u8(0, 64, 0)),
				))
					.observe(pick_bg_color::<Out>(Color::srgb_u8(216, 216, 216)))
					.observe(pick_bg_color::<Over>(Color::srgb_u8(0, 60, 0)))
					.observe(pick_bg_color::<Down>(Color::srgb_u8(0, 55, 0)))
					.observe(pick_bg_color::<Up>(Color::srgb_u8(0, 60, 0)))
					.observe(|trigger: Trigger<Pointer<Click>>, mut checkmarks: Query<(&mut Node, &EntityPointer)>, mut checkboxes: Query<&mut Checkbox>| {
						let (mut node, root) = checkmarks.get_mut(trigger.entity()).unwrap();
						let mut checkbox: Mut<Checkbox> = checkboxes.get_mut(root.0).unwrap();
						checkbox.0 = !checkbox.0;
						node.display = if checkbox.0 {Display::Flex} else {Display::None}
					});
			});
		});
		self
	}
}