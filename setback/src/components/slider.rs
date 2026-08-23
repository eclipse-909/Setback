use {crate::components::*, dioxus::prelude::*, uuid::Uuid};

#[derive(Props, PartialEq, Clone)]
pub struct SliderProps {
	max: u8,
	min: u8,
	value: u8,
	step: u8,
	#[props(default)]
	percentage: bool,
	#[props(default)]
	oninput: Option<EventHandler<u8>>,
	children: Element,
}

const fn percentage(value: u8, min: u8, max: u8) -> f32 {
	(value as f32 - min as f32) / (max as f32 - min as f32) * 100.
}

#[component]
pub fn Slider(props: SliderProps) -> Element {
	let min = props.min;
	let max = props.max;
	let mut value = use_signal(|| props.value);
	let is_percentage = if props.percentage { "%" } else { "" };
	let input_uuid = Uuid::new_v4().to_string();
	let input_id = input_uuid.as_str();
	rsx! {
		document::Link { rel: "stylesheet", href: SLIDER_CSS }

		div { class: "slider-field",
			div { class: "slider-field__head",
				label { r#for: input_id, {props.children} }
				span { class: "slider-field__value", id: "{input_id}-val", "{value}{is_percentage}" }
			}
			input {
				class: "slider",
				"data-value-suffix": is_percentage,
				"data-value-target": "#{input_id}-val",
				id: input_id,
				max: "{max}",
				min: "{min}",
				r#type: "range",
				style: "--fill:{percentage(value(), min, max)}%",
				value: "{value}",
				oninput: move |evt| {
					let v = evt.value().parse::<u8>().unwrap_or_default();
					value.set(v);
					if let Some(callback) = props.oninput {callback.call(v)}
				}
			}
		}
	}
}