use {
	serde::{Deserialize, Serialize},
	winit::keyboard::{Key, KeyCode, NamedKey, SmolStr}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keybind {
	///What is displayed in the GUI
	pub key_code: KeyCode,
	///What is being compared for keybind checks
	pub logical_key: Key
}

impl PartialEq for Keybind {
	fn eq(&self, other: &Self) -> bool {
		self.key_code == other.key_code
	}
}
impl Eq for Keybind {}

impl std::hash::Hash for Keybind {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.key_code.hash(state);
	}
}

impl Keybind {
	pub const fn named(key: NamedKey, key_code: KeyCode) -> Self {Self {
		logical_key: Key::Named(key),
		key_code
	}}
	
	pub const fn character(key: &str, key_code: KeyCode) -> Self {Self {
		logical_key: Key::Character(SmolStr::new_inline(key)),
		key_code
	}}
}

pub enum TextInput {
	///Valid ascii text that should be appended to a buffer
	Text(SmolStr),
	///Control key: backspace, arrow, etc
	Named(NamedKey)
}