use {crate::components::*, dioxus::prelude::*, uuid::Uuid};

#[derive(Props, PartialEq, Clone)]
pub struct TextInputProps {
	label: String,
	#[props(default)]
	placeholder: String,
	#[props(default)]
	input_type: TextInputType,
	#[props(default)]
	hint_msg: String,
	#[props(default)]
	hint_is_err: bool,
	#[props(default)]
	style: String,
}

#[derive(PartialEq, Clone, Default)]
pub enum TextInputType {
	#[default]
	Text,
	Code,
	Password,
}

#[component]
pub fn TextInput(props: TextInputProps) -> Element {
	let (has_err, hint_err) = if props.hint_is_err {
		("has-error", "field__hint--error")
	} else {
		("", "")
	};
	let input_uuid = Uuid::new_v4().to_string();
	let input_id = input_uuid.as_str();
	let input = match props.input_type {
		TextInputType::Text => rsx! {
			input {
				class: "text-input {has_err}",
				id: input_id,
				placeholder: "{props.placeholder}",
				r#type: "text",
			}
		},
		TextInputType::Code => rsx! {
			input {
				class: "text-input text-input--code {has_err}",
				id: input_id,
				maxlength: "6",
				placeholder: "{props.placeholder}",
				r#type: "text",
			}
		},
		TextInputType::Password => rsx! {
			input {
				class: "text-input {has_err}",
				id: input_id,
				r#type: "password"
			}
		},
	};
	rsx! {
		document::Link { rel: "stylesheet", href: TEXT_INPUT_CSS }

		div { class: "field", style: props.style,
			label { class: "field__label", r#for: input_id, "{props.label}" }
			{input}
			if !props.hint_msg.is_empty() {
				p { class: "field__hint {hint_err}", "{props.hint_msg}" }
			}
		}
	}
}
