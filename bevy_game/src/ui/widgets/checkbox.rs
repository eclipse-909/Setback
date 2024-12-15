use {
	crate::ui::widgets::UiBuilder,
	bevy::prelude::*
};

#[derive(Component)]
struct Checkbox {
	checked: bool
}

pub trait CheckboxExt {
	fn checkbox(&mut self, label: &str, checked: bool) -> UiBuilder<Entity>;
}

impl CheckboxExt for UiBuilder<Entity> {
	fn checkbox(&mut self, label: &str, checked: bool) -> UiBuilder<Entity> {
		todo!()
	}
}