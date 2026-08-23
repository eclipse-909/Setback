use {
	dioxus::prelude::*,
	uuid::Uuid,
};

#[derive(Props, PartialEq, Clone)]
pub struct DropdownProps {
	options: Vec<String>,
	#[props(default)]
	selected_idx: usize,
}

#[component]
pub fn Dropdown(props: DropdownProps) -> Element {
	debug_assert!(props.selected_idx < props.options.len());
	rsx! {
		select { class: "dropdown",
			for (i, opt) in props.options.iter().enumerate() {
				if i == props.selected_idx {
					option {
						selected: "false",
						value: "standard",
						"{opt}"
					}
				} else {
					option {
						value: "standard",
						"{opt}"
					}
				}
			}
		}
	}
}

#[derive(Props, PartialEq, Clone)]
pub struct LabeledDropdownProps {
	label: String,
	options: Vec<String>,
	#[props(default)]
	selected_idx: usize,
}

#[component]
pub fn LabeledDropdown(props: LabeledDropdownProps) -> Element {
	debug_assert!(props.selected_idx < props.options.len());
	let input_uuid = Uuid::new_v4().to_string();
	let input_id = input_uuid.as_str();
	rsx! {
		label {
			class: "field__label",
			r#for: input_id,
			"Rules"
		}
		select { class: "dropdown", id: input_id,
			for (i, opt) in props.options.iter().enumerate() {
				if i == props.selected_idx {
					option {
						selected: "false",
						value: "standard",
						"{opt}"
					}
				} else {
					option {
						value: "standard",
						"{opt}"
					}
				}
			}
		}
	}
}
