use image::{
    ExtendedColorType, ImageFormat, RgbaImage,
    codecs::ico::{IcoEncoder, IcoFrame},
};
use resvg::tiny_skia;
use resvg::usvg;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));

    // Ensures assets/tailwind.css exists not to fail the build for `asset!("/assets/tailwind.css")` on local or CI.
    let tailwind_css_path = manifest_dir.join("assets/tailwind.css");
    if !tailwind_css_path.exists() {
        fs::write(&tailwind_css_path, "").expect("Failed to create tailwind.css");
    }
    println!("cargo:rerun-if-changed={}", tailwind_css_path.display());

    // Generate favicons from SVG
    let out_dir = manifest_dir.join("assets");
    fs::create_dir_all(&out_dir).expect("failed to create assets/");

    // Light theme favicons
    let light_svg = manifest_dir.join("../../assets/icons/eure-icon-light.svg");
    if light_svg.exists() {
        println!("cargo:rerun-if-changed={}", light_svg.display());
        generate_favicons(&light_svg, &out_dir, "");
    }

    // Dark theme favicons
    let dark_svg = manifest_dir.join("../../assets/icons/eure-icon-dark.svg");
    if dark_svg.exists() {
        println!("cargo:rerun-if-changed={}", dark_svg.display());
        generate_favicons(&dark_svg, &out_dir, "-dark");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn generate_favicons(svg_path: &Path, out_dir: &Path, suffix: &str) {
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

    let mut pixmap = tiny_skia::Pixmap::new(out_size, out_size).expect("failed to create pixmap");
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

/// Write only if content changed (avoids unnecessary timestamp changes)
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
