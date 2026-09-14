use {crate::components::*, dioxus::prelude::*, setback::*};

#[derive(Props, PartialEq, Clone)]
pub struct CardChipProps {
	#[props(default)]
	num: u8,
	suit: Suit,
	#[props(default)]
	face_up: bool,
	#[props(default)]
	size: CardChipSize,
	#[props(default)]
	interactive: bool,
}

impl Default for CardChipProps {
	fn default() -> Self {
		Self {
			num: 2,
			suit: Suit::Hearts,
			face_up: true,
			size: CardChipSize::Md,
			interactive: false,
		}
	}
}

#[derive(PartialEq, Clone, Copy, Default)]
pub enum CardChipSize {
	Sm,
	#[default]
	Md,
	Lg,
}

impl CardChipSize {
	const fn as_class(self) -> &'static str {
		match self {
			CardChipSize::Sm => "card-chip--sm",
			CardChipSize::Md => "card-chip--md",
			CardChipSize::Lg => "card-chip--lg",
		}
	}
}

#[component]
pub fn CardChip(props: CardChipProps) -> Element {
	let interactive = if props.interactive {
		"card-chip--interactive"
	} else {
		""
	};
	let red = if props.suit.is_red() {
		"card-chip--red"
	} else {
		""
	};
	rsx! {
		document::Link { rel: "stylesheet", href: CARD_CHIP_CSS }

		div { class: "card-chip {props.size.as_class()} {interactive} {red}",
			if props.face_up {
				if props.num != 0 {
					span { class: "card-chip__corner",
						"{Card::num_small(props.num)}"
						br {}
						"{props.suit.as_icon()}"
					}
				}
				span { class: "card-chip__pip", "{props.suit.as_icon()}" }
			}
		}
	}
}
