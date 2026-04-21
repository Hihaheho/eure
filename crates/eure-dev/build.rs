use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "build-image")]
use std::io::{self, Write};

use eure_doc_builder::{DocsPageKind, DocsSite, build_docs_site};

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let cargo_out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));

    // Ensures assets/tailwind.css exists not to fail the build for `asset!("/assets/tailwind.css")` on local or CI.
    let tailwind_css_path = manifest_dir.join("assets/tailwind.css");
    if !tailwind_css_path.exists() {
        fs::write(&tailwind_css_path, "").expect("Failed to create tailwind.css");
    }
    println!("cargo:rerun-if-changed={}", tailwind_css_path.display());

    let out_dir = manifest_dir.join("assets");
    fs::create_dir_all(&out_dir).expect("failed to create assets/");

    #[cfg(feature = "build-image")]
    {
        // Light theme favicons
        let light_svg = manifest_dir.join("../../assets/icons/eure-icon.svg");
        if light_svg.exists() {
            println!("cargo:rerun-if-changed={}", light_svg.display());
            generate_favicons(&light_svg, &out_dir, "");
        }
    }

    #[cfg(not(feature = "build-image"))]
    {
        /// All favicon files that need to exist for asset!() to compile
        const FAVICON_FILES: &[&str] = &[
            "favicon-16x16.png",
            "favicon-32x32.png",
            "apple-touch-icon.png",
            "android-chrome-192x192.png",
            "android-chrome-512x512.png",
            "favicon.ico",
        ];
        // Generate empty placeholder files so asset!() doesn't fail
        for file in FAVICON_FILES {
            let path = out_dir.join(file);
            if !path.exists() {
                fs::write(&path, b"").expect("Failed to create placeholder favicon");
            }
        }
    }

    generate_docs_site(&manifest_dir, &cargo_out_dir);

    println!("cargo:rerun-if-changed=build.rs");
    // Re-run when feature flag changes
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_BUILD_IMAGE");
}

fn generate_docs_site(manifest_dir: &Path, cargo_out_dir: &Path) {
    let docs_root = manifest_dir.join("../../docs");
    emit_docs_rerun_if_changed(&docs_root);

    let site = build_docs_site(&docs_root)
        .unwrap_or_else(|error| panic!("failed to build docs site: {error}"));
    let shared_css = site
        .pages
        .first()
        .map(|page| page.css.clone())
        .unwrap_or_default();

    for page in &site.pages {
        assert_eq!(
            page.css, shared_css,
            "expected docs pages to share one CSS bundle"
        );
    }

    let docs_out_dir = cargo_out_dir.join("docs-site");
    let pages_out_dir = docs_out_dir.join("pages");
    fs::create_dir_all(&pages_out_dir).expect("failed to create docs-site/pages");
    fs::write(docs_out_dir.join("docs.css"), shared_css).expect("failed to write docs.css");

    for page in &site.pages {
        let artifact_name = artifact_name_for_public_path(&page.public_path);
        fs::write(
            pages_out_dir.join(format!("{artifact_name}.html")),
            &page.html,
        )
        .unwrap_or_else(|error| {
            panic!(
                "failed to write docs artifact for {}: {error}",
                page.public_path
            )
        });
    }

    let generated_module = render_docs_module(&site);
    fs::write(
        cargo_out_dir.join("docs_site_generated.rs"),
        generated_module,
    )
    .expect("failed to write generated docs module");
}

fn emit_docs_rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap_or_else(|error| {
            panic!("failed to read docs directory {}: {error}", path.display());
        }) {
            let entry = entry.unwrap_or_else(|error| {
                panic!("failed to read entry under {}: {error}", path.display());
            });
            emit_docs_rerun_if_changed(&entry.path());
        }
    }
}

fn artifact_name_for_public_path(public_path: &str) -> String {
    let trimmed = public_path
        .trim_start_matches("/docs")
        .trim_matches('/')
        .replace('/', "__");
    if trimmed.is_empty() {
        "index".to_string()
    } else {
        trimmed
    }
}

fn render_docs_module(site: &DocsSite) -> String {
    use std::fmt::Write as _;

    let mut output = String::new();
    output.push_str("pub const DOCS_CSS: &str = include_str!(concat!(env!(\"OUT_DIR\"), \"/docs-site/docs.css\"));\n");
    output.push_str("pub const DOCS_NAV: BuiltDocsNav = BuiltDocsNav {\n");
    writeln!(output, "    title: {},", rust_str(&site.nav.title)).unwrap();
    output.push_str("    groups: &[\n");
    for group in &site.nav.groups {
        output.push_str("        BuiltDocsNavGroup {\n");
        writeln!(output, "            title: {},", rust_str(&group.title)).unwrap();
        writeln!(
            output,
            "            description: {},",
            rust_option_str(group.description.as_deref())
        )
        .unwrap();
        output.push_str("            entries: &[\n");
        for entry in &group.entries {
            output.push_str("                BuiltDocsNavEntry {\n");
            writeln!(
                output,
                "                    path: {},",
                rust_str(&entry.path)
            )
            .unwrap();
            writeln!(
                output,
                "                    label: {},",
                rust_str(&entry.label)
            )
            .unwrap();
            output.push_str("                },\n");
        }
        output.push_str("            ],\n");
        output.push_str("        },\n");
    }
    output.push_str("    ],\n");
    output.push_str("};\n\n");

    output.push_str("pub const DOCS_PAGES: &[BuiltDocsPage] = &[\n");
    for page in &site.pages {
        let artifact_name = artifact_name_for_public_path(&page.public_path);
        output.push_str("    BuiltDocsPage {\n");
        writeln!(
            output,
            "        public_path: {},",
            rust_str(&page.public_path)
        )
        .unwrap();
        writeln!(output, "        title: {},", rust_str(&page.title)).unwrap();
        writeln!(
            output,
            "        description: {},",
            rust_str(&page.description)
        )
        .unwrap();
        writeln!(
            output,
            "        html: include_str!(concat!(env!(\"OUT_DIR\"), \"/docs-site/pages/{}.html\")),",
            artifact_name
        )
        .unwrap();
        writeln!(
            output,
            "        kind: BuiltDocsPageKind::{},",
            match page.kind {
                DocsPageKind::Guide => "Guide",
                DocsPageKind::Adr => "Adr",
                DocsPageKind::AdrIndex => "AdrIndex",
            }
        )
        .unwrap();
        output.push_str("        headings: &[\n");
        for heading in &page.headings {
            output.push_str("            BuiltDocsHeading {\n");
            writeln!(output, "                id: {},", rust_str(&heading.id)).unwrap();
            writeln!(
                output,
                "                title: {},",
                rust_str(&heading.title)
            )
            .unwrap();
            writeln!(output, "                level: {},", heading.level).unwrap();
            output.push_str("            },\n");
        }
        output.push_str("        ],\n");
        writeln!(
            output,
            "        tags: {},",
            rust_str_slice(&page.tags.iter().map(String::as_str).collect::<Vec<_>>())
        )
        .unwrap();
        writeln!(
            output,
            "        status: {},",
            rust_option_str(page.status.as_deref())
        )
        .unwrap();
        writeln!(
            output,
            "        decision_date: {},",
            rust_option_str(page.decision_date.as_deref())
        )
        .unwrap();
        output.push_str("    },\n");
    }
    output.push_str("];\n\n");

    output.push_str("pub const DOCS_ADRS: &[BuiltDocsAdrSummary] = &[\n");
    for adr in &site.adrs {
        output.push_str("    BuiltDocsAdrSummary {\n");
        writeln!(output, "        path: {},", rust_str(&adr.path)).unwrap();
        writeln!(output, "        title: {},", rust_str(&adr.title)).unwrap();
        writeln!(output, "        status: {},", rust_str(&adr.status)).unwrap();
        writeln!(
            output,
            "        decision_date: {},",
            rust_str(&adr.decision_date)
        )
        .unwrap();
        writeln!(
            output,
            "        tags: {},",
            rust_str_slice(&adr.tags.iter().map(String::as_str).collect::<Vec<_>>())
        )
        .unwrap();
        output.push_str("    },\n");
    }
    output.push_str("];\n");

    output
}

fn rust_str(value: &str) -> String {
    format!("{value:?}")
}

fn rust_option_str(value: Option<&str>) -> String {
    value
        .map(rust_str)
        .map(|value| format!("Some({value})"))
        .unwrap_or_else(|| "None".to_string())
}

fn rust_str_slice(values: &[&str]) -> String {
    if values.is_empty() {
        "&[]".to_string()
    } else {
        format!(
            "&[{}]",
            values
                .iter()
                .map(|value| rust_str(value))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[cfg(feature = "build-image")]
fn generate_favicons(svg_path: &Path, out_dir: &Path, suffix: &str) {
    use image::{
        ExtendedColorType, ImageFormat, RgbaImage,
        codecs::ico::{IcoEncoder, IcoFrame},
    };
    use resvg::tiny_skia;
    use resvg::usvg;

    let svg_data = fs::read(svg_path).unwrap_or_else(|e| {
        panic!("failed to read {}: {e}", svg_path.display());
    });

    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt)
        .unwrap_or_else(|e| panic!("failed to parse svg {}: {e}", svg_path.display()));

    // PNG sizes to generate
    let png_jobs: &[(&str, u32)] = &[
        ("favicon{}-16x16.png", 16),
        ("favicon{}-32x32.png", 32),
        ("apple-touch-icon{}.png", 180),
        ("android-chrome{}-192x192.png", 192),
        ("android-chrome{}-512x512.png", 512),
    ];

    for (name_template, size) in png_jobs {
        let name = name_template.replace("{}", suffix);
        let img = render_svg_square_to_rgba(&tree, *size);
        let png_bytes = encode_png(&img);
        write_if_changed(&out_dir.join(&name), &png_bytes).unwrap_or_else(|e| {
            panic!("failed to write {name}: {e}");
        });
    }

    // favicon.ico (multi-size: 16, 32, 48)
    {
        let ico_sizes: &[u32] = &[16, 32, 48];
        let mut frames: Vec<IcoFrame> = Vec::new();

        for &size in ico_sizes {
            let img = render_svg_square_to_rgba(&tree, size);
            let png_bytes = encode_png(&img);
            let frame = IcoFrame::with_encoded(png_bytes, size, size, ExtendedColorType::Rgba8)
                .expect("failed to create ico frame");
            frames.push(frame);
        }

        let mut ico_bytes: Vec<u8> = Vec::new();
        IcoEncoder::new(&mut ico_bytes)
            .encode_images(&frames)
            .expect("failed to encode multi-size ico");

        let ico_name = format!("favicon{suffix}.ico");
        write_if_changed(&out_dir.join(&ico_name), &ico_bytes)
            .expect("failed to write favicon.ico");
    }

    fn render_svg_square_to_rgba(tree: &usvg::Tree, out_size: u32) -> RgbaImage {
        let svg_size = tree.size().to_int_size();
        let svg_w = svg_size.width() as f32;
        let svg_h = svg_size.height() as f32;

        let out_w = out_size as f32;
        let out_h = out_size as f32;

        // Aspect ratio preserving fit + centering
        let scale = (out_w / svg_w).min(out_h / svg_h);
        let drawn_w = svg_w * scale;
        let drawn_h = svg_h * scale;
        let tx = (out_w - drawn_w) / 2.0;
        let ty = (out_h - drawn_h) / 2.0;

        let mut pixmap =
            tiny_skia::Pixmap::new(out_size, out_size).expect("failed to create pixmap");
        pixmap.fill(tiny_skia::Color::from_rgba8(0, 0, 0, 0));

        // x' = scale*x + tx, y' = scale*y + ty
        let transform = tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);

        resvg::render(tree, transform, &mut pixmap.as_mut());

        // tiny-skia uses premultiplied RGBA, so demultiply before passing to image crate
        let mut rgba: Vec<u8> = Vec::with_capacity((out_size * out_size * 4) as usize);
        for p in pixmap.pixels() {
            let c: tiny_skia::ColorU8 = p.demultiply();
            rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }

        RgbaImage::from_raw(out_size, out_size, rgba).expect("invalid rgba buffer")
    }

    fn encode_png(img: &RgbaImage) -> Vec<u8> {
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img.clone())
            .write_to(&mut io::Cursor::new(&mut buf), ImageFormat::Png)
            .expect("failed to encode png");
        buf
    }
}

/// Write only if content changed (avoids unnecessary timestamp changes)
#[cfg(feature = "build-image")]
fn write_if_changed(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Ok(existing) = fs::read(path)
        && existing == bytes
    {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::File::create(path)?;
    f.write_all(bytes)?;
    Ok(())
}
