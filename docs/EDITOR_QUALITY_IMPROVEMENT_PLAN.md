# EURE Editor Support Quality Improvement Plan

**Date**: 2025-10-25
**Status**: Proposed
**Target**: `eure-ls` and `eure-editor-support` crates

## Executive Summary

This document outlines a comprehensive improvement plan for EURE's editor support infrastructure. Based on codebase analysis, we identified significant opportunities to improve code quality, reduce technical debt, and enhance maintainability.

**Current State**:
- Total Lines of Code: 4,850
- Test Coverage: 54% (eure-ls has 0% coverage)
- Code Duplication: 15-20%
- Critical Issues: 6 unwrap/expect, 67 eprintln!, 12+ TODOs

---

## Priority 1: Critical Quality Issues (High Priority)

### 1.1 Consolidate Duplicate Context Tracking Implementations

**Problem**: Three separate implementations solve the same problem:
- `completion_analyzer.rs` (327 lines)
- `path_context.rs` (445 lines)
- `completion_context_tracker.rs` (757 lines)

**Impact**:
- Maintenance burden
- Inconsistent behavior
- Harder to extend features

**Solution**:
- Analyze which implementation is most robust (likely `completion_context_tracker.rs`)
- Migrate all functionality to the chosen implementation
- Remove deprecated modules
- Update all call sites

**Estimated Effort**: 2-3 days

**Files to Modify**:
- `/crates/eure-editor-support/src/completion_analyzer.rs` (remove)
- `/crates/eure-editor-support/src/path_context.rs` (remove or merge)
- `/crates/eure-editor-support/src/completion_context_tracker.rs` (keep and enhance)
- `/crates/eure-editor-support/src/completions.rs` (update call sites)

---

### 1.2 Remove Dangerous `unwrap()` and `expect()` Calls

**Problem**: 6 instances of unsafe unwrapping that could cause panics:

1. **parser.rs:19**
   ```rust
   let cst = tree_builder.build().expect("TreeConstruction never fails");
   ```

2. **completions.rs:251**
   ```rust
   let field = parts.last().unwrap().to_string();
   ```

3. **completions.rs:930**
   ```rust
   let variants_key = KeyCmpValue::MetaExtension(Identifier::from_str("variants").unwrap());
   ```

4. **schema_validation.rs:234**
   ```rust
   let path_str = schema_ref.strip_prefix("file://").unwrap();
   ```

5. **schema_validation.rs:234**
   ```rust
   KeyCmpValue::Tuple(_) => todo!(), // CRITICAL: Unimplemented
   ```

**Solution**:
- Replace all `unwrap()` with proper error propagation using `?` operator
- Return `Result<T, EditorError>` types with descriptive errors
- Implement `KeyCmpValue::Tuple` variant handling
- Add error recovery strategies for each case

**Estimated Effort**: 1-2 days

**Files to Modify**:
- `/crates/eure-editor-support/src/parser.rs`
- `/crates/eure-editor-support/src/completions.rs`
- `/crates/eure-editor-support/src/schema_validation.rs`

---

### 1.3 Replace Debug Output with Proper Logging

**Problem**: 67 `eprintln!()` statements pollute stderr

**Current Pattern**:
```rust
eprintln!("DEBUG: Context type determined: {context:?}");
eprintln!("DEBUG: Byte offset: {}, Input length: {}", byte_offset, self.input.len());
```

**Solution**:
- Integrate `tracing` crate for structured logging
- Define log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Make logging configurable via environment variables
- Remove all `eprintln!()` statements

**Implementation**:
```rust
use tracing::{debug, info, warn, error};

// Replace eprintln! with:
debug!("Context type determined: {:?}", context);
debug!(byte_offset = %byte_offset, input_len = %self.input.len(), "Processing position");
```

**Estimated Effort**: 1 day

**Files to Modify**:
- `Cargo.toml` (add `tracing` dependency)
- `/crates/eure-ls/src/main.rs` (initialize tracing subscriber)
- `/crates/eure-editor-support/src/completion_analyzer.rs` (67 eprintln! calls)
- `/crates/eure-editor-support/src/completions.rs`
- Other affected files

---

### 1.4 Add Test Coverage for LSP Server

**Problem**: `eure-ls` (671 lines) has **zero tests**

**Required Test Categories**:

1. **Request/Response Tests**
   - Semantic tokens request
   - Completion request
   - Diagnostic request

2. **Document Lifecycle Tests**
   - Document open/change/close
   - Cache invalidation
   - Version tracking

3. **Error Recovery Tests**
   - Invalid JSON-RPC messages
   - Parser failures
   - Schema loading failures
   - Concurrent requests

4. **Schema Association Tests**
   - Schema discovery
   - Multi-document schemas
   - Schema reload

**Implementation Approach**:
```rust
// tests/lsp_tests.rs
#[test]
fn test_semantic_tokens_request() {
    let (server, client) = setup_test_server();
    // Send semantic tokens request
    // Verify response format
}
```

**Estimated Effort**: 2-3 days

**New Files**:
- `/crates/eure-ls/tests/lsp_tests.rs`
- `/crates/eure-ls/tests/common/mod.rs` (test helpers)

---

## Priority 2: Performance Improvements (Medium Priority)

### 2.1 Reduce Excessive Cloning

**Problem**: 9+ clone operations in hot paths (main.rs)

**Current Pattern**:
```rust
// eure-ls/src/main.rs:145-166
legend: legend.clone(),
req.clone(),
uri_string.clone(),
cst.clone(),
text.clone()  // Cloning entire CST for each operation
```

**Solution**:
- Use `Arc<>` for shared large structures (CST, schemas)
- Pass references instead of owned values where possible
- Profile and measure impact

**Estimated Effort**: 1-2 days

**Files to Modify**:
- `/crates/eure-ls/src/main.rs`
- `/crates/eure-editor-support/src/schema_validation.rs`

---

### 2.2 Implement Incremental Synchronization

**Problem**: Only full document sync is supported

**Current Limitation**:
```rust
// main.rs:33
full: Some(SemanticTokensFullOptions::Delta { delta: Some(false) }),
text_document_sync: FULL // Inefficient for large documents
```

**Solution**:
- Implement `TextDocumentSyncKind::INCREMENTAL`
- Support delta semantic tokens
- Implement efficient diff-based CST updates

**Benefits**:
- Faster response for large files
- Reduced bandwidth
- Better user experience

**Estimated Effort**: 2-3 days

**Files to Modify**:
- `/crates/eure-ls/src/main.rs`
- `/crates/eure-editor-support/src/parser.rs`

---

### 2.3 Add Schema Caching

**Problem**: Schemas loaded repeatedly without caching

**Solution**:
- Implement `Arc<Schema>` caching in `SchemaManager`
- Add cache invalidation on schema file changes
- Watch schema files for changes (file system events)

**Estimated Effort**: 1-2 days

**Files to Modify**:
- `/crates/eure-editor-support/src/schema_validation.rs`

---

## Priority 3: Architecture Improvements (Low Priority)

### 3.1 Reduce Function Complexity

**Problem**: Large, complex functions that are hard to maintain

**Hot Spots**:
- `completions.rs::get_completions()` - 101 lines
- `main.rs::process_document()` - 214 lines
- `schema_validation.rs::validate_document()` - complex logic

**Solution**:
- Apply Extract Method refactoring
- Single Responsibility Principle
- Aim for functions < 50 lines

**Estimated Effort**: 2 days

**Files to Modify**:
- `/crates/eure-editor-support/src/completions.rs`
- `/crates/eure-ls/src/main.rs`
- `/crates/eure-editor-support/src/schema_validation.rs`

---

### 3.2 Decouple LSP Protocol from Business Logic

**Problem**: `ServerContext` mixes LSP protocol handling with document/schema management

**Current Architecture**:
```
main.rs (ServerContext)
  ├─ LSP request/response handling
  ├─ Document cache management
  ├─ Schema loading
  └─ Validation logic
```

**Proposed Architecture**:
```
LSP Layer (main.rs)
  └─> Service Layer (document_service.rs, schema_service.rs)
       └─> Domain Layer (eure-editor-support modules)
```

**Benefits**:
- Testable business logic without LSP protocol
- Clearer separation of concerns
- Easier to add alternative frontends (e.g., CLI tools)

**Estimated Effort**: 3-4 days

**New Files**:
- `/crates/eure-ls/src/services/document_service.rs`
- `/crates/eure-ls/src/services/schema_service.rs`
- `/crates/eure-ls/src/services/mod.rs`

---

### 3.3 Add Configuration System

**Problem**: No way to configure LSP behavior

**Desired Configuration**:
```rust
struct EditorConfig {
    // Schema settings
    schema_search_paths: Vec<PathBuf>,
    auto_schema_discovery: bool,

    // Completion settings
    max_completions: usize,
    trigger_characters: Vec<String>,

    // Logging
    log_level: LogLevel,
    log_file: Option<PathBuf>,

    // Performance
    incremental_sync: bool,
    cache_size_limit: usize,
}
```

**Implementation**:
- Support workspace configuration
- LSP `workspace/configuration` request
- `.eure/config.toml` file support

**Estimated Effort**: 1-2 days

**Files to Create**:
- `/crates/eure-ls/src/config.rs`

---

## Implementation Roadmap

### Phase 1: Stabilization (Week 1-2)
- [ ] 1.2: Remove unwrap/expect calls
- [ ] 1.3: Add proper logging
- [ ] 1.4: Add LSP server tests

### Phase 2: Consolidation (Week 3)
- [ ] 1.1: Consolidate context tracking
- [ ] 2.1: Reduce cloning

### Phase 3: Performance (Week 4-5)
- [ ] 2.2: Incremental sync
- [ ] 2.3: Schema caching

### Phase 4: Architecture (Week 6-8)
- [ ] 3.1: Reduce function complexity
- [ ] 3.2: Decouple LSP from business logic
- [ ] 3.3: Add configuration system

---

## Success Metrics

### Code Quality
- ✅ Zero `unwrap()/expect()` in production code
- ✅ Zero `eprintln!()` (replaced with `tracing`)
- ✅ Zero `todo!()` in critical paths
- ✅ < 5% code duplication

### Test Coverage
- ✅ `eure-ls`: > 70% coverage
- ✅ `eure-editor-support`: > 80% coverage
- ✅ All critical paths tested

### Performance
- ✅ Incremental sync support
- ✅ Schema caching implemented
- ✅ < 50 clone operations in hot paths

### Maintainability
- ✅ Average function length < 50 lines
- ✅ Clear separation of concerns
- ✅ Comprehensive documentation

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing functionality | High | Comprehensive test suite before refactoring |
| Performance regressions | Medium | Benchmark before/after changes |
| LSP protocol compliance issues | High | Test against multiple editors (VSCode, Neovim) |
| Increased complexity | Medium | Code review and architecture validation |

---

## References

- [Current Code Analysis](../EURE_PROJECT_STATUS_REPORT.md)
- [LSP Specification](https://microsoft.github.io/language-server-protocol/)
- [Rust Error Handling Best Practices](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Tracing Crate Documentation](https://docs.rs/tracing/)

---

## Notes

- This plan is a living document and should be updated as work progresses
- Each item should have corresponding GitHub issues/tracking
- Regular progress reviews recommended (bi-weekly)
- Consider incremental rollout with feature flags for risky changes
