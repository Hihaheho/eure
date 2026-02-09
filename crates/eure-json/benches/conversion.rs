//! Benchmarks comparing query system overhead vs manual pipeline calls.
//!
//! This benchmark measures the overhead of the query-flow system by comparing:
//! 1. Query pipeline: Input -> CST -> EureDocument -> JSON via query runtime
//! 2. Manual pipeline: Direct function calls without query system
//!
//! Run with: cargo bench -p eure-json

use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use eure::document::{EureDocument, cst_to_document};
use eure::query::{TextFile, TextFileContent, build_runtime};
use eure_json::{Config, EureToJson, document_to_value};
use eure_parol::parse_tolerant;
use eure_tree::prelude::Cst;
use query_flow::DurabilityLevel;

// =============================================================================
// Test Data
// =============================================================================

/// Small input: simple object with a few fields
fn small_input() -> &'static str {
    r#"name = "Alice"
age = 30
active = true
"#
}

/// Medium input: nested structure with arrays
fn medium_input() -> &'static str {
    r#"$schema = "user.schema.eure"

name = "Alice"
email = "alice@example.com"
age = 30
active = true

profile {
    bio = "Software engineer"
    location = "Tokyo"
    website = "https://alice.dev"
}

tags[] = "rust"
tags[] = "typescript"
tags[] = "python"

contacts[] {
    type = "email"
    value = "alice@work.com"
    primary = true
}
contacts[] {
    type = "phone"
    value = "+1-555-1234"
    primary = false
}

metadata {
    created_at = "2024-01-15"
    updated_at = "2024-06-20"
    version = 3
}
"#
}

/// Large input: array with many elements using Eure section syntax
fn large_input() -> String {
    let mut s = String::new();
    for i in 0..100 {
        s.push_str(&format!(
            r#"items[] {{
    id = {i}
    name = "Item {i}"
    price = {price}
    in_stock = {in_stock}
    tags = ["tag-a", "tag-b", "tag-c"]
}}
"#,
            i = i,
            price = (i as f64) * 9.99,
            in_stock = i % 2 == 0
        ));
    }
    s
}

// =============================================================================
// Manual Pipeline (no query system)
// =============================================================================

/// Manual pipeline: parse -> document -> json without query system overhead
fn manual_pipeline(text: &str) -> serde_json::Value {
    // Step 1: Parse to CST
    let cst = parse_tolerant(text).cst();

    // Step 2: CST to EureDocument
    let doc = cst_to_document(text, &cst).expect("document construction should succeed");

    // Step 3: EureDocument to JSON
    document_to_value(&doc, &Config::default()).expect("json conversion should succeed")
}

/// Manual: parse only (Input -> CST)
fn manual_parse_only(text: &str) -> Cst {
    parse_tolerant(text).cst()
}

/// Manual: document only (CST -> EureDocument)
fn manual_document_only(text: &str, cst: &Cst) -> EureDocument {
    cst_to_document(text, cst).expect("document construction should succeed")
}

/// Manual: json only (EureDocument -> JSON)
fn manual_json_only(doc: &EureDocument) -> serde_json::Value {
    document_to_value(doc, &Config::default()).expect("json conversion should succeed")
}

// =============================================================================
// Query Pipeline (with query system)
// =============================================================================

/// Query pipeline: parse -> document -> json via query runtime
fn query_pipeline(text: &str) -> Arc<serde_json::Value> {
    let runtime = build_runtime();
    let file = TextFile::from_path(PathBuf::from("bench.eure"));
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(text.to_string()),
        DurabilityLevel::Static,
    );

    runtime
        .query(EureToJson::new(file, Config::default()))
        .expect("query should succeed")
}

/// Query pipeline with pre-built runtime (measures query execution only, not setup)
fn query_pipeline_with_runtime(
    runtime: &query_flow::QueryRuntime,
    file: &TextFile,
) -> Arc<serde_json::Value> {
    runtime
        .query(EureToJson::new(file.clone(), Config::default()))
        .expect("query should succeed")
}

// =============================================================================
// Benchmarks
// =============================================================================

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    // Small input
    let small = small_input();
    group.bench_with_input(BenchmarkId::new("manual", "small"), &small, |b, input| {
        b.iter(|| manual_pipeline(black_box(input)))
    });
    group.bench_with_input(BenchmarkId::new("query", "small"), &small, |b, input| {
        b.iter(|| query_pipeline(black_box(input)))
    });

    // Medium input
    let medium = medium_input();
    group.bench_with_input(BenchmarkId::new("manual", "medium"), &medium, |b, input| {
        b.iter(|| manual_pipeline(black_box(input)))
    });
    group.bench_with_input(BenchmarkId::new("query", "medium"), &medium, |b, input| {
        b.iter(|| query_pipeline(black_box(input)))
    });

    // Large input
    let large = large_input();
    group.bench_with_input(
        BenchmarkId::new("manual", "large"),
        large.as_str(),
        |b, input| b.iter(|| manual_pipeline(black_box(input))),
    );
    group.bench_with_input(
        BenchmarkId::new("query", "large"),
        large.as_str(),
        |b, input| b.iter(|| query_pipeline(black_box(input))),
    );

    group.finish();
}

fn bench_query_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_reuse");

    // This benchmark measures query execution with a pre-built runtime
    // to isolate the query overhead from runtime construction

    let small = small_input();
    let runtime = build_runtime();
    let file = TextFile::from_path(PathBuf::from("bench.eure"));
    runtime.resolve_asset(
        file.clone(),
        TextFileContent(small.to_string()),
        DurabilityLevel::Static,
    );

    group.bench_function("query_with_prebuilt_runtime/small", |b| {
        b.iter(|| query_pipeline_with_runtime(black_box(&runtime), black_box(&file)))
    });

    // Compare with manual for same input
    group.bench_function("manual/small", |b| {
        b.iter(|| manual_pipeline(black_box(small)))
    });

    group.finish();
}

fn bench_parse_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_phase");

    let small = small_input();
    let medium = medium_input();
    let large = large_input();

    group.bench_with_input(BenchmarkId::new("manual", "small"), &small, |b, input| {
        b.iter(|| manual_parse_only(black_box(input)))
    });
    group.bench_with_input(BenchmarkId::new("manual", "medium"), &medium, |b, input| {
        b.iter(|| manual_parse_only(black_box(input)))
    });
    group.bench_with_input(
        BenchmarkId::new("manual", "large"),
        large.as_str(),
        |b, input| b.iter(|| manual_parse_only(black_box(input))),
    );

    group.finish();
}

fn bench_document_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_phase");

    let small = small_input();
    let small_cst = manual_parse_only(small);

    let medium = medium_input();
    let medium_cst = manual_parse_only(medium);

    let large = large_input();
    let large_cst = manual_parse_only(&large);

    group.bench_function("manual/small", |b| {
        b.iter(|| manual_document_only(black_box(small), black_box(&small_cst)))
    });
    group.bench_function("manual/medium", |b| {
        b.iter(|| manual_document_only(black_box(medium), black_box(&medium_cst)))
    });
    group.bench_function("manual/large", |b| {
        b.iter(|| manual_document_only(black_box(&large), black_box(&large_cst)))
    });

    group.finish();
}

fn bench_json_phase(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_phase");

    let small = small_input();
    let small_cst = manual_parse_only(small);
    let small_doc = manual_document_only(small, &small_cst);

    let medium = medium_input();
    let medium_cst = manual_parse_only(medium);
    let medium_doc = manual_document_only(medium, &medium_cst);

    let large = large_input();
    let large_cst = manual_parse_only(&large);
    let large_doc = manual_document_only(&large, &large_cst);

    group.bench_function("manual/small", |b| {
        b.iter(|| manual_json_only(black_box(&small_doc)))
    });
    group.bench_function("manual/medium", |b| {
        b.iter(|| manual_json_only(black_box(&medium_doc)))
    });
    group.bench_function("manual/large", |b| {
        b.iter(|| manual_json_only(black_box(&large_doc)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_pipeline,
    bench_query_reuse,
    bench_parse_phase,
    bench_document_phase,
    bench_json_phase,
);
criterion_main!(benches);
