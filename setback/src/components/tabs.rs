use dioxus::prelude::*;

const TABS_CSS: Asset = asset!("/assets/styling/tabs.css");

#[derive(Props, PartialEq, Clone)]
pub struct TabsProps {
    tabs: Vec<TabContent>,
}

#[derive(PartialEq, Clone)]
pub struct TabContent {
    label: String,
    children: Element,
}

#[component]
pub fn Tabs(props: TabsProps) -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TABS_CSS }
        
        div { class: "tabs",
            div { class: "tabs__list", role: "tablist",
                for (i, label) in props.tabs.iter().map(|t| t.label.as_str()).enumerate() {
                    if i == 0 {
                        button {
                            class: "tabs__tab is-active",
                            "data-tab": "{i}",
                            role: "tab",
                            "{label}"
                        }
                    } else {
                        button {
                            class: "tabs__tab",
                            "data-tab": "{i}",
                            role: "tab",
                            "{label}"
                        }
                    }
                }
            }
            for (i, children) in props.tabs.iter().map(|t| &t.children).enumerate() {
                if i == 0 {
                    div {
                        class: "tabs__panel",
                        "data-panel": "{i}",
                        {children}
                    }
                } else {
                    div {
                        class: "tabs__panel",
                        "data-panel": "{i}",
                        hidden: "false",
                        {children}
                    }
                }
            }
        }
    }
}