//! Docs site models and rendering.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use eure::FromEure;
use eure::query::{SemanticToken, SemanticTokenModifier, SemanticTokenType, semantic_tokens};
use eure::value::{Language, Text};
use eure_document::map::Map;
use giallo::{FontStyle, HighlightOptions, HtmlRenderer, Registry, RenderOptions, ThemeVariant};
use markdown::{CompileOptions, Options};
use maud::{Markup, PreEscaped, html};
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocsPageKind {
    Guide,
    Adr,
    AdrIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsHeading {
    pub id: String,
    pub title: String,
    pub level: u8,
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

    #[error("failed to compile markdown to HTML: {message}")]
    MarkdownRender { message: String },

    #[error("failed to initialize syntax highlighter: {source}")]
    HighlighterInit {
        #[source]
        source: giallo::Error,
    },

    #[error("failed to generate syntax CSS: {source}")]
    SyntaxCss {
        #[source]
        source: giallo::Error,
    },

    #[error("failed to highlight {language} code: {source}")]
    Highlight {
        language: String,
        #[source]
        source: giallo::Error,
    },

    #[error("trusted HTML blocks must use the html language tag")]
    TrustedHtmlMustUseHtmlLanguage,

    #[error("nav entry {path} is listed more than once")]
    DuplicateNavEntry { path: String },

    #[error("guide page contains a duplicate section id {id}")]
    DuplicateSectionId { id: String },

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromEure)]
#[eure(crate = ::eure::document)]
pub enum AlertType {
    #[eure(rename = "NOTE")]
    Note,
    #[eure(rename = "TIP")]
    Tip,
    #[eure(rename = "IMPORTANT")]
    Important,
    #[eure(rename = "WARNING")]
    Warning,
    #[eure(rename = "CAUTION")]
    Caution,
}

#[derive(Debug, Clone, PartialEq, FromEure, Default)]
#[eure(crate = ::eure::document)]
pub struct MarkOptions {
    #[eure(default)]
    pub alert: Option<AlertType>,
    #[eure(rename = "dangerously-inner-html", default)]
    pub dangerously_inner_html: bool,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct GuideDocument {
    #[eure(ext)]
    pub docs: DocsFrontmatter,
    #[eure(rename = "#")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<TextOrNested<Level2>>>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct DocsFrontmatter {
    pub title: Text,
    pub description: Text,
    #[eure(default)]
    pub order: Option<i64>,
    #[eure(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub enum Item<T> {
    Normal(T),
    List(Vec<T>),
    Toc(Toc),
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Toc {
    #[eure(ext)]
    pub toc: bool,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub enum TextOrNested<T> {
    Text {
        #[eure(flatten)]
        text: Text,
        #[eure(ext, default)]
        mark: MarkOptions,
    },
    Nested(T),
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Level2 {
    #[eure(rename = "##")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<TextOrNested<Level3>>>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Level3 {
    #[eure(rename = "###")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<TextOrNested<Level4>>>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Level4 {
    #[eure(rename = "####")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<TextOrNested<Level5>>>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Level5 {
    #[eure(rename = "#####")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<TextOrNested<Level6>>>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Level6 {
    #[eure(rename = "######")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<Text>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TocEntry {
    id: String,
    title: String,
    level: u8,
    children: Vec<TocEntry>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct AdrDocument {
    #[eure(ext, default)]
    pub schema: Option<String>,
    pub id: Text,
    pub title: Text,
    pub status: Text,
    #[eure(rename = "decision-date")]
    pub decision_date: Text,
    #[eure(default)]
    pub tags: Vec<String>,
    #[eure(rename = "related-adrs", default)]
    pub related_adrs: Vec<String>,
    #[eure(rename = "related-links", default)]
    pub related_links: Vec<String>,
    #[eure(default)]
    pub authors: Vec<String>,
    pub context: Text,
    pub decision: Text,
    pub consequences: Text,
    #[eure(rename = "alternatives-considered", default)]
    pub alternatives_considered: Vec<Text>,
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
    let highlighter = CodeHighlighter::new()?;
    let shared_css = generate_docs_css(&highlighter)?;

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
        let doc = parse_guide_document(&file)?;
        let rendered = render_guide_page(&doc, &public_path, &shared_css, &highlighter)?;
        guide_paths.insert(rendered.public_path.clone());
        pages.push(rendered);
    }

    if !found_docs_index {
        return Err(DocsError::MissingDocsIndex);
    }

    let mut adr_pages = Vec::new();
    for file in adr_files {
        let public_path = public_path_for_file(docs_dir, &file);
        let doc = parse_adr_document(&file)?;
        let rendered = render_adr_page(&doc, &public_path, &shared_css, &highlighter)?;
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

    let adr_index = render_adr_index_page(&adr_summaries, &shared_css)?;
    pages.push(adr_index);
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
    let groups = nav
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
        .collect();
    Ok(DocsNav {
        title: nav.title.unwrap_or_else(|| "Documentation".to_string()),
        groups,
    })
}

fn parse_guide_document(path: &Path) -> Result<GuideDocument, DocsError> {
    parse_eure_file(path)
}

fn parse_adr_document(path: &Path) -> Result<AdrDocument, DocsError> {
    parse_eure_file(path)
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
    let cst = eure::parol::parse(&source).map_err(|source| DocsError::ParseEure {
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

fn render_guide_page(
    doc: &GuideDocument,
    public_path: &str,
    css: &str,
    highlighter: &CodeHighlighter,
) -> Result<RenderedDocsPage, DocsError> {
    let mut seen_ids = HashSet::new();
    let toc_entries = collect_toc_entries(&doc.sections, &mut seen_ids)?;
    let headings = flatten_toc_entries(&toc_entries);

    let mut rendered_items = Vec::new();
    for (key, item) in doc.sections.iter() {
        rendered_items.push(render_item_with_id(key, item, highlighter, &toc_entries)?);
    }

    let body = html! {
        article class="docs-page docs-page-guide" {
            @for item in &rendered_items {
                (item)
            }
        }
    };

    Ok(RenderedDocsPage {
        public_path: public_path.to_string(),
        title: doc.docs.title.as_str().to_string(),
        description: doc.docs.description.as_str().to_string(),
        html: body.into_string(),
        css: css.to_string(),
        kind: DocsPageKind::Guide,
        headings,
        tags: doc.docs.tags.clone(),
        status: None,
        decision_date: None,
    })
}

fn render_adr_page(
    doc: &AdrDocument,
    public_path: &str,
    css: &str,
    highlighter: &CodeHighlighter,
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

    let alternatives = if doc.alternatives_considered.is_empty() {
        None
    } else {
        let mut rendered = Vec::new();
        for alternative in &doc.alternatives_considered {
            rendered.push(render_block_text(alternative, highlighter)?);
        }
        Some(rendered)
    };

    let body = html! {
        article class="docs-page docs-page-adr" {
            section class="docs-adr-meta" {
                dl class="docs-adr-meta-grid" {
                    dt { "ID" }
                    dd { (doc.id.as_str()) }
                    dt { "Status" }
                    dd { (doc.status.as_str()) }
                    dt { "Decision Date" }
                    dd { (doc.decision_date.as_str()) }
                    @if !doc.tags.is_empty() {
                        dt { "Tags" }
                        dd {
                            div class="docs-tag-list" {
                                @for tag in &doc.tags {
                                    span class="docs-tag" { (tag) }
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

            section class="docs-adr-section" {
                h2 id="context" { "Context" }
                (render_block_text(&doc.context, highlighter)?)
            }

            section class="docs-adr-section" {
                h2 id="decision" { "Decision" }
                (render_block_text(&doc.decision, highlighter)?)
            }

            section class="docs-adr-section" {
                h2 id="consequences" { "Consequences" }
                (render_block_text(&doc.consequences, highlighter)?)
            }

            @if let Some(alternatives) = &alternatives {
                section class="docs-adr-section" {
                    h2 id="alternatives-considered" { "Alternatives Considered" }
                    @for alternative in alternatives {
                        div class="docs-adr-alternative" {
                            (alternative)
                        }
                    }
                }
            }

            @if !doc.related_adrs.is_empty() {
                section class="docs-adr-section" {
                    h2 id="related-adrs" { "Related ADRs" }
                    ul class="docs-link-list" {
                        @for related_adr in &doc.related_adrs {
                            li {
                                a href=(format!("/docs/adrs/{}", related_adr)) { (related_adr) }
                            }
                        }
                    }
                }
            }

            @if !doc.related_links.is_empty() {
                section class="docs-adr-section" {
                    h2 id="related-links" { "Related Links" }
                    ul class="docs-link-list" {
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
        status: Some(doc.status.as_str().to_string()),
        decision_date: Some(doc.decision_date.as_str().to_string()),
    })
}

fn render_adr_index_page(adrs: &[AdrSummary], css: &str) -> Result<RenderedDocsPage, DocsError> {
    let body = html! {
        article class="docs-page docs-page-adr-index" {
            p class="docs-intro" {
                "Architecture decision records capture notable language and implementation choices."
            }
            div class="docs-card-list" {
                @for adr in adrs {
                    article class="docs-card" {
                        h2 class="docs-card-title" {
                            a href=(adr.path.as_str()) { (adr.title.as_str()) }
                        }
                        div class="docs-card-meta" {
                            span { (adr.status.as_str()) }
                            span { "•" }
                            span { (adr.decision_date.as_str()) }
                        }
                        @if !adr.tags.is_empty() {
                            div class="docs-tag-list" {
                                @for tag in &adr.tags {
                                    span class="docs-tag" { (tag) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(RenderedDocsPage {
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
    })
}

fn collect_toc_entries<T: CollectToc>(
    sections: &Map<String, Item<TextOrNested<T>>>,
    seen_ids: &mut HashSet<String>,
) -> Result<Vec<TocEntry>, DocsError> {
    let mut entries = Vec::new();

    for (id, item) in sections.iter() {
        match item {
            Item::Normal(value) => {
                if let Some(entry) = value.collect_toc_entry(id, seen_ids)? {
                    entries.push(entry);
                }
            }
            Item::List(items) => {
                for value in items {
                    if let Some(entry) = value.collect_toc_entry(id, seen_ids)? {
                        entries.push(entry);
                    }
                }
            }
            Item::Toc(_) => {}
        }
    }

    Ok(entries)
}

fn flatten_toc_entries(entries: &[TocEntry]) -> Vec<DocsHeading> {
    let mut headings = Vec::new();
    flatten_toc_entries_into(entries, &mut headings);
    headings
}

fn flatten_toc_entries_into(entries: &[TocEntry], headings: &mut Vec<DocsHeading>) {
    for entry in entries {
        headings.push(DocsHeading {
            id: entry.id.clone(),
            title: entry.title.clone(),
            level: entry.level,
        });
        flatten_toc_entries_into(&entry.children, headings);
    }
}

trait CollectToc {
    fn collect_toc_entry(
        &self,
        id: &str,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Option<TocEntry>, DocsError>;
}

impl CollectToc for Text {
    fn collect_toc_entry(
        &self,
        _id: &str,
        _seen_ids: &mut HashSet<String>,
    ) -> Result<Option<TocEntry>, DocsError> {
        Ok(None)
    }
}

impl<T: CollectToc> CollectToc for TextOrNested<T> {
    fn collect_toc_entry(
        &self,
        id: &str,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Option<TocEntry>, DocsError> {
        match self {
            TextOrNested::Text { .. } => Ok(None),
            TextOrNested::Nested(nested) => nested.collect_toc_entry(id, seen_ids),
        }
    }
}

macro_rules! impl_collect_toc_for_level {
    ($ty:ty, $level:expr, $include_in_toc:expr) => {
        impl CollectToc for $ty {
            fn collect_toc_entry(
                &self,
                id: &str,
                seen_ids: &mut HashSet<String>,
            ) -> Result<Option<TocEntry>, DocsError> {
                if !seen_ids.insert(id.to_string()) {
                    return Err(DocsError::DuplicateSectionId { id: id.to_string() });
                }

                let children = collect_toc_entries(&self.sections, seen_ids)?;
                if $include_in_toc {
                    Ok(Some(TocEntry {
                        id: id.to_string(),
                        title: self.header.as_str().to_string(),
                        level: $level,
                        children,
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    };
}

impl_collect_toc_for_level!(Level2, 2, true);
impl_collect_toc_for_level!(Level3, 3, true);
impl_collect_toc_for_level!(Level4, 4, false);
impl_collect_toc_for_level!(Level5, 5, false);

impl CollectToc for Level6 {
    fn collect_toc_entry(
        &self,
        id: &str,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Option<TocEntry>, DocsError> {
        if !seen_ids.insert(id.to_string()) {
            return Err(DocsError::DuplicateSectionId { id: id.to_string() });
        }
        Ok(None)
    }
}

fn render_item_with_id<T: RenderNestedWithId>(
    key: &str,
    item: &Item<T>,
    highlighter: &CodeHighlighter,
    toc_entries: &[TocEntry],
) -> Result<Markup, DocsError> {
    match item {
        Item::Normal(value) => Ok(html! {
            div class="docs-content-item" data-key=(key) {
                (value.render_with_id(key, highlighter, toc_entries)?)
            }
        }),
        Item::List(items) => {
            let mut rendered_items = Vec::new();
            for value in items {
                rendered_items.push(value.render_with_id(key, highlighter, toc_entries)?);
            }
            Ok(html! {
                div class="docs-content-list" data-key=(key) {
                    @for value in &rendered_items {
                        div class="docs-content-list-item" {
                            (value)
                        }
                    }
                }
            })
        }
        Item::Toc(_) => Ok(render_toc(toc_entries)),
    }
}

fn render_toc(entries: &[TocEntry]) -> Markup {
    if entries.is_empty() {
        html! {}
    } else {
        html! {
            details class="docs-toc" open {
                summary { "On This Page" }
                nav {
                    (render_toc_list(entries))
                }
            }
        }
    }
}

fn render_toc_list(entries: &[TocEntry]) -> Markup {
    html! {
        ul {
            @for entry in entries {
                li {
                    a href=(format!("#{}", entry.id)) { (entry.title.as_str()) }
                    @if !entry.children.is_empty() {
                        (render_toc_list(&entry.children))
                    }
                }
            }
        }
    }
}

trait RenderNestedWithId {
    fn render_with_id(
        &self,
        id: &str,
        highlighter: &CodeHighlighter,
        toc_entries: &[TocEntry],
    ) -> Result<Markup, DocsError>;
}

impl RenderNestedWithId for Text {
    fn render_with_id(
        &self,
        _id: &str,
        highlighter: &CodeHighlighter,
        _toc_entries: &[TocEntry],
    ) -> Result<Markup, DocsError> {
        render_block_text(self, highlighter)
    }
}

impl<T: RenderNestedWithId> RenderNestedWithId for TextOrNested<T> {
    fn render_with_id(
        &self,
        id: &str,
        highlighter: &CodeHighlighter,
        toc_entries: &[TocEntry],
    ) -> Result<Markup, DocsError> {
        match self {
            TextOrNested::Text { text, mark } => render_text_with_mark(text, mark, highlighter),
            TextOrNested::Nested(nested) => nested.render_with_id(id, highlighter, toc_entries),
        }
    }
}

macro_rules! impl_render_for_level {
    ($ty:ty, $tag:literal) => {
        impl RenderNestedWithId for $ty {
            fn render_with_id(
                &self,
                id: &str,
                highlighter: &CodeHighlighter,
                toc_entries: &[TocEntry],
            ) -> Result<Markup, DocsError> {
                render_section_with_id(
                    id,
                    self.header.as_str(),
                    &self.sections,
                    $tag,
                    highlighter,
                    toc_entries,
                )
            }
        }
    };
}

impl_render_for_level!(Level2, "h2");
impl_render_for_level!(Level3, "h3");
impl_render_for_level!(Level4, "h4");
impl_render_for_level!(Level5, "h5");
impl_render_for_level!(Level6, "h6");

fn render_section_with_id<T: RenderNestedWithId>(
    id: &str,
    header: &str,
    sections: &Map<String, Item<T>>,
    level: &str,
    highlighter: &CodeHighlighter,
    toc_entries: &[TocEntry],
) -> Result<Markup, DocsError> {
    let mut rendered_items = Vec::new();
    for (key, item) in sections.iter() {
        rendered_items.push(render_item_with_id(key, item, highlighter, toc_entries)?);
    }

    Ok(html! {
        section class=(format!("docs-section docs-section-{}", level)) {
            @match level {
                "h2" => h2 class="docs-section-heading" id=(id) { (header) },
                "h3" => h3 class="docs-section-heading" id=(id) { (header) },
                "h4" => h4 class="docs-section-heading" id=(id) { (header) },
                "h5" => h5 class="docs-section-heading" id=(id) { (header) },
                "h6" => h6 class="docs-section-heading" id=(id) { (header) },
                _ => h2 class="docs-section-heading" id=(id) { (header) },
            }
            @for item in &rendered_items {
                (item)
            }
        }
    })
}

fn render_text_with_mark(
    text: &Text,
    mark: &MarkOptions,
    highlighter: &CodeHighlighter,
) -> Result<Markup, DocsError> {
    let content = if mark.dangerously_inner_html {
        if !text.language.is_other("html") {
            return Err(DocsError::TrustedHtmlMustUseHtmlLanguage);
        }
        html! { div class="docs-markdown-content" { (PreEscaped(text.as_str().to_string())) } }
    } else {
        render_block_text(text, highlighter)?
    };

    if let Some(alert) = mark.alert {
        let alert_title = match alert {
            AlertType::Note => "Note",
            AlertType::Tip => "Tip",
            AlertType::Important => "Important",
            AlertType::Warning => "Warning",
            AlertType::Caution => "Caution",
        };
        let alert_class = match alert {
            AlertType::Note => "note",
            AlertType::Tip => "tip",
            AlertType::Important => "important",
            AlertType::Warning => "warning",
            AlertType::Caution => "caution",
        };
        Ok(html! {
            div class=(format!("docs-alert docs-alert-{}", alert_class)) {
                div class="docs-alert-title" { (alert_title) }
                div class="docs-alert-body" { (content) }
            }
        })
    } else {
        Ok(content)
    }
}

fn render_block_text(text: &Text, highlighter: &CodeHighlighter) -> Result<Markup, DocsError> {
    match &text.language {
        Language::Plaintext => Ok(html! { p class="docs-text-plain" { (text.as_str()) } }),
        Language::Implicit => {
            Ok(html! { pre class="docs-pre docs-pre-plain" { code { (text.as_str()) } } })
        }
        Language::Other(language) => match language.as_ref() {
            "markdown" => render_markdown(text.as_str()),
            "html" => Ok(
                html! { div class="docs-markdown-content" { (PreEscaped(text.as_str().to_string())) } },
            ),
            "eure" => Ok(render_eure_highlighted(text.as_str(), highlighter, false)),
            "ebnf" | "pkl" => Ok(render_plain_code_block(text.as_str(), language)),
            other => highlighter.highlight(text.as_str(), other),
        },
    }
}

fn render_plain_code_block(content: &str, language: &str) -> Markup {
    html! {
        pre class="docs-code-block docs-pre-plain" data-language=(format_language_name(language)) {
            code { (content) }
        }
    }
}

fn render_markdown(content: &str) -> Result<Markup, DocsError> {
    let options = Options {
        compile: CompileOptions {
            allow_dangerous_html: true,
            ..CompileOptions::default()
        },
        ..Options::gfm()
    };
    let html_output = markdown::to_html_with_options(content, &options).map_err(|message| {
        DocsError::MarkdownRender {
            message: message.to_string(),
        }
    })?;
    Ok(html! {
        div class="docs-markdown-content" {
            (PreEscaped(html_output))
        }
    })
}

fn summarize_text(text: &Text) -> String {
    let summary = text.as_str().trim().replace('\n', " ");
    summary.chars().take(180).collect()
}

pub(crate) struct CodeHighlighter {
    registry: Registry,
}

impl CodeHighlighter {
    fn new() -> Result<Self, DocsError> {
        let mut registry =
            Registry::builtin().map_err(|source| DocsError::HighlighterInit { source })?;
        registry.link_grammars();
        Ok(Self { registry })
    }

    fn generate_css(&self) -> Result<String, DocsError> {
        self.registry
            .generate_css("catppuccin-mocha", "docs-hl-")
            .map_err(|source| DocsError::SyntaxCss { source })
    }

    fn highlight(&self, code: &str, language: &str) -> Result<Markup, DocsError> {
        let options = HighlightOptions::new(language, ThemeVariant::Single("catppuccin-mocha"));
        let highlighted =
            self.registry
                .highlight(code, &options)
                .map_err(|source| DocsError::Highlight {
                    language: language.to_string(),
                    source,
                })?;
        let renderer = HtmlRenderer {
            css_class_prefix: Some("docs-hl-".to_string()),
            ..Default::default()
        };
        let html_output = renderer.render(&highlighted, &RenderOptions::default());
        let html_with_badge = html_output.replacen(
            "<pre class=\"giallo docs-hl-code\">",
            &format!(
                "<pre class=\"docs-code-block giallo docs-hl-code\" data-language=\"{}\">",
                format_language_name(language)
            ),
            1,
        );
        Ok(html! { (PreEscaped(html_with_badge)) })
    }

    fn highlight_line(&self, line: &str, language: &str) -> Option<Markup> {
        let options = HighlightOptions::new(language, ThemeVariant::Single("catppuccin-mocha"));
        let highlighted = self.registry.highlight(line, &options).ok()?;
        Some(html! {
            @for line_tokens in &highlighted.tokens {
                @for token in line_tokens {
                    @if let ThemeVariant::Single(style) = &token.style {
                        @let color = style.foreground.as_hex();
                        @let is_bold = style.font_style.contains(FontStyle::BOLD);
                        @let is_italic = style.font_style.contains(FontStyle::ITALIC);
                        @if is_bold {
                            span style=(format!("color:{};font-weight:bold", color)) { (token.text) }
                        } @else if is_italic {
                            span style=(format!("color:{};font-style:italic", color)) { (token.text) }
                        } @else {
                            span style=(format!("color:{}", color)) { (token.text) }
                        }
                    } @else {
                        (token.text)
                    }
                }
            }
        })
    }
}

fn render_eure_highlighted(
    content: &str,
    highlighter: &CodeHighlighter,
    with_line_numbers: bool,
) -> Markup {
    let cst = match eure::parol::parse_tolerant(content) {
        eure::parol::ParseResult::Ok(cst) => cst,
        eure::parol::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    let tokens = semantic_tokens(content, &cst);
    let code_blocks = find_code_block_regions(content);
    if with_line_numbers {
        html! {
            pre class="docs-eure-source docs-eure-source-with-lines" {
                code { (render_tokens_by_line(content, &tokens, &code_blocks, highlighter)) }
            }
        }
    } else {
        html! {
            pre class="docs-code-block docs-eure-source" data-language="Eure" {
                code { (render_tokens(content, &tokens, &code_blocks, highlighter)) }
            }
        }
    }
}

struct CodeBlockRegion {
    content_start: usize,
    content_end: usize,
    language: String,
}

fn find_code_block_regions(content: &str) -> Vec<CodeBlockRegion> {
    let mut regions = Vec::new();
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut index = 0;

    while index < len {
        if index + 3 <= len && &bytes[index..index + 3] == b"```" {
            let start = index;
            let mut backtick_count = 3;
            while index + backtick_count < len
                && bytes[index + backtick_count] == b'`'
                && backtick_count < 6
            {
                backtick_count += 1;
            }

            let after_backticks = index + backtick_count;
            let mut lang_end = after_backticks;
            while lang_end < len
                && bytes[lang_end] != b'\n'
                && !bytes[lang_end].is_ascii_whitespace()
            {
                lang_end += 1;
            }
            let language = content[after_backticks..lang_end].to_string();

            let mut newline_pos = lang_end;
            while newline_pos < len && bytes[newline_pos] != b'\n' {
                newline_pos += 1;
            }

            if newline_pos < len {
                let content_start = newline_pos + 1;
                let closing_pattern = &content[start..start + backtick_count];
                if let Some(relative_close) = content[content_start..].find(closing_pattern) {
                    let content_end = content_start + relative_close;
                    regions.push(CodeBlockRegion {
                        content_start,
                        content_end,
                        language,
                    });
                    index = content_end + backtick_count;
                    continue;
                }
            }
        }
        index += 1;
    }

    regions
}

fn find_code_block_for_range(
    gap_start: usize,
    gap_end: usize,
    regions: &[CodeBlockRegion],
) -> Option<&CodeBlockRegion> {
    regions
        .iter()
        .find(|region| gap_start < region.content_end && gap_end > region.content_start)
}

fn render_tokens_to_string(
    content: &str,
    tokens: &[SemanticToken],
    code_blocks: &[CodeBlockRegion],
    highlighter: &CodeHighlighter,
) -> String {
    let mut html_output = String::new();
    let mut last_end = 0usize;

    for token in tokens {
        let start = token.start as usize;
        let end = start + token.length as usize;

        if start > last_end {
            let gap_text = &content[last_end..start];
            if let Some(region) = find_code_block_for_range(last_end, start, code_blocks) {
                if !region.language.is_empty() {
                    let content_in_gap_start = region.content_start.max(last_end);
                    let content_in_gap_end = region.content_end.min(start);
                    if last_end < content_in_gap_start {
                        html_output
                            .push_str(&escape_html(&content[last_end..content_in_gap_start]));
                    }
                    let code_content = &content[content_in_gap_start..content_in_gap_end];
                    if region.language == "eure" {
                        html_output.push_str(&render_eure_tokens_only(code_content));
                    } else {
                        for (line_index, line) in code_content.split('\n').enumerate() {
                            if line_index > 0 {
                                html_output.push('\n');
                            }
                            if let Some(highlighted) =
                                highlighter.highlight_line(line, &region.language)
                            {
                                html_output.push_str(&highlighted.into_string());
                            } else {
                                html_output.push_str(&escape_html(line));
                            }
                        }
                    }
                    if content_in_gap_end < start {
                        html_output.push_str(&escape_html(&content[content_in_gap_end..start]));
                    }
                } else {
                    html_output.push_str(&escape_html(gap_text));
                }
            } else {
                html_output.push_str(&escape_html(gap_text));
            }
        }

        let token_text = &content[start..end];
        let classes = build_eure_classes(token);
        html_output.push_str(&format!(
            "<span class=\"{}\">{}</span>",
            classes,
            escape_html(token_text)
        ));
        last_end = end;
    }

    if last_end < content.len() {
        html_output.push_str(&escape_html(&content[last_end..]));
    }

    html_output
}

fn render_tokens(
    content: &str,
    tokens: &[SemanticToken],
    code_blocks: &[CodeBlockRegion],
    highlighter: &CodeHighlighter,
) -> Markup {
    html! { (PreEscaped(render_tokens_to_string(content, tokens, code_blocks, highlighter))) }
}

fn render_tokens_by_line(
    content: &str,
    tokens: &[SemanticToken],
    code_blocks: &[CodeBlockRegion],
    highlighter: &CodeHighlighter,
) -> Markup {
    let html_output = render_tokens_to_string(content, tokens, code_blocks, highlighter);
    let result: String = html_output
        .split('\n')
        .map(|line| format!("<span class=\"docs-line\">{}</span>", line))
        .collect();
    html! { (PreEscaped(result)) }
}

fn render_eure_tokens_only(content: &str) -> String {
    let cst = match eure::parol::parse_tolerant(content) {
        eure::parol::ParseResult::Ok(cst) => cst,
        eure::parol::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    let tokens = semantic_tokens(content, &cst);
    let mut html_output = String::new();
    let mut last_end = 0usize;

    for token in &tokens {
        let start = token.start as usize;
        let end = start + token.length as usize;
        if start > last_end {
            html_output.push_str(&escape_html(&content[last_end..start]));
        }
        html_output.push_str(&format!(
            "<span class=\"{}\">{}</span>",
            build_eure_classes(token),
            escape_html(&content[start..end]),
        ));
        last_end = end;
    }

    if last_end < content.len() {
        html_output.push_str(&escape_html(&content[last_end..]));
    }

    html_output
}

fn build_eure_classes(token: &SemanticToken) -> String {
    let mut classes = vec![match token.token_type {
        SemanticTokenType::Keyword => "docs-eure-keyword",
        SemanticTokenType::Number => "docs-eure-number",
        SemanticTokenType::String => "docs-eure-string",
        SemanticTokenType::Comment => "docs-eure-comment",
        SemanticTokenType::Operator => "docs-eure-operator",
        SemanticTokenType::Property => "docs-eure-property",
        SemanticTokenType::Punctuation => "docs-eure-punctuation",
        SemanticTokenType::Macro => "docs-eure-macro",
        SemanticTokenType::Decorator => "docs-eure-decorator",
        SemanticTokenType::SectionMarker => "docs-eure-section-marker",
        SemanticTokenType::ExtensionMarker => "docs-eure-extension-marker",
        SemanticTokenType::ExtensionIdent => "docs-eure-extension-ident",
    }];
    if token.modifiers & SemanticTokenModifier::Declaration.bitmask() != 0 {
        classes.push("docs-eure-mod-declaration");
    }
    if token.modifiers & SemanticTokenModifier::Definition.bitmask() != 0 {
        classes.push("docs-eure-mod-definition");
    }
    if token.modifiers & SemanticTokenModifier::SectionHeader.bitmask() != 0 {
        classes.push("docs-eure-mod-section-header");
    }
    classes.join(" ")
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn format_language_name(language: &str) -> String {
    match language.to_lowercase().as_str() {
        "toml" => "TOML".to_string(),
        "yaml" => "YAML".to_string(),
        "json" => "JSON".to_string(),
        "html" => "HTML".to_string(),
        "css" => "CSS".to_string(),
        "sql" => "SQL".to_string(),
        "xml" => "XML".to_string(),
        "rust" => "Rust".to_string(),
        "bash" | "shell" | "shellscript" | "sh" => "Bash".to_string(),
        "javascript" | "js" => "JavaScript".to_string(),
        "typescript" | "ts" => "TypeScript".to_string(),
        "python" | "py" => "Python".to_string(),
        "ruby" | "rb" => "Ruby".to_string(),
        "go" | "golang" => "Go".to_string(),
        "markdown" | "md" => "Markdown".to_string(),
        "dockerfile" => "Dockerfile".to_string(),
        "makefile" => "Makefile".to_string(),
        other => other.to_uppercase(),
    }
}

fn generate_docs_css(highlighter: &CodeHighlighter) -> Result<String, DocsError> {
    let syntax_css = highlighter.generate_css()?;
    let css = format!(
        r#"
:root {{
  --docs-bg: #11111b;
  --docs-surface: #181825;
  --docs-surface-2: #313244;
  --docs-border: #45475a;
  --docs-text: #cdd6f4;
  --docs-muted: #a6adc8;
  --docs-blue: #89b4fa;
  --docs-green: #a6e3a1;
  --docs-yellow: #f9e2af;
  --docs-red: #f38ba8;
  --docs-mauve: #cba6f7;
  --docs-peach: #fab387;
  --docs-teal: #94e2d5;
}}

.docs-content {{
  color: var(--docs-text);
}}

.docs-content .docs-page {{
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}}

.docs-content .docs-intro,
.docs-content .docs-text-plain {{
  color: var(--docs-muted);
  line-height: 1.8;
}}

.docs-content .docs-toc {{
  border: 1px solid var(--docs-border);
  border-radius: 12px;
  background: rgba(24, 24, 37, 0.7);
}}

.docs-content .docs-toc summary {{
  cursor: pointer;
  padding: 0.85rem 1rem;
  font-weight: 600;
}}

.docs-content .docs-toc nav {{
  padding: 0 1rem 1rem 1rem;
}}

.docs-content .docs-toc ul {{
  list-style: none;
  margin: 0;
  padding-left: 1rem;
}}

.docs-content .docs-toc > nav > ul {{
  padding-left: 0;
}}

.docs-content .docs-toc li {{
  margin: 0.35rem 0;
}}

.docs-content .docs-toc a,
.docs-content .docs-card-title a {{
  color: var(--docs-blue);
  text-decoration: none;
}}

.docs-content .docs-toc a:hover,
.docs-content .docs-card-title a:hover {{
  text-decoration: underline;
}}

.docs-content .docs-section {{
  display: flex;
  flex-direction: column;
  gap: 1rem;
}}

.docs-content .docs-section-heading {{
  color: var(--docs-mauve);
  line-height: 1.25;
  scroll-margin-top: 4rem;
}}

.docs-content .docs-section-h2 > .docs-section-heading {{
  font-size: 1.7rem;
}}

.docs-content .docs-section-h3 > .docs-section-heading {{
  font-size: 1.35rem;
}}

.docs-content .docs-markdown-content {{
  line-height: 1.8;
}}

.docs-content .docs-markdown-content h1,
.docs-content .docs-markdown-content h2,
.docs-content .docs-markdown-content h3,
.docs-content .docs-markdown-content h4,
.docs-content .docs-markdown-content h5,
.docs-content .docs-markdown-content h6 {{
  color: var(--docs-mauve);
  margin: 1.5rem 0 0.75rem;
}}

.docs-content .docs-markdown-content p,
.docs-content .docs-markdown-content ul,
.docs-content .docs-markdown-content ol,
.docs-content .docs-markdown-content blockquote,
.docs-content .docs-markdown-content table {{
  margin: 1rem 0;
}}

.docs-content .docs-markdown-content ul,
.docs-content .docs-markdown-content ol {{
  padding-left: 1.75rem;
}}

.docs-content .docs-markdown-content li {{
  margin: 0.4rem 0;
}}

.docs-content .docs-markdown-content code {{
  background: rgba(49, 50, 68, 0.9);
  color: var(--docs-peach);
  border-radius: 6px;
  padding: 0.15rem 0.35rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.92em;
}}

.docs-content .docs-markdown-content pre {{
  margin: 1rem 0;
}}

.docs-content .docs-markdown-content pre code {{
  background: transparent;
  padding: 0;
}}

.docs-content .docs-markdown-content a {{
  color: var(--docs-blue);
}}

.docs-content .docs-markdown-content table {{
  width: 100%;
  border-collapse: collapse;
}}

.docs-content .docs-markdown-content th,
.docs-content .docs-markdown-content td {{
  border: 1px solid var(--docs-border);
  padding: 0.65rem 0.75rem;
  text-align: left;
  vertical-align: top;
}}

.docs-content .docs-markdown-content th {{
  background: rgba(49, 50, 68, 0.8);
}}

.docs-content .docs-markdown-content blockquote {{
  border-left: 3px solid var(--docs-mauve);
  padding-left: 1rem;
  color: var(--docs-muted);
}}

.docs-content .docs-code-block,
.docs-content .docs-pre-plain,
.docs-content .docs-eure-source {{
  position: relative;
  overflow-x: auto;
  padding: 1rem;
  border-radius: 14px;
  background: var(--docs-surface);
  border: 1px solid var(--docs-border);
  box-shadow: 0 8px 30px rgba(17, 17, 27, 0.35);
  line-height: 1.6;
}}

.docs-content .docs-code-block[data-language] {{
  padding-top: 2rem;
}}

.docs-content .docs-code-block[data-language]::before {{
  content: attr(data-language);
  position: absolute;
  top: 0;
  left: 0;
  padding: 0.2rem 0.6rem;
  border-radius: 14px 0 10px 0;
  background: var(--docs-surface-2);
  color: var(--docs-muted);
  font-size: 0.75rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}}

.docs-content .docs-alert {{
  border-left: 4px solid var(--docs-blue);
  border-radius: 10px;
  background: rgba(49, 50, 68, 0.72);
  padding: 0.9rem 1rem;
}}

.docs-content .docs-alert-title {{
  font-weight: 700;
  margin-bottom: 0.5rem;
}}

.docs-content .docs-alert-tip {{
  border-left-color: var(--docs-green);
}}

.docs-content .docs-alert-important {{
  border-left-color: var(--docs-mauve);
}}

.docs-content .docs-alert-warning {{
  border-left-color: var(--docs-yellow);
}}

.docs-content .docs-alert-caution {{
  border-left-color: var(--docs-red);
}}

.docs-content .docs-card-list {{
  display: grid;
  gap: 1rem;
}}

.docs-content .docs-card {{
  border: 1px solid var(--docs-border);
  border-radius: 14px;
  background: rgba(24, 24, 37, 0.74);
  padding: 1rem 1.1rem;
}}

.docs-content .docs-card-title {{
  margin: 0 0 0.35rem;
}}

.docs-content .docs-card-meta {{
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  color: var(--docs-muted);
  font-size: 0.95rem;
}}

.docs-content .docs-tag-list {{
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  margin-top: 0.75rem;
}}

.docs-content .docs-link-list {{
  margin: 0;
  padding-left: 1.5rem;
}}

.docs-content .docs-link-list li {{
  margin: 0.45rem 0;
}}

.docs-content .docs-link-list a {{
  color: var(--docs-blue);
}}

.docs-content .docs-tag {{
  border-radius: 999px;
  background: rgba(69, 71, 90, 0.9);
  color: var(--docs-text);
  font-size: 0.8rem;
  padding: 0.2rem 0.55rem;
}}

.docs-content .docs-adr-meta {{
  border: 1px solid var(--docs-border);
  border-radius: 14px;
  background: rgba(24, 24, 37, 0.78);
  padding: 1rem 1.1rem;
}}

.docs-content .docs-adr-meta-grid {{
  display: grid;
  grid-template-columns: minmax(9rem, auto) 1fr;
  gap: 0.5rem 1rem;
  margin: 0;
}}

.docs-content .docs-adr-meta-grid dt {{
  color: var(--docs-muted);
  font-weight: 600;
}}

.docs-content .docs-adr-meta-grid dd {{
  margin: 0;
}}

.docs-content .docs-adr-section {{
  display: flex;
  flex-direction: column;
  gap: 1rem;
}}

.docs-eure-source {{
  color: var(--docs-text);
}}

.docs-eure-keyword {{ color: #cba6f7; }}
.docs-eure-number {{ color: #fab387; }}
.docs-eure-string {{ color: #a6e3a1; }}
.docs-eure-comment {{ color: #6c7086; font-style: italic; }}
.docs-eure-operator {{ color: #89dceb; }}
.docs-eure-property {{ color: #89b4fa; }}
.docs-eure-punctuation {{ color: #9399b2; }}
.docs-eure-macro {{ color: #f38ba8; }}
.docs-eure-decorator {{ color: #f9e2af; }}
.docs-eure-section-marker {{ color: #f5c2e7; font-weight: bold; }}
.docs-eure-extension-marker {{ color: #94e2d5; }}
.docs-eure-extension-ident {{ color: #94e2d5; }}
.docs-eure-mod-declaration {{ font-weight: 600; }}
.docs-eure-mod-definition {{ font-weight: 700; }}
.docs-eure-mod-section-header {{ text-decoration: underline; }}

.docs-eure-source-with-lines {{
  counter-reset: line;
  white-space: pre-wrap;
  word-break: break-word;
}}

.docs-eure-source-with-lines .docs-line {{
  display: block;
  counter-increment: line;
  position: relative;
  min-height: 1.5em;
}}

.docs-eure-source-with-lines .docs-line::before {{
  content: counter(line);
  position: absolute;
  right: 100%;
  width: 3rem;
  margin-right: 0.75rem;
  text-align: right;
  color: var(--docs-muted);
  user-select: none;
}}

{}
"#,
        syntax_css
    );
    Ok(css)
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
        std::env::temp_dir().join(format!("eure-mark-docs-{name}-{unique}"))
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
$docs {
  title: Home
  description: Home page
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
$docs {
  title: Guide
  description: Guide page
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

  "html-note" = ```html
<details><summary>More</summary><p>Trusted HTML</p></details>
```
  "html-note".$mark.dangerously-inner-html = true
}
"####,
        );
        write_file(
            &root.join("adrs/0001-example.eure"),
            r####"
$schema: ../../assets/schemas/eure-adr.schema.eure

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
        assert!(guide.html.contains("docs-alert"));
        assert!(guide.html.contains("Trusted HTML"));
        assert!(guide.css.contains("docs-hl-"));
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
$docs {
  title: Home
  description: Home page
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
