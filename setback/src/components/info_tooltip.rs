use {crate::components::*, dioxus::prelude::*};

#[derive(Props, PartialEq, Clone)]
pub struct InfoTooltipProps {
	tip: String,
}

#[component]
pub fn InfoTooltip(props: InfoTooltipProps) -> Element {
	rsx! {
		document::Link { rel: "stylesheet", href: INFO_TOOLTIP_CSS }

		span {
			class: "info-icon",
			tabindex: "0",
			svg {
				fill: "none",
				stroke: "currentColor",
				stroke_width: "2.4",
				view_box: "0 0 24 24",
				circle {
					cx: "12",
					cy: "12",
					r: "10"
				}
				path { d: "M12 16v-4M12 8h.01" }
			}
			span { class: "info-icon__tip", "{props.tip}" }
		}
	}
}
