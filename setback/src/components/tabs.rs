use {crate::components::*, dioxus::prelude::*};

#[derive(Props, PartialEq, Clone)]
pub struct TabsProps {
	tabs: Vec<(String, Element)>,
	#[props(default)]
	active_idx: usize,
}

/// You probably don't want to use this directly. See [setback::tabs!]
#[component]
pub fn Tabs(props: TabsProps) -> Element {
	let mut active_idx = use_signal(|| props.active_idx);
	rsx! {
		document::Link { rel: "stylesheet", href: TABS_CSS }

		div { class: "tabs",
			div { class: "tabs__list", role: "tablist",
				for (i, label) in props.tabs.iter().map(|t| t.0.as_str()).enumerate() {
					button {
						class: if i == active_idx() { "tabs__tab is-active" } else { "tabs__tab" },
						"data-tab": "{i}",
						role: "tab",
						onclick: move |_| active_idx.set(i),

						"{label}"
					}
				}
			}
			div { class: "tabs__body",
				div {
					class: "tabs__panel",
					"data-panel": "{active_idx}",
					{&props.tabs[active_idx()].1}
				}
			}
		}
	}
}

/// To use in rsx
/// ## Example
/// ```
/// rsx! {
///   div { { // <-- extra set of curly brackets needed to insert expression
///     tabs! [
///       {
///         "Tab 1",
///         div { "tab 1 content" }
///       },
///       {
///         "Tab 2",
///         div { "tab 2 content" }
///       }
///     ]
///   } }
/// }
/// ```
macro_rules! tabs {
    ( $( { $label:expr, $($body:tt)* } ),* $(,)? ) => {
        rsx! {
            Tabs {
                tabs: vec![
                    $(
                        (
                            $label.into(),
                            rsx! { $($body)* }
                        )
                    ),*
                ]
            }
        }
    };
}

pub(crate) use tabs;
