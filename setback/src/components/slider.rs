use dioxus::prelude::*;

const SLIDER_CSS: Asset = asset!("/assets/styling/slider.css");

#[derive(Props, PartialEq, Clone)]
pub struct SliderProps {
    label: String,
    id: String,
    max: u8,
    min: u8,
    value: u8,
}

#[component]
pub fn Slider(props: SliderProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: SLIDER_CSS }
        
        div { class: "slider-field panel-like",
            div { class: "slider-field__head",
                label { r#for: "{props.id}", "{props.label}" }
                span { class: "slider-field__value", id: "{props.id}Val", "{props.value}%" }
            }
            input {
                class: "slider",
                id: "{props.id}",
                max: "{props.max}",
                min: "{props.min}",
                r#type: "range",
                style: "--fill:{props.value}%",
                value: "{props.value}",
            }
        }
    }
}