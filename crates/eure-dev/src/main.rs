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
    #[route("/?:example")]
    Home { example: Option<String> },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}
