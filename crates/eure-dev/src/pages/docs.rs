use std::rc::Rc;

use dioxus::prelude::*;
use dioxus::{
    events::MountedData,
    html::{ScrollBehavior, geometry::PixelsVector2D},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltDocsPageKind {
    Guide,
    Adr,
    AdrIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltDocsHeading {
    pub id: &'static str,
    pub title: &'static str,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltDocsNavEntry {
    pub path: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltDocsNavGroup {
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub entries: &'static [BuiltDocsNavEntry],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltDocsNav {
    pub title: &'static str,
    pub groups: &'static [BuiltDocsNavGroup],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltDocsAdrSummary {
    pub path: &'static str,
    pub title: &'static str,
    pub status: &'static str,
    pub decision_date: &'static str,
    pub tags: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltDocsPage {
    pub public_path: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub html: &'static str,
    pub kind: BuiltDocsPageKind,
    pub headings: &'static [BuiltDocsHeading],
    pub tags: &'static [&'static str],
    pub status: Option<&'static str>,
    pub decision_date: Option<&'static str>,
}

include!(concat!(env!("OUT_DIR"), "/docs_site_generated.rs"));

#[component]
pub fn DocsIndex() -> Element {
    render_docs_path("/docs/".to_string())
}

#[component]
pub fn Docs(segments: Vec<String>) -> Element {
    let public_path = if segments.is_empty() {
        "/docs/".to_string()
    } else {
        format!("/docs/{}", segments.join("/"))
    };
    render_docs_path(public_path)
}

fn render_docs_path(public_path: String) -> Element {
    let mut scroll_container: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    use_effect(use_reactive((&public_path,), move |(public_path,)| {
        let _ = public_path;
        if let Some(container) = scroll_container.read().clone() {
            spawn(async move {
                let _ = container
                    .scroll(PixelsVector2D::new(0.0, 0.0), ScrollBehavior::Instant)
                    .await;
            });
        }
    }));

    let page = find_page(&public_path);
    let breadcrumbs = build_breadcrumbs(&public_path, page);
    let title = page
        .map(|page| format!("{} | Eure Docs", page.title))
        .unwrap_or_else(|| "Documentation | Eure Docs".to_string());
    let description = page
        .map(|page| page.description.to_string())
        .unwrap_or_else(|| format!("No generated docs page found for {}.", public_path));

    rsx! {
        document::Title { "{title}" }
        document::Meta {
            name: "description".to_string(),
            content: description.clone(),
        }
        document::Style { "{DOCS_CSS}" }

        div {
            class: "h-full overflow-auto px-4 pb-8",
            onmounted: move |e| {
                scroll_container.set(Some(e.data()));
            },
            div { class: "mx-auto grid max-w-screen-2xl gap-8 py-6 lg:grid-cols-[18rem_minmax(0,1fr)]",
                aside { class: "lg:sticky lg:top-0 lg:max-h-[calc(100vh-7rem)] lg:overflow-auto",
                    div { class: "rounded-2xl border border-white/10 bg-white/5 p-4",
                        p { class: "text-xs uppercase tracking-[0.2em] opacity-60", "{DOCS_NAV.title}" }
                        div { class: "mt-4 space-y-5",
                            for group in DOCS_NAV.groups {
                                div { class: "space-y-2",
                                    h2 { class: "text-sm font-semibold", "{group.title}" }
                                    if let Some(description) = group.description {
                                        p { class: "text-xs leading-5 opacity-70", "{description}" }
                                    }
                                    nav { class: "space-y-1",
                                        for entry in group.entries {
                                            {render_sidebar_link(entry.path, entry.label, public_path.as_str())}
                                        }
                                    }
                                }
                            }

                            div { class: "space-y-2 border-t border-white/10 pt-5",
                                h2 { class: "text-sm font-semibold", "Architecture Decision Records" }
                                p { class: "text-xs leading-5 opacity-70",
                                    "Generated from docs/adrs/*.eure and sorted by decision date."
                                }
                                nav { class: "space-y-1",
                                    {render_sidebar_link("/docs/adrs", "ADR Index", public_path.as_str())}
                                    for adr in DOCS_ADRS {
                                        {render_sidebar_link(adr.path, adr.title, public_path.as_str())}
                                    }
                                }
                            }
                        }
                    }
                }

                section { class: "min-w-0",
                    div { class: "rounded-3xl border border-white/10 bg-black/10 p-5 md:p-8",
                        nav { class: "mb-4 flex flex-wrap items-center gap-2 text-sm opacity-70",
                            for (index, crumb) in breadcrumbs.iter().enumerate() {
                                if index > 0 {
                                    span { "/" }
                                }
                                if crumb.current {
                                    span { class: "font-medium opacity-100", "{crumb.label}" }
                                } else {
                                    Link {
                                        to: crumb.path.clone(),
                                        class: "hover:opacity-100 hover:underline",
                                        "{crumb.label}"
                                    }
                                }
                            }
                        }

                        if let Some(page) = page {
                            header { class: "mb-6 space-y-3",
                                h1 { class: "text-3xl font-semibold leading-tight md:text-4xl", "{page.title}" }
                                p { class: "max-w-3xl text-sm leading-7 opacity-80 md:text-base", "{page.description}" }
                                div { class: "flex flex-wrap items-center gap-2 text-xs",
                                    if let Some(status) = page.status {
                                        span { class: "rounded-full border border-sky-400/30 bg-sky-400/10 px-2.5 py-1 uppercase tracking-wide text-sky-200", "{status}" }
                                    }
                                    if let Some(date) = page.decision_date {
                                        span { class: "rounded-full border border-white/10 bg-white/5 px-2.5 py-1", "{date}" }
                                    }
                                    for tag in page.tags {
                                        span { class: "rounded-full border border-white/10 bg-white/5 px-2.5 py-1", "{tag}" }
                                    }
                                }
                            }

                            article {
                                class: "docs-content",
                                dangerous_inner_html: "{page.html}"
                            }
                        } else {
                            header { class: "space-y-3",
                                h1 { class: "text-3xl font-semibold leading-tight md:text-4xl", "Page not found" }
                                p { class: "max-w-3xl text-sm leading-7 opacity-80 md:text-base",
                                    "The requested docs page was not generated from the current docs/ tree."
                                }
                                code { class: "block rounded-xl border border-white/10 bg-black/20 px-3 py-2 text-sm", "{public_path}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_sidebar_link(path: &'static str, label: &'static str, current_path: &str) -> Element {
    let is_active = current_path == path;
    let class = if is_active {
        "block rounded-xl border border-sky-400/30 bg-sky-400/10 px-3 py-2 text-sm font-medium text-sky-100"
    } else {
        "block rounded-xl px-3 py-2 text-sm opacity-80 transition hover:bg-white/5 hover:opacity-100"
    };

    rsx! {
        Link {
            to: path,
            class: class,
            "{label}"
        }
    }
}

fn find_page(public_path: &str) -> Option<&'static BuiltDocsPage> {
    DOCS_PAGES.iter().find(|page| page.public_path == public_path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Breadcrumb {
    label: String,
    path: String,
    current: bool,
}

fn build_breadcrumbs(
    public_path: &str,
    current_page: Option<&'static BuiltDocsPage>,
) -> Vec<Breadcrumb> {
    let mut breadcrumbs = vec![Breadcrumb {
        label: "Docs".to_string(),
        path: "/docs/".to_string(),
        current: public_path == "/docs/",
    }];

    if public_path == "/docs/" {
        return breadcrumbs;
    }

    let path_without_prefix = public_path.trim_start_matches("/docs/").trim_matches('/');
    if path_without_prefix.is_empty() {
        return breadcrumbs;
    }

    let segments: Vec<&str> = path_without_prefix.split('/').collect();
    let mut accumulated = String::from("/docs");
    for (index, segment) in segments.iter().enumerate() {
        accumulated.push('/');
        accumulated.push_str(segment);
        let label = find_page(&accumulated)
            .map(|page| page.title.to_string())
            .or_else(|| find_adr(segment).map(|adr| adr.title.to_string()))
            .unwrap_or_else(|| humanize_segment(segment));
        breadcrumbs.push(Breadcrumb {
            label,
            path: accumulated.clone(),
            current: index + 1 == segments.len(),
        });
    }

    if let Some(page) = current_page
        && let Some(last) = breadcrumbs.last_mut()
    {
        last.label = page.title.to_string();
    }

    breadcrumbs
}

fn find_adr(slug: &str) -> Option<&'static BuiltDocsAdrSummary> {
    DOCS_ADRS.iter().find(|adr| adr.path.ends_with(slug))
}

fn humanize_segment(segment: &str) -> String {
    segment
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.extend(first.to_uppercase());
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
