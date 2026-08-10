use {crate::components::*, dioxus::prelude::*, uuid::Uuid};

#[derive(Props, PartialEq, Clone)]
pub struct SliderProps {
	max: u8,
	min: u8,
	value: u8,
	step: u8,
	#[props(default)]
	percentage: bool,
	children: Element,
}

#[component]
pub fn Slider(props: SliderProps) -> Element {
	let percentage = if props.percentage { "%" } else { "" };
	let fill =
		(props.value as f32 - props.min as f32) / (props.max as f32 - props.min as f32) * 100.;
	let input_uuid = Uuid::new_v4().to_string();
	let input_id = input_uuid.as_str();
	rsx! {
		document::Link { rel: "stylesheet", href: SLIDER_CSS }

		div { class: "slider-field",
			div { class: "slider-field__head",
				label { r#for: input_id, {props.children} }
				span { class: "slider-field__value", id: "{input_id}-val", "{props.value}{percentage}" }
			}
			input {
				class: "slider",
				"data-value-suffix": percentage,
				"data-value-target": "#{input_id}-val",
				id: input_id,
				max: "{props.max}",
				min: "{props.min}",
				r#type: "range",
				style: "--fill:{fill}%",
				value: "{props.value}",
			}
		}
	}
}
