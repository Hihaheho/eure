//! Docs site assembly for Eure-authored content trees.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use eure::FromEure;
use eure::value::Text;
use eure_mark::{ArticleError, PageHeading, PageRenderer, parse_article_file};
use maud::{PreEscaped, html};
use regex::Regex;
use thiserror::Error;

pub type DocsHeading = PageHeading;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsPageKind {
    Guide,
    Adr,
    AdrIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsNav {
    pub title: String,
    pub groups: Vec<DocsNavGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsNavGroup {
    pub title: String,
    pub description: Option<String>,
    pub entries: Vec<DocsNavEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsNavEntry {
    pub path: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdrSummary {
    pub path: String,
    pub title: String,
    pub status: String,
    pub decision_date: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedDocsPage {
    pub public_path: String,
    pub title: String,
    pub description: String,
    pub html: String,
    pub css: String,
    pub kind: DocsPageKind,
    pub headings: Vec<DocsHeading>,
    pub tags: Vec<String>,
    pub status: Option<String>,
    pub decision_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsSite {
    pub nav: DocsNav,
    pub pages: Vec<RenderedDocsPage>,
    pub adrs: Vec<AdrSummary>,
}

#[derive(Debug, Error)]
pub enum DocsError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse eure syntax in {path}: {source}")]
    ParseEure {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to build a document from {path}: {source}")]
    BuildDocument {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to decode {path} into the docs model: {source}")]
    DecodeDocument {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to render guide page {path}: {source}")]
    RenderGuide {
        path: PathBuf,
        #[source]
        source: ArticleError,
    },

    #[error("failed to initialize article renderer: {source}")]
    InitRenderer {
        #[source]
        source: ArticleError,
    },

    #[error("failed to render ADR text for {path}: {source}")]
    RenderAdrText {
        path: PathBuf,
        #[source]
        source: ArticleError,
    },

    #[error("nav entry {path} is listed more than once")]
    DuplicateNavEntry { path: String },

    #[error("nav entry {path} does not map to a guide page")]
    MissingNavTarget { path: String },

    #[error("guide page {path} is missing from docs/_nav.eure")]
    UnlistedGuidePage { path: String },

    #[error("page {page} links to a missing docs page {target}")]
    MissingLinkedPage { page: String, target: String },

    #[error("page {page} links to missing anchor #{anchor} on {target}")]
    MissingAnchor {
        page: String,
        target: String,
        anchor: String,
    },

    #[error("page {page} still contains a repo-relative docs link {href}")]
    UnmigratedDocsLink { page: String, href: String },

    #[error("docs/index.eure is required but was not found")]
    MissingDocsIndex,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
struct AdrDocument {
    #[eure(ext, default)]
    schema: Option<String>,
    id: Text,
    title: Text,
    status: AdrStatus,
    #[eure(rename = "decision-date")]
    decision_date: Text,
    #[eure(default)]
    tags: Vec<String>,
    #[eure(rename = "related-adrs", default)]
    related_adrs: Vec<String>,
    #[eure(rename = "related-links", default)]
    related_links: Vec<String>,
    #[eure(default)]
    authors: Vec<String>,
    context: Text,
    decision: Text,
    consequences: Text,
    #[eure(rename = "alternatives-considered", default)]
    alternatives_considered: Vec<Text>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
enum AdrStatus {
    #[eure(rename = "proposed")]
    Proposed,
    #[eure(rename = "accepted")]
    Accepted,
    #[eure(rename = "rejected")]
    Rejected { reason: Text },
    #[eure(rename = "deprecated")]
    Deprecated { reason: Text },
    #[eure(rename = "superseded")]
    Superseded {
        #[eure(rename = "superseded_by")]
        superseded_by: String,
    },
}

impl AdrStatus {
    fn label(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Rejected { .. } => "rejected",
            Self::Deprecated { .. } => "deprecated",
            Self::Superseded { .. } => "superseded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, FromEure)]
#[eure(crate = ::eure::document)]
struct DocsNavDocument {
    #[eure(default)]
    title: Option<String>,
    #[eure(default)]
    groups: Vec<DocsNavGroupDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromEure)]
#[eure(crate = ::eure::document)]
struct DocsNavGroupDocument {
    title: String,
    #[eure(default)]
    description: Option<String>,
    #[eure(default)]
    pages: Vec<DocsNavEntryDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, FromEure)]
#[eure(crate = ::eure::document)]
struct DocsNavEntryDocument {
    path: String,
    label: String,
}

static HREF_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]+)""#).expect("valid href regex"));

pub fn build_docs_site(docs_dir: &Path) -> Result<DocsSite, DocsError> {
    let nav = parse_nav(&docs_dir.join("_nav.eure"))?;
    let renderer = PageRenderer::new().map_err(|source| DocsError::InitRenderer { source })?;
    let shared_css = format!("{}\n{}", renderer.css(), generate_builder_css());

    let guide_files = collect_guide_files(docs_dir)?;
    let adr_files = collect_adr_files(docs_dir)?;

    let mut pages = Vec::new();
    let mut guide_paths = HashSet::new();
    let mut adr_summaries = Vec::new();
    let mut found_docs_index = false;

    for file in guide_files {
        let public_path = public_path_for_file(docs_dir, &file);
        if public_path == "/docs/" {
            found_docs_index = true;
        }

        let article = parse_article_file(&file).map_err(|source| DocsError::RenderGuide {
            path: file.clone(),
            source,
        })?;
        let rendered =
            renderer
                .render_article(&article)
                .map_err(|source| DocsError::RenderGuide {
                    path: file.clone(),
                    source,
                })?;

        guide_paths.insert(public_path.clone());
        pages.push(RenderedDocsPage {
            public_path,
            title: rendered.title,
            description: rendered.description,
            html: rendered.html,
            css: shared_css.clone(),
            kind: DocsPageKind::Guide,
            headings: rendered.headings,
            tags: rendered.metadata.tags,
            status: None,
            decision_date: None,
        });
    }

    if !found_docs_index {
        return Err(DocsError::MissingDocsIndex);
    }

    let mut adr_pages = Vec::new();
    for file in adr_files {
        let public_path = public_path_for_file(docs_dir, &file);
        let document = parse_eure_file(&file)?;
        let rendered = render_adr_page(&document, &public_path, &shared_css, &renderer, &file)?;
        adr_summaries.push(AdrSummary {
            path: rendered.public_path.clone(),
            title: rendered.title.clone(),
            status: rendered.status.clone().unwrap_or_default(),
            decision_date: rendered.decision_date.clone().unwrap_or_default(),
            tags: rendered.tags.clone(),
        });
        adr_pages.push(rendered);
    }

    adr_summaries.sort_by(|left, right| {
        right
            .decision_date
            .cmp(&left.decision_date)
            .then_with(|| right.title.cmp(&left.title))
    });
    adr_pages.sort_by(|left, right| {
        right
            .decision_date
            .cmp(&left.decision_date)
            .then_with(|| right.title.cmp(&left.title))
    });

    validate_nav(&nav, &guide_paths)?;

    pages.push(render_adr_index_page(&adr_summaries, &shared_css));
    pages.extend(adr_pages);

    validate_links(&pages)?;

    Ok(DocsSite {
        nav,
        pages,
        adrs: adr_summaries,
    })
}

fn collect_guide_files(root: &Path) -> Result<Vec<PathBuf>, DocsError> {
    let mut files = Vec::new();
    collect_eure_files(root, root, &mut files)?;
    files.retain(|path| {
        if path.file_name().is_some_and(|name| name == "_nav.eure") {
            return false;
        }
        !path
            .strip_prefix(root)
            .expect("path is under docs root")
            .starts_with("adrs")
    });
    files.sort();
    Ok(files)
}

fn collect_adr_files(root: &Path) -> Result<Vec<PathBuf>, DocsError> {
    let mut files = Vec::new();
    let adr_root = root.join("adrs");
    if adr_root.exists() {
        collect_eure_files(root, &adr_root, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_eure_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), DocsError> {
    for entry in fs::read_dir(dir).map_err(|source| DocsError::ReadFile {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| DocsError::ReadFile {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".DS_Store") {
            continue;
        }
        if path.is_dir() {
            collect_eure_files(root, &path, files)?;
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "eure") && path.starts_with(root) {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_nav(path: &Path) -> Result<DocsNav, DocsError> {
    let nav: DocsNavDocument = parse_eure_file(path)?;
    Ok(DocsNav {
        title: nav.title.unwrap_or_else(|| "Documentation".to_string()),
        groups: nav
            .groups
            .into_iter()
            .map(|group| DocsNavGroup {
                title: group.title,
                description: group.description,
                entries: group
                    .pages
                    .into_iter()
                    .map(|entry| DocsNavEntry {
                        path: normalize_docs_path(&entry.path),
                        label: entry.label,
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn parse_eure_file<T>(path: &Path) -> Result<T, DocsError>
where
    for<'doc> T: FromEure<'doc>,
    for<'doc> <T as FromEure<'doc>>::Error: std::error::Error + Send + Sync + 'static,
{
    let source = fs::read_to_string(path).map_err(|source| DocsError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let cst = eure::parol::parse(&source, path).map_err(|source| DocsError::ParseEure {
        path: path.to_path_buf(),
        source: Box::new(source),
    })?;
    let document = eure::document::cst_to_document(&source, &cst).map_err(|source| {
        DocsError::BuildDocument {
            path: path.to_path_buf(),
            source: Box::new(source),
        }
    })?;
    document
        .parse(document.get_root_id())
        .map_err(|source| DocsError::DecodeDocument {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
}

fn public_path_for_file(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .expect("docs file is inside docs root");
    let relative = relative.with_extension("");
    if relative == Path::new("index") {
        "/docs/".to_string()
    } else {
        format!("/docs/{}", relative.to_string_lossy().replace('\\', "/"))
    }
}

fn normalize_docs_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed == "/docs" || trimmed == "/docs/" {
        "/docs/".to_string()
    } else if let Some(rest) = trimmed.strip_prefix("/docs/") {
        format!("/docs/{}", rest.trim_end_matches('/'))
    } else if let Some(rest) = trimmed.strip_prefix("docs/") {
        format!("/docs/{}", rest.trim_end_matches('/'))
    } else {
        trimmed.to_string()
    }
}

fn validate_nav(nav: &DocsNav, guide_paths: &HashSet<String>) -> Result<(), DocsError> {
    let mut seen = HashSet::new();
    for group in &nav.groups {
        for entry in &group.entries {
            if !seen.insert(entry.path.clone()) {
                return Err(DocsError::DuplicateNavEntry {
                    path: entry.path.clone(),
                });
            }
            if !guide_paths.contains(&entry.path) {
                return Err(DocsError::MissingNavTarget {
                    path: entry.path.clone(),
                });
            }
        }
    }

    for path in guide_paths {
        if !seen.contains(path) {
            return Err(DocsError::UnlistedGuidePage { path: path.clone() });
        }
    }

    Ok(())
}

fn validate_links(pages: &[RenderedDocsPage]) -> Result<(), DocsError> {
    let mut page_map = HashMap::new();
    for (index, page) in pages.iter().enumerate() {
        page_map.insert(page.public_path.clone(), index);
    }

    for page in pages {
        for capture in HREF_PATTERN.captures_iter(&page.html) {
            let href = capture
                .get(1)
                .expect("href regex always contains capture")
                .as_str();
            if href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with("mailto:")
                || href.starts_with("data:")
            {
                continue;
            }
            if href.contains(".md") || href.contains(".eure") {
                return Err(DocsError::UnmigratedDocsLink {
                    page: page.public_path.clone(),
                    href: href.to_string(),
                });
            }

            if let Some(anchor) = href.strip_prefix('#') {
                if !anchor.is_empty() && !page.headings.iter().any(|heading| heading.id == anchor) {
                    return Err(DocsError::MissingAnchor {
                        page: page.public_path.clone(),
                        target: page.public_path.clone(),
                        anchor: anchor.to_string(),
                    });
                }
                continue;
            }

            if !href.starts_with("/docs") {
                continue;
            }

            let (target_path, anchor) = href
                .split_once('#')
                .map_or((href, None), |(path, anchor)| (path, Some(anchor)));
            let target_path = normalize_docs_path(target_path);
            let Some(target_index) = page_map.get(&target_path) else {
                return Err(DocsError::MissingLinkedPage {
                    page: page.public_path.clone(),
                    target: target_path,
                });
            };
            if let Some(anchor) = anchor
                && !anchor.is_empty()
                && !pages[*target_index]
                    .headings
                    .iter()
                    .any(|heading| heading.id == anchor)
            {
                return Err(DocsError::MissingAnchor {
                    page: page.public_path.clone(),
                    target: pages[*target_index].public_path.clone(),
                    anchor: anchor.to_string(),
                });
            }
        }
    }

    Ok(())
}

fn render_adr_page(
    doc: &AdrDocument,
    public_path: &str,
    css: &str,
    renderer: &PageRenderer,
    path: &Path,
) -> Result<RenderedDocsPage, DocsError> {
    let mut headings = vec![
        DocsHeading {
            id: "context".to_string(),
            title: "Context".to_string(),
            level: 2,
        },
        DocsHeading {
            id: "decision".to_string(),
            title: "Decision".to_string(),
            level: 2,
        },
        DocsHeading {
            id: "consequences".to_string(),
            title: "Consequences".to_string(),
            level: 2,
        },
    ];
    if !doc.alternatives_considered.is_empty() {
        headings.push(DocsHeading {
            id: "alternatives-considered".to_string(),
            title: "Alternatives Considered".to_string(),
            level: 2,
        });
    }
    if !doc.related_adrs.is_empty() {
        headings.push(DocsHeading {
            id: "related-adrs".to_string(),
            title: "Related ADRs".to_string(),
            level: 2,
        });
    }
    if !doc.related_links.is_empty() {
        headings.push(DocsHeading {
            id: "related-links".to_string(),
            title: "Related Links".to_string(),
            level: 2,
        });
    }

    let context = renderer
        .render_text_fragment(&doc.context)
        .map_err(|source| DocsError::RenderAdrText {
            path: path.to_path_buf(),
            source,
        })?;
    let decision = renderer
        .render_text_fragment(&doc.decision)
        .map_err(|source| DocsError::RenderAdrText {
            path: path.to_path_buf(),
            source,
        })?;
    let consequences = renderer
        .render_text_fragment(&doc.consequences)
        .map_err(|source| DocsError::RenderAdrText {
            path: path.to_path_buf(),
            source,
        })?;

    let mut alternatives = Vec::new();
    for alternative in &doc.alternatives_considered {
        alternatives.push(
            renderer
                .render_text_fragment(alternative)
                .map_err(|source| DocsError::RenderAdrText {
                    path: path.to_path_buf(),
                    source,
                })?,
        );
    }

    let status_detail = render_status_detail(doc, renderer, path)?;
    let body = html! {
        article class="edoc-page edoc-page-adr" {
            section class="edoc-adr-meta" {
                dl class="edoc-adr-meta-grid" {
                    dt { "ID" }
                    dd { (doc.id.as_str()) }
                    dt { "Status" }
                    dd { (doc.status.label()) }
                    dt { "Decision Date" }
                    dd { (doc.decision_date.as_str()) }
                    @if !doc.tags.is_empty() {
                        dt { "Tags" }
                        dd {
                            div class="edoc-tag-list" {
                                @for tag in &doc.tags {
                                    span class="edoc-tag" { (tag) }
                                }
                            }
                        }
                    }
                    @if !doc.authors.is_empty() {
                        dt { "Authors" }
                        dd { (doc.authors.join(", ")) }
                    }
                }
            }

            @if let Some(status_detail) = status_detail {
                section class="edoc-adr-status-detail" {
                    (PreEscaped(status_detail))
                }
            }

            section class="edoc-adr-section" {
                h2 id="context" { "Context" }
                (PreEscaped(context))
            }

            section class="edoc-adr-section" {
                h2 id="decision" { "Decision" }
                (PreEscaped(decision))
            }

            section class="edoc-adr-section" {
                h2 id="consequences" { "Consequences" }
                (PreEscaped(consequences))
            }

            @if !alternatives.is_empty() {
                section class="edoc-adr-section" {
                    h2 id="alternatives-considered" { "Alternatives Considered" }
                    @for alternative in &alternatives {
                        div class="edoc-adr-alternative" {
                            (PreEscaped(alternative))
                        }
                    }
                }
            }

            @if !doc.related_adrs.is_empty() {
                section class="edoc-adr-section" {
                    h2 id="related-adrs" { "Related ADRs" }
                    ul class="edoc-link-list" {
                        @for related_adr in &doc.related_adrs {
                            li {
                                a href=(format!("/docs/adrs/{}", related_adr)) { (related_adr) }
                            }
                        }
                    }
                }
            }

            @if !doc.related_links.is_empty() {
                section class="edoc-adr-section" {
                    h2 id="related-links" { "Related Links" }
                    ul class="edoc-link-list" {
                        @for related_link in &doc.related_links {
                            li {
                                a href=(related_link) { (related_link) }
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(RenderedDocsPage {
        public_path: public_path.to_string(),
        title: doc.title.as_str().to_string(),
        description: summarize_text(&doc.context),
        html: body.into_string(),
        css: css.to_string(),
        kind: DocsPageKind::Adr,
        headings,
        tags: doc.tags.clone(),
        status: Some(doc.status.label().to_string()),
        decision_date: Some(doc.decision_date.as_str().to_string()),
    })
}

fn render_status_detail(
    doc: &AdrDocument,
    renderer: &PageRenderer,
    path: &Path,
) -> Result<Option<String>, DocsError> {
    match &doc.status {
        AdrStatus::Rejected { reason } | AdrStatus::Deprecated { reason } => renderer
            .render_text_fragment(reason)
            .map(Some)
            .map_err(|source| DocsError::RenderAdrText {
                path: path.to_path_buf(),
                source,
            }),
        AdrStatus::Superseded { superseded_by } => Ok(Some(
            html! {
                p {
                    "Superseded by "
                    a href=(format!("/docs/adrs/{}", superseded_by)) { (superseded_by) }
                    "."
                }
            }
            .into_string(),
        )),
        AdrStatus::Proposed | AdrStatus::Accepted => Ok(None),
    }
}

fn render_adr_index_page(adrs: &[AdrSummary], css: &str) -> RenderedDocsPage {
    let body = html! {
        article class="edoc-page edoc-page-adr-index" {
            p class="edoc-intro" {
                "Architecture decision records capture notable language and implementation choices."
            }
            div class="edoc-card-list" {
                @for adr in adrs {
                    article class="edoc-card" {
                        h2 class="edoc-card-title" {
                            a href=(adr.path.as_str()) { (adr.title.as_str()) }
                        }
                        div class="edoc-card-meta" {
                            span { (adr.status.as_str()) }
                            span { "•" }
                            span { (adr.decision_date.as_str()) }
                        }
                        @if !adr.tags.is_empty() {
                            div class="edoc-tag-list" {
                                @for tag in &adr.tags {
                                    span class="edoc-tag" { (tag) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    RenderedDocsPage {
        public_path: "/docs/adrs".to_string(),
        title: "Architecture Decision Records".to_string(),
        description: "Decision records generated from docs/adrs/*.eure.".to_string(),
        html: body.into_string(),
        css: css.to_string(),
        kind: DocsPageKind::AdrIndex,
        headings: Vec::new(),
        tags: Vec::new(),
        status: None,
        decision_date: None,
    }
}

fn summarize_text(text: &Text) -> String {
    let summary = text.as_str().trim().replace('\n', " ");
    summary.chars().take(180).collect()
}

pub fn generate_llms_txt(site: &DocsSite, base_url: &str) -> String {
    let base_url = base_url.trim_end_matches('/');
    let mut out = String::new();

    out.push_str(&format!("# {}\n", site.nav.title));

    if let Some(index_page) = site.pages.iter().find(|p| p.public_path == "/docs/")
        && !index_page.description.is_empty()
    {
        out.push('\n');
        out.push_str(&format!("> {}\n", index_page.description));
    }

    let page_desc: std::collections::HashMap<&str, &str> = site
        .pages
        .iter()
        .map(|p| (p.public_path.as_str(), p.description.as_str()))
        .collect();

    for group in &site.nav.groups {
        out.push('\n');
        out.push_str(&format!("## {}\n\n", group.title));
        for entry in &group.entries {
            let desc = page_desc.get(entry.path.as_str()).copied().unwrap_or("");
            if desc.is_empty() {
                out.push_str(&format!(
                    "- [{}]({}{})\n",
                    entry.label, base_url, entry.path
                ));
            } else {
                out.push_str(&format!(
                    "- [{}]({}{}):{}{}\n",
                    entry.label,
                    base_url,
                    entry.path,
                    if desc.starts_with(' ') { "" } else { " " },
                    desc
                ));
            }
        }
    }

    if !site.adrs.is_empty() {
        out.push('\n');
        out.push_str("## Optional\n\n");
        for adr in &site.adrs {
            out.push_str(&format!(
                "- [{}]({}{}):{}{}, {}\n",
                adr.title,
                base_url,
                adr.path,
                if adr.decision_date.starts_with(' ') {
                    ""
                } else {
                    " "
                },
                adr.decision_date,
                adr.status
            ));
        }
    }

    out
}

/// Returns the combined CSS for all static docs pages as a single stylesheet.
///
/// Write this to `public/docs/docs.css`; each page references it via `<link>`.
pub fn generate_static_docs_css(site: &DocsSite) -> String {
    let article_css = site.pages.first().map(|p| p.css.as_str()).unwrap_or("");
    let layout_css = generate_static_layout_css();
    format!("{article_css}\n{layout_css}")
}

/// Generates complete standalone HTML pages for each docs page.
///
/// Returns `(relative_file_path, full_html)` pairs, e.g.:
/// - `("docs/index.html", "<!DOCTYPE html>...")`
/// - `("docs/getting-started.html", ...)`
/// - `("docs/adrs/0001-foo.html", ...)`
pub fn generate_static_docs_site(site: &DocsSite) -> Vec<(String, String)> {
    site.pages
        .iter()
        .map(|page| {
            let file_path = public_path_to_static_file_path(&page.public_path);
            let html = render_static_page(site, page);
            (file_path, html)
        })
        .collect()
}

fn public_path_to_static_file_path(public_path: &str) -> String {
    // "/docs/"         → "docs/index.html"
    // "/docs/foo"      → "docs/foo.html"
    // "/docs/adrs/bar" → "docs/adrs/bar.html"
    let without_leading_slash = public_path.trim_start_matches('/');
    if without_leading_slash == "docs/" || without_leading_slash == "docs" {
        return "docs/index.html".to_string();
    }
    let trimmed = without_leading_slash.trim_end_matches('/');
    format!("{trimmed}.html")
}

fn render_static_page(site: &DocsSite, page: &RenderedDocsPage) -> String {
    let title = format!("{} | Eure Docs", page.title);
    let breadcrumbs = build_static_breadcrumbs(&page.public_path, page, site);
    let sidebar = render_static_sidebar(site, &page.public_path);

    let markup = maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                meta name="description" content=(page.description);
                link rel="stylesheet" href="/docs/docs.css";
            }
            body {
                div class="ssg-layout" {
                    aside class="ssg-sidebar" {
                        nav class="ssg-sidebar-inner" {
                            (sidebar)
                        }
                    }
                    main class="ssg-main" {
                        nav class="ssg-breadcrumbs" {
                            @for (i, crumb) in breadcrumbs.iter().enumerate() {
                                @if i > 0 { span class="ssg-breadcrumb-sep" { "/" } }
                                @if crumb.2 {
                                    span class="ssg-breadcrumb-current" { (crumb.0.as_str()) }
                                } @else {
                                    a href=(crumb.1.as_str()) class="ssg-breadcrumb-link" { (crumb.0.as_str()) }
                                }
                            }
                        }
                        header class="ssg-page-header" {
                            h1 class="ssg-page-title" { (page.title) }
                            @if !page.description.is_empty() {
                                p class="ssg-page-desc" { (page.description) }
                            }
                            @if page.status.is_some() || page.decision_date.is_some() || !page.tags.is_empty() {
                                div class="ssg-page-meta" {
                                    @if let Some(status) = &page.status {
                                        span class="ssg-tag ssg-tag-status" { (status) }
                                    }
                                    @if let Some(date) = &page.decision_date {
                                        span class="ssg-tag" { (date) }
                                    }
                                    @for tag in &page.tags {
                                        span class="ssg-tag" { (tag) }
                                    }
                                }
                            }
                        }
                        article class="docs-content" {
                            (maud::PreEscaped(&page.html))
                        }
                    }
                }
            }
        }
    };
    markup.into_string()
}

/// Returns `(label, path, is_current)` breadcrumb triples.
fn build_static_breadcrumbs(
    public_path: &str,
    current_page: &RenderedDocsPage,
    site: &DocsSite,
) -> Vec<(String, String, bool)> {
    let mut crumbs: Vec<(String, String, bool)> = vec![(
        "Docs".to_string(),
        "/docs/".to_string(),
        public_path == "/docs/",
    )];

    if public_path == "/docs/" {
        return crumbs;
    }

    let inner = public_path.trim_start_matches("/docs/").trim_matches('/');
    if inner.is_empty() {
        return crumbs;
    }

    let segments: Vec<&str> = inner.split('/').collect();
    let mut accumulated = String::from("/docs");
    for (i, seg) in segments.iter().enumerate() {
        accumulated.push('/');
        accumulated.push_str(seg);
        let label = site
            .pages
            .iter()
            .find(|p| p.public_path == accumulated)
            .map(|p| p.title.clone())
            .or_else(|| {
                site.adrs
                    .iter()
                    .find(|a| a.path.ends_with(seg))
                    .map(|a| a.title.clone())
            })
            .unwrap_or_else(|| humanize_slug(seg));
        let is_current = i + 1 == segments.len();
        let label = if is_current {
            current_page.title.clone()
        } else {
            label
        };
        crumbs.push((label, accumulated.clone(), is_current));
    }
    crumbs
}

fn humanize_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
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

fn render_static_sidebar(site: &DocsSite, current_path: &str) -> maud::Markup {
    maud::html! {
        p class="ssg-nav-label" { (site.nav.title) }
        @for group in &site.nav.groups {
            div class="ssg-nav-group" {
                h2 class="ssg-nav-group-title" { (group.title) }
                @if let Some(desc) = &group.description {
                    p class="ssg-nav-group-desc" { (desc) }
                }
                nav {
                    @for entry in &group.entries {
                        (render_static_nav_link(&entry.path, &entry.label, current_path))
                    }
                }
            }
        }
        div class="ssg-nav-group ssg-nav-group-adrs" {
            h2 class="ssg-nav-group-title" { "Architecture Decision Records" }
            p class="ssg-nav-group-desc" { "Generated from docs/adrs/*.eure and sorted by decision date." }
            nav {
                (render_static_nav_link("/docs/adrs", "ADR Index", current_path))
                @for adr in &site.adrs {
                    (render_static_nav_link(&adr.path, &adr.title, current_path))
                }
            }
        }
    }
}

fn render_static_nav_link(path: &str, label: &str, current_path: &str) -> maud::Markup {
    let is_active = current_path == path;
    let class = if is_active {
        "ssg-nav-link ssg-nav-link-active"
    } else {
        "ssg-nav-link"
    };
    maud::html! {
        a href=(path) class=(class) { (label) }
    }
}

fn generate_static_layout_css() -> &'static str {
    r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
  background: var(--emark-bg, #11111b);
  color: var(--emark-text, #cdd6f4);
  min-height: 100vh;
  font-size: 15px;
  line-height: 1.6;
}

a { color: var(--emark-blue, #89b4fa); text-decoration: none; }
a:hover { text-decoration: underline; }

.ssg-layout {
  display: grid;
  grid-template-columns: 18rem minmax(0, 1fr);
  max-width: 1400px;
  margin: 0 auto;
  gap: 2rem;
  padding: 1.5rem 1rem 3rem;
  min-height: 100vh;
}

@media (max-width: 900px) {
  .ssg-layout { grid-template-columns: 1fr; }
  .ssg-sidebar { position: static !important; }
}

.ssg-sidebar {
  position: sticky;
  top: 1.5rem;
  max-height: calc(100vh - 3rem);
  overflow-y: auto;
}

.ssg-sidebar-inner {
  border: 1px solid rgba(255,255,255,0.1);
  background: rgba(255,255,255,0.03);
  border-radius: 1rem;
  padding: 1rem;
}

.ssg-nav-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.2em;
  opacity: 0.5;
  margin-bottom: 1rem;
}

.ssg-nav-group {
  margin-bottom: 1.25rem;
}

.ssg-nav-group-adrs {
  border-top: 1px solid rgba(255,255,255,0.1);
  padding-top: 1.25rem;
}

.ssg-nav-group-title {
  font-size: 0.85rem;
  font-weight: 600;
  margin-bottom: 0.4rem;
}

.ssg-nav-group-desc {
  font-size: 0.75rem;
  opacity: 0.65;
  line-height: 1.5;
  margin-bottom: 0.5rem;
}

.ssg-nav-link {
  display: block;
  border-radius: 0.6rem;
  padding: 0.4rem 0.75rem;
  font-size: 0.85rem;
  opacity: 0.8;
  color: inherit;
  transition: background 0.1s, opacity 0.1s;
}

.ssg-nav-link:hover {
  background: rgba(255,255,255,0.05);
  opacity: 1;
  text-decoration: none;
}

.ssg-nav-link-active {
  border: 1px solid rgba(137,180,250,0.3);
  background: rgba(137,180,250,0.1);
  color: #bdd5ff;
  opacity: 1;
  font-weight: 500;
}

.ssg-main {
  min-width: 0;
}

.ssg-breadcrumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.85rem;
  opacity: 0.6;
  margin-bottom: 1rem;
}

.ssg-breadcrumb-sep { opacity: 0.5; }

.ssg-breadcrumb-link:hover { opacity: 1; }

.ssg-breadcrumb-current { font-weight: 500; opacity: 1; }

.ssg-page-header {
  margin-bottom: 2rem;
  border: 1px solid rgba(255,255,255,0.08);
  background: rgba(0,0,0,0.15);
  border-radius: 1.25rem;
  padding: 1.5rem 2rem 1.25rem;
}

.ssg-page-title {
  font-size: 2rem;
  font-weight: 600;
  line-height: 1.25;
  margin-bottom: 0.6rem;
}

.ssg-page-desc {
  font-size: 0.95rem;
  line-height: 1.7;
  opacity: 0.75;
  max-width: 42rem;
  margin-bottom: 0.75rem;
}

.ssg-page-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.ssg-tag {
  border-radius: 999px;
  border: 1px solid rgba(255,255,255,0.1);
  background: rgba(255,255,255,0.05);
  padding: 0.2rem 0.7rem;
  font-size: 0.75rem;
}

.ssg-tag-status {
  border-color: rgba(137,180,250,0.3);
  background: rgba(137,180,250,0.1);
  color: #bdd5ff;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

article.docs-content {
  border: 1px solid rgba(255,255,255,0.08);
  background: rgba(0,0,0,0.1);
  border-radius: 1.5rem;
  padding: 1.5rem 2rem 2rem;
}
"#
}

fn generate_builder_css() -> &'static str {
    r#"
:root {
  --emark-bg: #11111b;
  --emark-surface: #181825;
  --emark-surface-2: #313244;
  --emark-border: #45475a;
  --emark-text: #cdd6f4;
  --emark-muted: #a6adc8;
  --emark-blue: #89b4fa;
  --emark-green: #a6e3a1;
  --emark-yellow: #f9e2af;
  --emark-red: #f38ba8;
  --emark-mauve: #cba6f7;
  --emark-peach: #fab387;
  --emark-teal: #94e2d5;
}

.emark-page {
  color: var(--emark-text);
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.emark-text-plain {
  color: var(--emark-muted);
  line-height: 1.8;
}

.emark-toc {
  border: 1px solid var(--emark-border);
  border-radius: 12px;
  background: rgba(24, 24, 37, 0.7);
}

.emark-toc summary {
  cursor: pointer;
  padding: 0.85rem 1rem;
  font-weight: 600;
}

.emark-toc nav {
  padding: 0 1rem 1rem 1rem;
}

.emark-toc ul {
  list-style: none;
  margin: 0;
  padding-left: 1rem;
}

.emark-toc > nav > ul {
  padding-left: 0;
}

.emark-toc li {
  margin: 0.35rem 0;
}

.emark-toc a {
  color: var(--emark-blue);
  text-decoration: none;
}

.emark-toc a:hover {
  text-decoration: underline;
}

.emark-section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.emark-section-heading {
  color: var(--emark-mauve);
  line-height: 1.25;
  scroll-margin-top: 4rem;
}

.emark-section-h2 > .emark-section-heading {
  font-size: 1.7rem;
}

.emark-section-h3 > .emark-section-heading {
  font-size: 1.35rem;
}

.emark-markdown {
  line-height: 1.8;
}

.emark-markdown h1,
.emark-markdown h2,
.emark-markdown h3,
.emark-markdown h4,
.emark-markdown h5,
.emark-markdown h6 {
  color: var(--emark-mauve);
  margin: 1.5rem 0 0.75rem;
}

.emark-markdown p,
.emark-markdown ul,
.emark-markdown ol,
.emark-markdown blockquote,
.emark-markdown table {
  margin: 1rem 0;
}

.emark-markdown ul,
.emark-markdown ol {
  padding-left: 1.75rem;
}

.emark-markdown li {
  margin: 0.4rem 0;
}

.emark-markdown code {
  background: rgba(49, 50, 68, 0.9);
  color: var(--emark-peach);
  border-radius: 6px;
  padding: 0.15rem 0.35rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.92em;
}

.emark-markdown pre {
  margin: 1rem 0;
}

.emark-markdown pre code {
  background: transparent;
  padding: 0;
}

.emark-markdown a {
  color: var(--emark-blue);
}

.emark-markdown table {
  width: 100%;
  border-collapse: collapse;
}

.emark-markdown th,
.emark-markdown td {
  border: 1px solid var(--emark-border);
  padding: 0.65rem 0.75rem;
  text-align: left;
  vertical-align: top;
}

.emark-markdown th {
  background: rgba(49, 50, 68, 0.8);
}

.emark-markdown blockquote {
  border-left: 3px solid var(--emark-mauve);
  padding-left: 1rem;
  color: var(--emark-muted);
}

.emark-code-block,
.emark-pre-plain,
.emark-eure-source {
  position: relative;
  overflow-x: auto;
  padding: 1rem;
  border-radius: 14px;
  background: var(--emark-surface);
  border: 1px solid var(--emark-border);
  box-shadow: 0 8px 30px rgba(17, 17, 27, 0.35);
  line-height: 1.6;
}

.emark-code-block[data-language] {
  padding-top: 2rem;
}

.emark-code-block[data-language]::before {
  content: attr(data-language);
  position: absolute;
  top: 0;
  left: 0;
  padding: 0.2rem 0.6rem;
  border-radius: 14px 0 10px 0;
  background: var(--emark-surface-2);
  color: var(--emark-muted);
  font-size: 0.75rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.emark-alert {
  border-left: 4px solid var(--emark-blue);
  border-radius: 10px;
  background: rgba(49, 50, 68, 0.72);
  padding: 0.9rem 1rem;
}

.emark-alert-title {
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.emark-alert-tip {
  border-left-color: var(--emark-green);
}

.emark-alert-important {
  border-left-color: var(--emark-mauve);
}

.emark-alert-warning {
  border-left-color: var(--emark-yellow);
}

.emark-alert-caution {
  border-left-color: var(--emark-red);
}

.emark-eure-source {
  color: var(--emark-text);
}

.edoc-page {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.edoc-intro {
  color: var(--emark-muted);
  line-height: 1.8;
}

.edoc-card-list {
  display: grid;
  gap: 1rem;
}

.edoc-card {
  border: 1px solid var(--emark-border);
  border-radius: 14px;
  background: rgba(24, 24, 37, 0.74);
  padding: 1rem 1.1rem;
}

.edoc-card-title {
  margin: 0 0 0.35rem;
}

.edoc-card-title a,
.edoc-link-list a {
  color: var(--emark-blue);
  text-decoration: none;
}

.edoc-card-title a:hover,
.edoc-link-list a:hover {
  text-decoration: underline;
}

.edoc-card-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  color: var(--emark-muted);
  font-size: 0.95rem;
}

.edoc-tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  margin-top: 0.75rem;
}

.edoc-tag {
  border-radius: 999px;
  background: rgba(69, 71, 90, 0.9);
  color: var(--emark-text);
  font-size: 0.8rem;
  padding: 0.2rem 0.55rem;
}

.edoc-adr-meta {
  border: 1px solid var(--emark-border);
  border-radius: 14px;
  background: rgba(24, 24, 37, 0.78);
  padding: 1rem 1.1rem;
}

.edoc-adr-meta-grid {
  display: grid;
  grid-template-columns: minmax(9rem, auto) 1fr;
  gap: 0.5rem 1rem;
  margin: 0;
}

.edoc-adr-meta-grid dt {
  color: var(--emark-muted);
  font-weight: 600;
}

.edoc-adr-meta-grid dd {
  margin: 0;
}

.edoc-adr-status-detail {
  border-left: 4px solid var(--emark-peach);
  border-radius: 10px;
  background: rgba(49, 50, 68, 0.72);
  padding: 0.9rem 1rem;
}

.edoc-adr-section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.edoc-link-list {
  margin: 0;
  padding-left: 1.5rem;
}

.edoc-link-list li {
  margin: 0.45rem 0;
}
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_docs_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("eure-doc-builder-{name}-{unique}"))
    }

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent directories");
        }
        fs::write(path, contents).expect("failed to write file");
    }

    #[test]
    fn build_docs_site_renders_guide_and_adr_pages() {
        let root = temp_docs_dir("render");
        write_file(
            &root.join("_nav.eure"),
            r####"
title = "Documentation"

groups[] {
  title = "Docs"
  pages[] { path = "/docs/" label = "Home" }
  pages[] { path = "/docs/guide" label = "Guide" }
}
"####,
        );
        write_file(
            &root.join("index.eure"),
            r####"
$frontmatter {
  title = "Home"
  description = "Home page"
}

"#": Home

"body" = ```markdown
See the [guide](/docs/guide).
```
"####,
        );
        write_file(
            &root.join("guide.eure"),
            r####"
$frontmatter {
  title = "Guide"
  description = "Guide page"
  tags = ["example"]
}

"#": Guide

"toc".$toc = true

@ "overview" {
  "##": Overview

  "body" = ```markdown
Guide intro.
```

  "note" = ```markdown
Important note.
```
  "note".$mark.alert: NOTE

  "rust-example" = ```rust
fn main() {}
```
}
"####,
        );
        write_file(
            &root.join("adrs/0001-example.eure"),
            r####"
$schema: ../../crates/eure-doc-builder/assets/adr.schema.eure

id = "0001-example"
title = "Example ADR"
status = "accepted"
decision-date = `2026-04-21`
tags = ["architecture"]
related-adrs = ["0001-example"]
related-links = ["https://example.com/adr"]
authors = ["Eure Team"]

context = ```markdown
Context body.
```

decision = ```markdown
Decision body.
```

consequences = ```markdown
Consequences body.
```

alternatives-considered = [
  ```markdown
  Alternative body.
  ```
]
"####,
        );

        let site = build_docs_site(&root).expect("docs site should build");
        let guide = site
            .pages
            .iter()
            .find(|page| page.public_path == "/docs/guide")
            .expect("guide page should exist");
        assert!(guide.html.contains("emark-alert"));
        assert!(guide.css.contains("emark-hl-"));
        assert!(guide.html.contains("On This Page"));

        let adr_page = site
            .pages
            .iter()
            .find(|page| page.public_path == "/docs/adrs/0001-example")
            .expect("adr page should exist");
        assert!(adr_page.html.contains("Related ADRs"));
        assert!(adr_page.html.contains("https://example.com/adr"));

        let adr_index = site
            .pages
            .iter()
            .find(|page| page.public_path == "/docs/adrs")
            .expect("adr index should exist");
        assert!(adr_index.html.contains("Example ADR"));
    }

    #[test]
    fn generate_llms_txt_produces_correct_format() {
        let site = DocsSite {
            nav: DocsNav {
                title: "Eure".to_string(),
                groups: vec![DocsNavGroup {
                    title: "Getting Started".to_string(),
                    description: None,
                    entries: vec![
                        DocsNavEntry {
                            path: "/docs/".to_string(),
                            label: "Home".to_string(),
                        },
                        DocsNavEntry {
                            path: "/docs/guide".to_string(),
                            label: "Guide".to_string(),
                        },
                    ],
                }],
            },
            pages: vec![
                RenderedDocsPage {
                    public_path: "/docs/".to_string(),
                    title: "Home".to_string(),
                    description: "A minimalist data format.".to_string(),
                    html: String::new(),
                    css: String::new(),
                    kind: DocsPageKind::Guide,
                    headings: vec![],
                    tags: vec![],
                    status: None,
                    decision_date: None,
                },
                RenderedDocsPage {
                    public_path: "/docs/guide".to_string(),
                    title: "Guide".to_string(),
                    description: "How to use Eure.".to_string(),
                    html: String::new(),
                    css: String::new(),
                    kind: DocsPageKind::Guide,
                    headings: vec![],
                    tags: vec![],
                    status: None,
                    decision_date: None,
                },
            ],
            adrs: vec![AdrSummary {
                path: "/docs/adrs/0001-example".to_string(),
                title: "Example Decision".to_string(),
                status: "accepted".to_string(),
                decision_date: "2024-01-01".to_string(),
                tags: vec![],
            }],
        };

        let output = generate_llms_txt(&site, "https://eure.dev");

        assert!(output.starts_with("# Eure\n"));
        assert!(output.contains("> A minimalist data format.\n"));
        assert!(output.contains("## Getting Started\n"));
        assert!(output.contains("- [Home](https://eure.dev/docs/): A minimalist data format.\n"));
        assert!(output.contains("- [Guide](https://eure.dev/docs/guide): How to use Eure.\n"));
        assert!(output.contains("## Optional\n"));
        assert!(output.contains(
            "- [Example Decision](https://eure.dev/docs/adrs/0001-example): 2024-01-01, accepted\n"
        ));
    }

    #[test]
    fn build_docs_site_rejects_missing_links() {
        let root = temp_docs_dir("links");
        write_file(
            &root.join("_nav.eure"),
            r####"
groups[] {
  title = "Docs"
  pages[] { path = "/docs/" label = "Home" }
}
"####,
        );
        write_file(
            &root.join("index.eure"),
            r####"
$frontmatter {
  title = "Home"
  description = "Home page"
}

"#": Home

"body" = ```markdown
Broken [link](/docs/missing).
```
"####,
        );

        let error = build_docs_site(&root).expect_err("missing link should fail");
        assert!(matches!(
            error,
            DocsError::MissingLinkedPage { target, .. } if target == "/docs/missing"
        ));
    }
}
