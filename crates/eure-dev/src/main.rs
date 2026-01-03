mod components {
    automod::dir!(pub "src/components");
}
mod pages {
    automod::dir!(pub "src/pages");
}
mod theme;

use dioxus::prelude::*;
use pages::home::Home;

use crate::components::layout::Layout;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
    #[route("/?:example&:tab")]
    Home { example: Option<String>, tab: Option<String> },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Light theme favicons
        document::Link { rel: "icon", r#type: "image/x-icon", media: "(prefers-color-scheme: light)", href: asset!("/assets/favicon.ico") }
        document::Link { rel: "icon", r#type: "image/png", sizes: "16x16", media: "(prefers-color-scheme: light)", href: asset!("/assets/favicon-16x16.png") }
        document::Link { rel: "icon", r#type: "image/png", sizes: "32x32", media: "(prefers-color-scheme: light)", href: asset!("/assets/favicon-32x32.png") }
        document::Link { rel: "apple-touch-icon", sizes: "180x180", href: asset!("/assets/apple-touch-icon.png") }
        // Dark theme favicons
        document::Link { rel: "icon", r#type: "image/x-icon", media: "(prefers-color-scheme: dark)", href: asset!("/assets/favicon-dark.ico") }
        document::Link { rel: "icon", r#type: "image/png", sizes: "16x16", media: "(prefers-color-scheme: dark)", href: asset!("/assets/favicon-dark-16x16.png") }
        document::Link { rel: "icon", r#type: "image/png", sizes: "32x32", media: "(prefers-color-scheme: dark)", href: asset!("/assets/favicon-dark-32x32.png") }
        document::Link { rel: "apple-touch-icon", sizes: "180x180", href: asset!("/assets/apple-touch-icon-dark.png") }
        // Stylesheet
        document::Link { rel: "stylesheet", href: asset!("/assets/tailwind.css") }
        Router::<Route> {}
    }
}
