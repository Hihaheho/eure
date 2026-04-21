use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigratedGuideDocument {
    pub title: String,
    pub description: String,
    pub eure_source: String,
}

#[derive(Debug, Error)]
pub enum GuideMigrationError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to migrate {path}: document is empty")]
    EmptyDocument { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GuideItem {
    Toc,
    Text(TextBlock),
    Section(GuideSection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextBlock {
    language: Option<String>,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GuideSection {
    level: u8,
    id: String,
    heading: String,
    items: Vec<GuideItem>,
}

static MARKDOWN_LINK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\(([^)]+)\)").expect("valid markdown link regex"));

pub fn migrate_markdown_file(
    input_path: &Path,
    docs_root: &Path,
) -> Result<MigratedGuideDocument, GuideMigrationError> {
    let markdown =
        fs::read_to_string(input_path).map_err(|source| GuideMigrationError::ReadFile {
            path: input_path.to_path_buf(),
            source,
        })?;
    migrate_markdown_guide(&markdown, input_path, docs_root)
}

pub fn migrate_markdown_guide(
    markdown: &str,
    input_path: &Path,
    docs_root: &Path,
) -> Result<MigratedGuideDocument, GuideMigrationError> {
    let relative_path = input_path
        .strip_prefix(docs_root)
        .unwrap_or(input_path)
        .to_path_buf();
    let mut state = ParseState::new(relative_path, file_stem_title(input_path));
    state.parse(markdown);

    let title = state
        .title
        .as_ref()
        .filter(|title| !title.is_empty())
        .cloned()
        .unwrap_or_else(|| file_stem_title(input_path));
    if title.trim().is_empty() && state.root_items.is_empty() {
        return Err(GuideMigrationError::EmptyDocument {
            path: input_path.to_path_buf(),
        });
    }

    if !state.contains_toc()
        && state
            .root_items
            .iter()
            .any(|item| matches!(item, GuideItem::Section(_)))
    {
        let insert_at = state
            .root_items
            .iter()
            .take_while(|item| matches!(item, GuideItem::Text(_)))
            .count();
        state.root_items.insert(insert_at, GuideItem::Toc);
    }

    let description = collect_description(&state.root_items)
        .unwrap_or_else(|| format!("Documentation page for {}.", title));
    let eure_source = render_guide_document(&title, &description, &state.root_items);

    Ok(MigratedGuideDocument {
        title,
        description,
        eure_source,
    })
}

pub fn migrate_markdown_guides_in_place(
    docs_root: &Path,
) -> Result<Vec<PathBuf>, GuideMigrationError> {
    let mut markdown_files = Vec::new();
    collect_markdown_files(docs_root, docs_root, &mut markdown_files)?;
    markdown_files.sort();

    let mut generated = Vec::new();
    for markdown_file in markdown_files {
        let migrated = migrate_markdown_file(&markdown_file, docs_root)?;
        let output_path = markdown_file.with_extension("eure");
        fs::write(&output_path, migrated.eure_source).map_err(|source| {
            GuideMigrationError::WriteFile {
                path: output_path.clone(),
                source,
            }
        })?;
        fs::remove_file(&markdown_file).map_err(|source| GuideMigrationError::WriteFile {
            path: markdown_file.clone(),
            source,
        })?;
        generated.push(output_path);
    }

    Ok(generated)
}

fn collect_markdown_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), GuideMigrationError> {
    for entry in fs::read_dir(dir).map_err(|source| GuideMigrationError::ReadFile {
        path: dir.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| GuideMigrationError::ReadFile {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".DS_Store") {
            continue;
        }
        if path.is_dir() {
            if path
                .strip_prefix(root)
                .ok()
                .is_some_and(|relative| relative.starts_with("adrs"))
            {
                continue;
            }
            collect_markdown_files(root, &path, files)?;
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
    Ok(())
}

struct ParseState {
    current_path: PathBuf,
    default_title: String,
    title: Option<String>,
    root_items: Vec<GuideItem>,
    stack: Vec<GuideSection>,
    used_section_ids: HashMap<String, usize>,
    markdown_buffer: String,
    skip_until_level: Option<u8>,
    code_fence: Option<CodeFence>,
    code_buffer: Vec<String>,
}

impl ParseState {
    fn new(current_path: PathBuf, default_title: String) -> Self {
        Self {
            current_path,
            default_title,
            title: None,
            root_items: Vec::new(),
            stack: Vec::new(),
            used_section_ids: HashMap::new(),
            markdown_buffer: String::new(),
            skip_until_level: None,
            code_fence: None,
            code_buffer: Vec::new(),
        }
    }

    fn parse(&mut self, markdown: &str) {
        for raw_line in markdown.lines() {
            let line = raw_line.trim_end_matches('\r');
            if let Some(fence) = self.code_fence.clone() {
                if is_closing_fence(line, &fence.delimiter) {
                    let content = self.code_buffer.join("\n");
                    self.push_item(GuideItem::Text(TextBlock {
                        language: fence.language,
                        content,
                    }));
                    self.code_fence = None;
                    self.code_buffer.clear();
                } else {
                    self.code_buffer.push(line.to_string());
                }
                continue;
            }

            if let Some((level, heading)) = parse_heading(line) {
                if let Some(skip_level) = self.skip_until_level {
                    if level > skip_level {
                        continue;
                    }
                    self.skip_until_level = None;
                }

                self.flush_markdown();

                if level == 1 && self.title.is_none() {
                    self.title = Some(heading);
                    continue;
                }

                if heading.eq_ignore_ascii_case("table of contents") {
                    self.push_item(GuideItem::Toc);
                    self.skip_until_level = Some(level);
                    continue;
                }

                self.open_section(level, heading);
                continue;
            }

            if self.skip_until_level.is_some() {
                continue;
            }

            if let Some(code_fence) = parse_opening_fence(line) {
                self.flush_markdown();
                self.code_fence = Some(code_fence);
                self.code_buffer.clear();
                continue;
            }

            self.markdown_buffer.push_str(line);
            self.markdown_buffer.push('\n');
        }

        if self.code_fence.take().is_some() {
            let content = self.code_buffer.join("\n");
            self.push_item(GuideItem::Text(TextBlock {
                language: Some("markdown".to_string()),
                content: format!("```\n{}\n```", content),
            }));
            self.code_buffer.clear();
        }

        self.flush_markdown();
        self.finish_sections();

        if self.title.is_none() {
            self.title = Some(self.default_title.clone());
        }
    }

    fn contains_toc(&self) -> bool {
        fn contains_toc_in(items: &[GuideItem]) -> bool {
            items.iter().any(|item| match item {
                GuideItem::Toc => true,
                GuideItem::Text(_) => false,
                GuideItem::Section(section) => contains_toc_in(&section.items),
            })
        }

        contains_toc_in(&self.root_items)
    }

    fn flush_markdown(&mut self) {
        let trimmed = self.markdown_buffer.trim();
        if trimmed.is_empty() {
            self.markdown_buffer.clear();
            return;
        }
        let content = rewrite_markdown_links(trimmed, &self.current_path);
        self.push_item(GuideItem::Text(TextBlock {
            language: Some("markdown".to_string()),
            content,
        }));
        self.markdown_buffer.clear();
    }

    fn open_section(&mut self, level: u8, heading: String) {
        while self
            .stack
            .last()
            .is_some_and(|section| section.level >= level)
        {
            self.finish_last_section();
        }
        let base_id = slugify(&heading);
        let id = self.allocate_section_id(base_id);
        self.stack.push(GuideSection {
            level,
            id,
            heading,
            items: Vec::new(),
        });
    }

    fn finish_last_section(&mut self) {
        let finished = self.stack.pop().expect("section exists");
        self.push_item(GuideItem::Section(finished));
    }

    fn finish_sections(&mut self) {
        while !self.stack.is_empty() {
            self.finish_last_section();
        }
    }

    fn push_item(&mut self, item: GuideItem) {
        if let Some(section) = self.stack.last_mut() {
            section.items.push(item);
        } else {
            self.root_items.push(item);
        }
    }

    fn allocate_section_id(&mut self, base_id: String) -> String {
        let count = self.used_section_ids.entry(base_id.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            base_id
        } else {
            format!("{base_id}-{}", *count)
        }
    }
}

#[derive(Debug, Clone)]
struct CodeFence {
    delimiter: String,
    language: Option<String>,
}

fn parse_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = trimmed[hashes..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((hashes as u8, rest.trim_end_matches('#').trim().to_string()))
}

fn parse_opening_fence(line: &str) -> Option<CodeFence> {
    let trimmed = line.trim_start();
    let ticks = trimmed.chars().take_while(|ch| *ch == '`').count();
    if ticks < 3 {
        return None;
    }
    let delimiter = "`".repeat(ticks);
    let language = trimmed[ticks..]
        .split_whitespace()
        .next()
        .map(|language| {
            language
                .trim_matches(|ch: char| {
                    !ch.is_alphanumeric() && ch != '+' && ch != '#' && ch != '-' && ch != '_'
                })
                .to_string()
        })
        .filter(|language| !language.is_empty());
    Some(CodeFence {
        delimiter,
        language,
    })
}

fn is_closing_fence(line: &str, delimiter: &str) -> bool {
    line.trim_start().starts_with(delimiter)
}

fn rewrite_markdown_links(content: &str, current_path: &Path) -> String {
    MARKDOWN_LINK_PATTERN
        .replace_all(content, |captures: &regex::Captures<'_>| {
            let raw_target = captures
                .get(1)
                .expect("markdown link pattern always captures")
                .as_str()
                .trim();
            let Some(rewritten) = rewrite_docs_target(raw_target, current_path) else {
                return captures.get(0).expect("match exists").as_str().to_string();
            };
            format!("]({rewritten})")
        })
        .into_owned()
}

fn rewrite_docs_target(target: &str, current_path: &Path) -> Option<String> {
    if target.starts_with('#')
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("data:")
    {
        return None;
    }

    let (path_part, anchor) = target
        .split_once('#')
        .map_or((target, None), |(path, anchor)| (path, Some(anchor)));
    if !path_part.ends_with(".md") {
        return None;
    }

    let parent = current_path.parent().unwrap_or(Path::new(""));
    let resolved = normalize_relative_path(&parent.join(path_part));
    let public = if resolved == Path::new("index.md") {
        "/docs/".to_string()
    } else {
        format!(
            "/docs/{}",
            resolved
                .with_extension("")
                .to_string_lossy()
                .replace('\\', "/")
        )
    };
    if let Some(anchor) = anchor {
        Some(format!("{public}#{anchor}"))
    } else {
        Some(public)
    }
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    normalized
}

fn collect_description(items: &[GuideItem]) -> Option<String> {
    for item in items {
        match item {
            GuideItem::Toc => {}
            GuideItem::Text(text) => {
                if text
                    .language
                    .as_deref()
                    .is_some_and(|language| language.eq_ignore_ascii_case("markdown"))
                {
                    let summary = summarize_markdown(&text.content);
                    if !summary.is_empty() {
                        return Some(summary);
                    }
                }
            }
            GuideItem::Section(section) => {
                if let Some(summary) = collect_description(&section.items) {
                    return Some(summary);
                }
            }
        }
    }
    None
}

fn summarize_markdown(markdown: &str) -> String {
    let paragraph = markdown
        .split("\n\n")
        .find(|paragraph| !paragraph.trim().is_empty())
        .unwrap_or(markdown);

    let mut summary = paragraph.to_string();
    summary = Regex::new(r"\[([^\]]+)\]\([^)]+\)")
        .expect("valid link cleanup regex")
        .replace_all(&summary, "$1")
        .into_owned();
    summary = summary
        .replace("**", "")
        .replace("__", "")
        .replace(['`', '*', '_'], "");
    summary = summary
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    summary.chars().take(180).collect()
}

fn render_guide_document(title: &str, description: &str, items: &[GuideItem]) -> String {
    let mut output = String::new();
    output.push_str("$docs {\n");
    output.push_str("  title: ");
    output.push_str(title.trim());
    output.push('\n');
    output.push_str("  description: ");
    output.push_str(description.trim());
    output.push('\n');
    output.push_str("}\n\n");
    output.push_str("\"#\": ");
    output.push_str(title.trim());
    output.push_str("\n\n");
    render_items(items, 0, &mut output);
    output
}

fn render_items(items: &[GuideItem], indent: usize, output: &mut String) {
    let mut allocator = KeyAllocator::default();
    for (index, item) in items.iter().enumerate() {
        match item {
            GuideItem::Toc => {
                let key = allocator.alloc("toc");
                write_indent(output, indent);
                output.push_str(&quote_key(&key));
                output.push_str(".$toc = true\n");
            }
            GuideItem::Text(text) => {
                let base = text_key_base(text);
                let key = allocator.alloc(&base);
                render_text_block(text, &key, indent, output);
            }
            GuideItem::Section(section) => {
                let key = allocator.alloc(&section.id);
                render_section(section, &key, indent, output);
            }
        }
        if index + 1 < items.len() {
            output.push('\n');
        }
    }
}

fn render_section(section: &GuideSection, key: &str, indent: usize, output: &mut String) {
    write_indent(output, indent);
    output.push_str("@ ");
    output.push_str(&quote_key(key));
    output.push_str(" {\n");
    write_indent(output, indent + 2);
    output.push_str(&quote_key(&"#".repeat(section.level as usize)));
    output.push_str(": ");
    output.push_str(section.heading.trim());
    output.push_str("\n\n");
    render_items(&section.items, indent + 2, output);
    output.push('\n');
    write_indent(output, indent);
    output.push_str("}\n");
}

fn render_text_block(text: &TextBlock, key: &str, indent: usize, output: &mut String) {
    let fence_len = fence_length(&text.content);
    let delimiter = "`".repeat(fence_len);

    write_indent(output, indent);
    output.push_str(&quote_key(key));
    output.push_str(" = ");
    output.push_str(&delimiter);
    if let Some(language) = &text.language {
        output.push_str(language);
    }
    output.push('\n');
    output.push_str(text.content.trim_end());
    output.push('\n');
    output.push_str(&delimiter);
    output.push('\n');
}

fn fence_length(content: &str) -> usize {
    let mut max_run = 0usize;
    let mut current_run = 0usize;
    for ch in content.chars() {
        if ch == '`' {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    max_run.max(2) + 1
}

fn write_indent(output: &mut String, indent: usize) {
    for _ in 0..indent {
        output.push(' ');
    }
}

fn quote_key(key: &str) -> String {
    format!("{:?}", key)
}

fn text_key_base(text: &TextBlock) -> String {
    match text.language.as_deref() {
        Some("markdown") => "body".to_string(),
        Some(language) => format!("{}-snippet", slugify(language)),
        None => "snippet".to_string(),
    }
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in text.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "section".to_string()
    } else {
        slug
    }
}

fn file_stem_title(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Documentation");
    stem.split('-')
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

#[derive(Default)]
struct KeyAllocator {
    used: HashSet<String>,
    counts: HashMap<String, usize>,
}

impl KeyAllocator {
    fn alloc(&mut self, base: &str) -> String {
        let base = if base.is_empty() { "item" } else { base };
        if self.used.insert(base.to_string()) {
            self.counts.insert(base.to_string(), 1);
            return base.to_string();
        }

        let next = self.counts.entry(base.to_string()).or_insert(1);
        loop {
            *next += 1;
            let candidate = format!("{}-{}", base, next);
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_markdown_headings_and_code_blocks() {
        let markdown = r#"# Extensions

Intro paragraph with [other docs](./other.md#anchor).

## Syntax vs Storage

Paragraph text.

```rust
fn main() {}
```

### Nested Topic

More details.
"#;
        let migrated = migrate_markdown_guide(
            markdown,
            Path::new("/repo/docs/extensions.md"),
            Path::new("/repo/docs"),
        )
        .expect("markdown should migrate");

        assert_eq!(migrated.title, "Extensions");
        assert!(migrated.eure_source.contains("$docs {"));
        assert!(migrated.eure_source.contains("\"#\": Extensions"));
        assert!(migrated.eure_source.contains("@ \"syntax-vs-storage\" {"));
        assert!(migrated.eure_source.contains("\"##\": Syntax vs Storage"));
        assert!(migrated.eure_source.contains("@ \"nested-topic\" {"));
        assert!(migrated.eure_source.contains("\"rust-snippet\" = ```rust"));
        assert!(migrated.eure_source.contains("/docs/other#anchor"));
    }

    #[test]
    fn manual_table_of_contents_becomes_toc_marker() {
        let markdown = r#"# Spec

## Table of Contents

1. [Intro](#intro)

## Intro

Hello.
"#;
        let migrated = migrate_markdown_guide(
            markdown,
            Path::new("/repo/docs/spec.md"),
            Path::new("/repo/docs"),
        )
        .expect("markdown should migrate");

        assert!(migrated.eure_source.contains("\"toc\".$toc = true"));
        assert!(!migrated.eure_source.contains("1. [Intro](#intro)"));
    }

    #[test]
    fn rewrites_relative_links_across_directories() {
        let content = "See [schema variants](../schema-extensions.md#variants).";
        let rewritten = rewrite_markdown_links(content, Path::new("spec/alpha.md"));
        assert_eq!(
            rewritten,
            "See [schema variants](/docs/schema-extensions#variants)."
        );
    }
}
