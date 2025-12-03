use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::bs_icons::BsGithub};

use crate::{Route, theme::Theme};

#[component]
pub fn Layout() -> Element {
    let mut theme = use_context_provider(|| Signal::new(Theme::default()));
    let theme_val = theme();
    let page_bg_color = theme_val.page_bg_color();
    let text_color = theme_val.text_color();
    let surface_color = theme_val.surface1_color();

    rsx! {
		div {
			class: "min-h-screen flex flex-col",
			style: "background-color: {page_bg_color}; color: {text_color}",

			// Header
			header { class: "p-4 shrink-0",
				div { class: "max-w-6xl mx-auto w-full flex justify-between items-center",
					Link { to: "/",
						h1 { class: "text-2xl font-bold", "Eure" }
					}

					div { class: "flex items-center gap-4",
						// Toggle switch
						button {
							class: "w-14 h-8 rounded-full relative transition-colors",
							style: "background-color: {surface_color}",
							onclick: move |_| theme.set(theme().toggle()),

							// Toggle knob with emoji
							span {
								class: if theme() == Theme::Dark { "absolute top-1 left-1 w-6 h-6 rounded-full flex items-center justify-center transition-all" } else { "absolute top-1 left-7 w-6 h-6 rounded-full flex items-center justify-center transition-all" },
								style: "background-color: {page_bg_color}",
								if theme() == Theme::Dark {
									"🌙"
								} else {
									"☀️"
								}
							}
						}

						// GitHub link
						a {
							href: "https://github.com/Hihaheho/eure",
							target: "_blank",
							class: "hover:opacity-80 transition-opacity",
							Icon { icon: BsGithub, width: 24, height: 24 }
						}
					}
				}
			}

			// Main content with max-width
			main { class: "flex-1 min-h-0",
				div { class: "max-w-screen-2xl mx-auto w-full h-full", Outlet::<Route> {} }
			}

			// Footer
			footer { class: "p-2 text-center text-sm opacity-50 shrink-0", "Eure" }
		}
	}
}
