//! Generic Eure-authored article/page parsing and rendering.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use eure::FromEure;
#[cfg(feature = "eure-highlight")]
use eure::query::{SemanticToken, SemanticTokenModifier, SemanticTokenType, semantic_tokens};
use eure::value::{Language, Text};
use eure_document::map::Map;
#[cfg(feature = "eure-highlight")]
use giallo::FontStyle;
use giallo::{HighlightOptions, HtmlRenderer, Registry, RenderOptions, ThemeVariant};
use markdown::{CompileOptions, Options};
use maud::{Markup, PreEscaped, html};
use thiserror::Error;

pub const ARTICLE_SCHEMA: &str =
    include_str!("../assets/eure-mark-article.schema.eure");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHeading {
    pub id: String,
    pub title: String,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageMetadata {
    pub date: Option<String>,
    pub tags: Vec<String>,
    pub draft: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPage {
    pub title: String,
    pub description: String,
    pub html: String,
    pub css: String,
    pub headings: Vec<PageHeading>,
    pub metadata: PageMetadata,
}

#[derive(Debug, Error)]
pub enum ArticleError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse eure syntax in {origin}: {source}")]
    ParseEure {
        origin: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to build a document from {origin}: {source}")]
    BuildDocument {
        origin: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to decode {origin} into the article model: {source}")]
    DecodeDocument {
        origin: String,
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

    #[error("trusted HTML blocks must use the html language tag")]
    TrustedHtmlMustUseHtmlLanguage,

    #[error("article contains a duplicate section id {id}")]
    DuplicateSectionId { id: String },
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

#[derive(Debug, Clone, PartialEq, Eq, FromEure, Default)]
#[eure(crate = ::eure::document)]
pub struct MarkOptions {
    #[eure(default)]
    pub alert: Option<AlertType>,
    #[eure(rename = "dangerously-inner-html", default)]
    pub dangerously_inner_html: bool,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct Article {
    #[eure(ext)]
    pub frontmatter: ArticleFrontmatter,
    #[eure(rename = "#")]
    pub header: Text,
    #[eure(flatten)]
    pub sections: Map<String, Item<TextOrNested<Level2>>>,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(crate = ::eure::document)]
pub struct ArticleFrontmatter {
    pub title: Text,
    pub description: Text,
    #[eure(default)]
    pub date: Option<Text>,
    #[eure(default)]
    pub tags: Vec<String>,
    #[eure(default)]
    pub draft: bool,
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

pub fn parse_article(source: &str) -> Result<Article, ArticleError> {
    parse_article_source(source, "<input>")
}

pub fn parse_article_file(path: &Path) -> Result<Article, ArticleError> {
    let source = fs::read_to_string(path).map_err(|source| ArticleError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    parse_article_source(&source, &path.display().to_string())
}

pub fn render_article(article: &Article) -> Result<RenderedPage, ArticleError> {
    PageRenderer::new()?.render_article(article)
}

pub struct PageRenderer {
    highlighter: CodeHighlighter,
    css: String,
}

impl PageRenderer {
    pub fn new() -> Result<Self, ArticleError> {
        let highlighter = CodeHighlighter::new()?;
        let css = generate_article_css(&highlighter)?;
        Ok(Self { highlighter, css })
    }

    pub fn css(&self) -> &str {
        &self.css
    }

    pub fn render_article(&self, article: &Article) -> Result<RenderedPage, ArticleError> {
        let mut seen_ids = HashSet::new();
        let toc_entries = collect_toc_entries(&article.sections, &mut seen_ids)?;
        let headings = flatten_toc_entries(&toc_entries);

        let mut rendered_items = Vec::new();
        for (key, item) in article.sections.iter() {
            rendered_items.push(render_item_with_id(
                key,
                item,
                &self.highlighter,
                &toc_entries,
            )?);
        }

        let body = html! {
            article class="emark-page emark-page-article" {
                @for item in &rendered_items {
                    (item)
                }
            }
        };

        Ok(RenderedPage {
            title: article.frontmatter.title.as_str().to_string(),
            description: article.frontmatter.description.as_str().to_string(),
            html: body.into_string(),
            css: self.css.clone(),
            headings,
            metadata: PageMetadata {
                date: article
                    .frontmatter
                    .date
                    .as_ref()
                    .map(|date| date.as_str().to_string()),
                tags: article.frontmatter.tags.clone(),
                draft: article.frontmatter.draft,
            },
        })
    }

    pub fn render_text_fragment(&self, text: &Text) -> Result<String, ArticleError> {
        Ok(render_block_text(text, &self.highlighter)?.into_string())
    }

    pub fn render_text_with_mark_fragment(
        &self,
        text: &Text,
        mark: &MarkOptions,
    ) -> Result<String, ArticleError> {
        Ok(render_text_with_mark(text, mark, &self.highlighter)?.into_string())
    }
}

fn parse_article_source(source: &str, origin: &str) -> Result<Article, ArticleError> {
    let cst = eure::parol::parse(source).map_err(|source| ArticleError::ParseEure {
        origin: origin.to_string(),
        source: Box::new(source),
    })?;
    let document = eure::document::cst_to_document(source, &cst).map_err(|source| {
        ArticleError::BuildDocument {
            origin: origin.to_string(),
            source: Box::new(source),
        }
    })?;
    document
        .parse(document.get_root_id())
        .map_err(|source| ArticleError::DecodeDocument {
            origin: origin.to_string(),
            source: Box::new(source),
        })
}

fn collect_toc_entries<T: CollectToc>(
    sections: &Map<String, Item<TextOrNested<T>>>,
    seen_ids: &mut HashSet<String>,
) -> Result<Vec<TocEntry>, ArticleError> {
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

fn flatten_toc_entries(entries: &[TocEntry]) -> Vec<PageHeading> {
    let mut headings = Vec::new();
    flatten_toc_entries_into(entries, &mut headings);
    headings
}

fn flatten_toc_entries_into(entries: &[TocEntry], headings: &mut Vec<PageHeading>) {
    for entry in entries {
        headings.push(PageHeading {
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
    ) -> Result<Option<TocEntry>, ArticleError>;
}

impl CollectToc for Text {
    fn collect_toc_entry(
        &self,
        _id: &str,
        _seen_ids: &mut HashSet<String>,
    ) -> Result<Option<TocEntry>, ArticleError> {
        Ok(None)
    }
}

impl<T: CollectToc> CollectToc for TextOrNested<T> {
    fn collect_toc_entry(
        &self,
        id: &str,
        seen_ids: &mut HashSet<String>,
    ) -> Result<Option<TocEntry>, ArticleError> {
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
            ) -> Result<Option<TocEntry>, ArticleError> {
                if !seen_ids.insert(id.to_string()) {
                    return Err(ArticleError::DuplicateSectionId { id: id.to_string() });
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
    ) -> Result<Option<TocEntry>, ArticleError> {
        if !seen_ids.insert(id.to_string()) {
            return Err(ArticleError::DuplicateSectionId { id: id.to_string() });
        }
        Ok(None)
    }
}

fn render_item_with_id<T: RenderNestedWithId>(
    key: &str,
    item: &Item<T>,
    highlighter: &CodeHighlighter,
    toc_entries: &[TocEntry],
) -> Result<Markup, ArticleError> {
    match item {
        Item::Normal(value) => Ok(html! {
            div class="emark-item" data-key=(key) {
                (value.render_with_id(key, highlighter, toc_entries)?)
            }
        }),
        Item::List(items) => {
            let mut rendered_items = Vec::new();
            for value in items {
                rendered_items.push(value.render_with_id(key, highlighter, toc_entries)?);
            }
            Ok(html! {
                div class="emark-item-list" data-key=(key) {
                    @for value in &rendered_items {
                        div class="emark-item-list-entry" {
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
            details class="emark-toc" open {
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
    ) -> Result<Markup, ArticleError>;
}

impl RenderNestedWithId for Text {
    fn render_with_id(
        &self,
        _id: &str,
        highlighter: &CodeHighlighter,
        _toc_entries: &[TocEntry],
    ) -> Result<Markup, ArticleError> {
        render_block_text(self, highlighter)
    }
}

impl<T: RenderNestedWithId> RenderNestedWithId for TextOrNested<T> {
    fn render_with_id(
        &self,
        id: &str,
        highlighter: &CodeHighlighter,
        toc_entries: &[TocEntry],
    ) -> Result<Markup, ArticleError> {
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
            ) -> Result<Markup, ArticleError> {
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
) -> Result<Markup, ArticleError> {
    let mut rendered_items = Vec::new();
    for (key, item) in sections.iter() {
        rendered_items.push(render_item_with_id(key, item, highlighter, toc_entries)?);
    }

    Ok(html! {
        section class=(format!("emark-section emark-section-{}", level)) {
            @match level {
                "h2" => h2 class="emark-section-heading" id=(id) { (header) },
                "h3" => h3 class="emark-section-heading" id=(id) { (header) },
                "h4" => h4 class="emark-section-heading" id=(id) { (header) },
                "h5" => h5 class="emark-section-heading" id=(id) { (header) },
                "h6" => h6 class="emark-section-heading" id=(id) { (header) },
                _ => h2 class="emark-section-heading" id=(id) { (header) },
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
) -> Result<Markup, ArticleError> {
    let content = if mark.dangerously_inner_html {
        if !text.language.is_other("html") {
            return Err(ArticleError::TrustedHtmlMustUseHtmlLanguage);
        }
        html! { div class="emark-markdown" { (PreEscaped(text.as_str().to_string())) } }
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
            div class=(format!("emark-alert emark-alert-{}", alert_class)) {
                div class="emark-alert-title" { (alert_title) }
                div class="emark-alert-body" { (content) }
            }
        })
    } else {
        Ok(content)
    }
}

fn render_block_text(text: &Text, highlighter: &CodeHighlighter) -> Result<Markup, ArticleError> {
    match &text.language {
        Language::Plaintext => Ok(html! { p class="emark-text-plain" { (text.as_str()) } }),
        Language::Implicit => Ok(html! {
            pre class="emark-pre-plain" {
                code { (text.as_str()) }
            }
        }),
        Language::Other(language) => match language.as_ref() {
            "markdown" => render_markdown(text.as_str()),
            "html" => {
                Ok(html! { div class="emark-markdown" { (PreEscaped(text.as_str().to_string())) } })
            }
            "eure" => render_eure_block(text.as_str(), highlighter),
            other => Ok(highlighter.highlight_or_plain(text.as_str(), other)),
        },
    }
}

fn render_eure_block(content: &str, highlighter: &CodeHighlighter) -> Result<Markup, ArticleError> {
    #[cfg(feature = "eure-highlight")]
    {
        Ok(render_eure_highlighted(content, highlighter))
    }

    #[cfg(not(feature = "eure-highlight"))]
    {
        Ok(highlighter.highlight_or_plain(content, "eure"))
    }
}

fn render_plain_code_block(content: &str, language: &str) -> Markup {
    html! {
        pre class="emark-code-block emark-pre-plain" data-language=(format_language_name(language)) {
            code { (content) }
        }
    }
}

fn render_markdown(content: &str) -> Result<Markup, ArticleError> {
    let options = Options {
        compile: CompileOptions {
            allow_dangerous_html: true,
            ..CompileOptions::default()
        },
        ..Options::gfm()
    };
    let html_output = markdown::to_html_with_options(content, &options).map_err(|message| {
        ArticleError::MarkdownRender {
            message: message.to_string(),
        }
    })?;
    Ok(html! {
        div class="emark-markdown" {
            (PreEscaped(html_output))
        }
    })
}

struct CodeHighlighter {
    registry: Registry,
}

impl CodeHighlighter {
    fn new() -> Result<Self, ArticleError> {
        let mut registry =
            Registry::builtin().map_err(|source| ArticleError::HighlighterInit { source })?;
        registry.link_grammars();
        Ok(Self { registry })
    }

    fn generate_css(&self) -> Result<String, ArticleError> {
        self.registry
            .generate_css("catppuccin-mocha", "emark-hl-")
            .map_err(|source| ArticleError::SyntaxCss { source })
    }

    fn highlight_or_plain(&self, code: &str, language: &str) -> Markup {
        let options = HighlightOptions::new(language, ThemeVariant::Single("catppuccin-mocha"));
        match self.registry.highlight(code, &options) {
            Ok(highlighted) => {
                let renderer = HtmlRenderer {
                    css_class_prefix: Some("emark-hl-".to_string()),
                    ..Default::default()
                };
                let html_output = renderer.render(&highlighted, &RenderOptions::default());
                let html_with_badge = html_output.replacen(
                    "<pre class=\"giallo emark-hl-code\">",
                    &format!(
                        "<pre class=\"emark-code-block giallo emark-hl-code\" data-language=\"{}\">",
                        format_language_name(language)
                    ),
                    1,
                );
                html! { (PreEscaped(html_with_badge)) }
            }
            Err(_) => render_plain_code_block(code, language),
        }
    }

    #[cfg(feature = "eure-highlight")]
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

#[cfg(feature = "eure-highlight")]
fn render_eure_highlighted(content: &str, highlighter: &CodeHighlighter) -> Markup {
    let cst = match eure::parol::parse_tolerant(content) {
        eure::parol::ParseResult::Ok(cst) => cst,
        eure::parol::ParseResult::ErrWithCst { cst, .. } => cst,
    };
    let tokens = semantic_tokens(content, &cst);
    let code_blocks = find_code_block_regions(content);
    html! {
        pre class="emark-code-block emark-eure-source" data-language="Eure" {
            code { (PreEscaped(render_tokens_to_string(content, &tokens, &code_blocks, highlighter))) }
        }
    }
}

#[cfg(feature = "eure-highlight")]
struct CodeBlockRegion {
    content_start: usize,
    content_end: usize,
    language: String,
}

#[cfg(feature = "eure-highlight")]
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

#[cfg(feature = "eure-highlight")]
fn find_code_block_for_range(
    gap_start: usize,
    gap_end: usize,
    regions: &[CodeBlockRegion],
) -> Option<&CodeBlockRegion> {
    regions
        .iter()
        .find(|region| gap_start < region.content_end && gap_end > region.content_start)
}

#[cfg(feature = "eure-highlight")]
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

#[cfg(feature = "eure-highlight")]
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

#[cfg(feature = "eure-highlight")]
fn build_eure_classes(token: &SemanticToken) -> String {
    let mut classes = vec![match token.token_type {
        SemanticTokenType::Keyword => "emark-eure-keyword",
        SemanticTokenType::Number => "emark-eure-number",
        SemanticTokenType::String => "emark-eure-string",
        SemanticTokenType::Comment => "emark-eure-comment",
        SemanticTokenType::Operator => "emark-eure-operator",
        SemanticTokenType::Property => "emark-eure-property",
        SemanticTokenType::Punctuation => "emark-eure-punctuation",
        SemanticTokenType::Macro => "emark-eure-macro",
        SemanticTokenType::Decorator => "emark-eure-decorator",
        SemanticTokenType::SectionMarker => "emark-eure-section-marker",
        SemanticTokenType::ExtensionMarker => "emark-eure-extension-marker",
        SemanticTokenType::ExtensionIdent => "emark-eure-extension-ident",
    }];
    if token.modifiers & SemanticTokenModifier::Declaration.bitmask() != 0 {
        classes.push("emark-eure-mod-declaration");
    }
    if token.modifiers & SemanticTokenModifier::Definition.bitmask() != 0 {
        classes.push("emark-eure-mod-definition");
    }
    if token.modifiers & SemanticTokenModifier::SectionHeader.bitmask() != 0 {
        classes.push("emark-eure-mod-section-header");
    }
    classes.join(" ")
}

#[cfg(feature = "eure-highlight")]
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
        other if other.eq_ignore_ascii_case("eure") => "Eure".to_string(),
        other => other.to_uppercase(),
    }
}

fn generate_article_css(highlighter: &CodeHighlighter) -> Result<String, ArticleError> {
    let syntax_css = highlighter.generate_css()?;
    let css = format!(
        r#"
:root {{
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
}}

.emark-page {{
  color: var(--emark-text);
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}}

.emark-text-plain {{
  color: var(--emark-muted);
  line-height: 1.8;
}}

.emark-toc {{
  border: 1px solid var(--emark-border);
  border-radius: 12px;
  background: rgba(24, 24, 37, 0.7);
}}

.emark-toc summary {{
  cursor: pointer;
  padding: 0.85rem 1rem;
  font-weight: 600;
}}

.emark-toc nav {{
  padding: 0 1rem 1rem 1rem;
}}

.emark-toc ul {{
  list-style: none;
  margin: 0;
  padding-left: 1rem;
}}

.emark-toc > nav > ul {{
  padding-left: 0;
}}

.emark-toc li {{
  margin: 0.35rem 0;
}}

.emark-toc a {{
  color: var(--emark-blue);
  text-decoration: none;
}}

.emark-toc a:hover {{
  text-decoration: underline;
}}

.emark-section {{
  display: flex;
  flex-direction: column;
  gap: 1rem;
}}

.emark-section-heading {{
  color: var(--emark-mauve);
  line-height: 1.25;
  scroll-margin-top: 4rem;
}}

.emark-section-h2 > .emark-section-heading {{
  font-size: 1.7rem;
}}

.emark-section-h3 > .emark-section-heading {{
  font-size: 1.35rem;
}}

.emark-markdown {{
  line-height: 1.8;
}}

.emark-markdown h1,
.emark-markdown h2,
.emark-markdown h3,
.emark-markdown h4,
.emark-markdown h5,
.emark-markdown h6 {{
  color: var(--emark-mauve);
  margin: 1.5rem 0 0.75rem;
}}

.emark-markdown p,
.emark-markdown ul,
.emark-markdown ol,
.emark-markdown blockquote,
.emark-markdown table {{
  margin: 1rem 0;
}}

.emark-markdown ul,
.emark-markdown ol {{
  padding-left: 1.75rem;
}}

.emark-markdown li {{
  margin: 0.4rem 0;
}}

.emark-markdown code {{
  background: rgba(49, 50, 68, 0.9);
  color: var(--emark-peach);
  border-radius: 6px;
  padding: 0.15rem 0.35rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.92em;
}}

.emark-markdown pre {{
  margin: 1rem 0;
}}

.emark-markdown pre code {{
  background: transparent;
  padding: 0;
}}

.emark-markdown a {{
  color: var(--emark-blue);
}}

.emark-markdown table {{
  width: 100%;
  border-collapse: collapse;
}}

.emark-markdown th,
.emark-markdown td {{
  border: 1px solid var(--emark-border);
  padding: 0.65rem 0.75rem;
  text-align: left;
  vertical-align: top;
}}

.emark-markdown th {{
  background: rgba(49, 50, 68, 0.8);
}}

.emark-markdown blockquote {{
  border-left: 3px solid var(--emark-mauve);
  padding-left: 1rem;
  color: var(--emark-muted);
}}

.emark-code-block,
.emark-pre-plain,
.emark-eure-source {{
  position: relative;
  overflow-x: auto;
  padding: 1rem;
  border-radius: 14px;
  background: var(--emark-surface);
  border: 1px solid var(--emark-border);
  box-shadow: 0 8px 30px rgba(17, 17, 27, 0.35);
  line-height: 1.6;
}}

.emark-code-block[data-language] {{
  padding-top: 2rem;
}}

.emark-code-block[data-language]::before {{
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
}}

.emark-alert {{
  border-left: 4px solid var(--emark-blue);
  border-radius: 10px;
  background: rgba(49, 50, 68, 0.72);
  padding: 0.9rem 1rem;
}}

.emark-alert-title {{
  font-weight: 700;
  margin-bottom: 0.5rem;
}}

.emark-alert-tip {{
  border-left-color: var(--emark-green);
}}

.emark-alert-important {{
  border-left-color: var(--emark-mauve);
}}

.emark-alert-warning {{
  border-left-color: var(--emark-yellow);
}}

.emark-alert-caution {{
  border-left-color: var(--emark-red);
}}

.emark-eure-source {{
  color: var(--emark-text);
}}

.emark-eure-keyword {{ color: #cba6f7; }}
.emark-eure-number {{ color: #fab387; }}
.emark-eure-string {{ color: #a6e3a1; }}
.emark-eure-comment {{ color: #6c7086; font-style: italic; }}
.emark-eure-operator {{ color: #89dceb; }}
.emark-eure-property {{ color: #89b4fa; }}
.emark-eure-punctuation {{ color: #9399b2; }}
.emark-eure-macro {{ color: #f38ba8; }}
.emark-eure-decorator {{ color: #f9e2af; }}
.emark-eure-section-marker {{ color: #f5c2e7; font-weight: bold; }}
.emark-eure-extension-marker {{ color: #94e2d5; }}
.emark-eure-extension-ident {{ color: #94e2d5; }}
.emark-eure-mod-declaration {{ font-weight: 600; }}
.emark-eure-mod-definition {{ font-weight: 700; }}
.emark-eure-mod-section-header {{ text-decoration: underline; }}

{}
"#,
        syntax_css
    );
    Ok(css)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARTICLE_SOURCE: &str = r####"
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

  "html-note" = ```html
<details><summary>More</summary><p>Trusted HTML</p></details>
```
  "html-note".$mark.dangerously-inner-html = true
}
"####;

    #[test]
    fn parses_and_renders_generic_article() {
        let article = parse_article(ARTICLE_SOURCE).expect("article should parse");
        let rendered = render_article(&article).expect("article should render");

        assert_eq!(rendered.title, "Guide");
        assert_eq!(rendered.metadata.tags, vec!["example"]);
        assert!(rendered.html.contains("emark-alert"));
        assert!(rendered.html.contains("Trusted HTML"));
        assert!(rendered.html.contains("On This Page"));
        assert!(rendered.css.contains("emark-hl-"));
    }

    #[test]
    fn duplicate_section_ids_fail_render() {
        let article = parse_article(
            r####"
$frontmatter {
  title = "Guide"
  description = "Guide page"
}

"#": Guide

@ "dup" {
  "##": One
}

@ "other" {
  "##": Two

  @ "dup" {
    "###": Nested
  }
}
"####,
        )
        .expect("article should parse");

        let error = render_article(&article).expect_err("duplicate ids should fail");
        assert!(matches!(error, ArticleError::DuplicateSectionId { id } if id == "dup"));
    }

    #[cfg(feature = "eure-highlight")]
    #[test]
    fn eure_blocks_use_semantic_highlight_classes() {
        let article = parse_article(
            r####"
$frontmatter {
  title = "Guide"
  description = "Guide page"
}

"#": Guide

"body" = ```eure
title: demo
```
"####,
        )
        .expect("article should parse");
        let rendered = render_article(&article).expect("article should render");
        assert!(rendered.html.contains("emark-eure-property"));
    }
}
