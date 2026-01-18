use query_flow::tracer::{ExecutionResult, SpanContext, SpanId, TraceId, Tracer, TracerAssetState};
use query_flow::{AssetCacheKey, QueryCacheKey};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use web_time::{Duration, Instant};

/// A single trace entry in the buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceEntry {
    /// Monotonic sequence number within buffer
    pub seq: u64,
    /// Time since app start
    pub timestamp: Duration,
    /// Wall clock datetime string (HH:MM:SS.mmm)
    pub datetime: String,
    /// The trace event
    pub event: TraceEvent,
}

/// Asset state for display.
#[derive(Debug, Clone, PartialEq)]
pub enum AssetState {
    Loading,
    Ready,
    NotFound,
}

impl From<TracerAssetState> for AssetState {
    fn from(state: TracerAssetState) -> Self {
        match state {
            TracerAssetState::Loading => AssetState::Loading,
            TracerAssetState::Ready => AssetState::Ready,
            TracerAssetState::NotFound => AssetState::NotFound,
        }
    }
}

/// Types of trace events we capture.
#[derive(Debug, Clone, PartialEq)]
pub enum TraceEvent {
    QueryEnd {
        span_id: SpanId,
        trace_id: TraceId,
        parent_span_id: Option<SpanId>,
        query_type: String,
        cache_key: String,
        result: ExecutionResult,
        duration_ms: f64,
    },
    AssetRequested {
        span_id: SpanId,
        trace_id: TraceId,
        parent_span_id: Option<SpanId>,
        asset_key: String,
        state: AssetState,
        duration_ms: f64,
    },
}

/// A trace tree containing all entries for a single trace.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceTree {
    /// The trace ID for this tree
    pub trace_id: TraceId,
    /// All entries in this trace
    pub entries: Vec<TraceEntry>,
}

struct TraceBufferInner {
    /// Stored as a list of trace trees (grouped by TraceId)
    trees: Vec<TraceTree>,
    /// Maximum number of trees to keep
    max_trees: usize,
    seq_counter: u64,
    start_time: Instant,
}

/// Thread-safe circular buffer for trace trees.
#[derive(Clone)]
pub struct TraceBuffer {
    inner: Arc<Mutex<TraceBufferInner>>,
}

impl TraceBuffer {
    /// Create a new trace buffer with the given maximum number of trees.
    pub fn new(max_trees: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TraceBufferInner {
                trees: Vec::new(),
                max_trees,
                seq_counter: 0,
                start_time: Instant::now(),
            })),
        }
    }

    /// Push a new trace event into the buffer.
    pub fn push(&self, trace_id: TraceId, event: TraceEvent) {
        let mut inner = self.inner.lock().unwrap();
        let seq = inner.seq_counter;
        inner.seq_counter += 1;

        let timestamp = inner.start_time.elapsed();

        // Get current datetime from JS Date
        let date = js_sys::Date::new_0();
        let datetime = format!(
            "{:02}:{:02}:{:02}.{:03}",
            date.get_hours(),
            date.get_minutes(),
            date.get_seconds(),
            date.get_milliseconds()
        );

        let entry = TraceEntry {
            seq,
            timestamp,
            datetime,
            event,
        };

        // Find or create the tree for this trace_id
        if let Some(tree) = inner.trees.iter_mut().find(|t| t.trace_id == trace_id) {
            tree.entries.push(entry);
        } else {
            // New trace tree - check if we need to remove old ones
            if inner.trees.len() >= inner.max_trees {
                inner.trees.remove(0);
            }
            inner.trees.push(TraceTree {
                trace_id,
                entries: vec![entry],
            });
        }
    }

    /// Get all trace entries flattened.
    pub fn get_all(&self) -> Vec<TraceEntry> {
        self.inner
            .lock()
            .unwrap()
            .trees
            .iter()
            .flat_map(|t| t.entries.clone())
            .collect()
    }

    /// Get the number of trees.
    pub fn tree_count(&self) -> usize {
        self.inner.lock().unwrap().trees.len()
    }

    /// Clear all trace entries.
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.trees.clear();
        inner.seq_counter = 0;
    }
}

/// Tracer implementation for eure-dev that collects query execution events.
pub struct EureDevTracer {
    buffer: TraceBuffer,
    span_counter: AtomicU64,
    trace_counter: AtomicU64,
    start_times: Arc<Mutex<HashMap<SpanId, Instant>>>,
}

impl EureDevTracer {
    pub fn new(buffer: TraceBuffer) -> Self {
        Self {
            buffer,
            span_counter: AtomicU64::new(0),
            trace_counter: AtomicU64::new(0),
            start_times: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Tracer for EureDevTracer {
    fn new_span_id(&self) -> SpanId {
        let id = self.span_counter.fetch_add(1, Ordering::SeqCst);
        SpanId(id)
    }

    fn new_trace_id(&self) -> TraceId {
        let id = self.trace_counter.fetch_add(1, Ordering::SeqCst);
        TraceId(id)
    }

    fn on_query_start(&self, ctx: &SpanContext, _query: &QueryCacheKey) {
        // Record start time for duration calculation
        self.start_times
            .lock()
            .unwrap()
            .insert(ctx.span_id, Instant::now());
    }

    fn on_query_end(&self, ctx: &SpanContext, query: &QueryCacheKey, result: ExecutionResult) {
        // Calculate duration
        let duration_ms = self
            .start_times
            .lock()
            .unwrap()
            .remove(&ctx.span_id)
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        // Shorten query_type: "eure::query::parse::ParseCst" -> "ParseCst"
        let query_type_full = query.type_name();
        let query_type = query_type_full
            .rfind("::")
            .map(|pos| &query_type_full[pos + 2..])
            .unwrap_or(query_type_full)
            .to_string();

        // Use the debug representation directly (now formatted nicely via #[query(debug = "...")])
        let cache_key = query.debug_repr();

        // Push end event (grouped by trace_id)
        self.buffer.push(
            ctx.trace_id,
            TraceEvent::QueryEnd {
                span_id: ctx.span_id,
                trace_id: ctx.trace_id,
                parent_span_id: ctx.parent_span_id,
                query_type,
                cache_key,
                result,
                duration_ms,
            },
        );
    }

    fn on_asset_requested(&self, ctx: &SpanContext, _asset: &AssetCacheKey) {
        // Record start time for duration calculation (like on_query_start)
        self.start_times
            .lock()
            .unwrap()
            .insert(ctx.span_id, Instant::now());
    }

    fn on_asset_located(&self, ctx: &SpanContext, asset: &AssetCacheKey, state: TracerAssetState) {
        // Calculate duration (like on_query_end)
        let duration_ms = self
            .start_times
            .lock()
            .unwrap()
            .remove(&ctx.span_id)
            .map(|start| start.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        self.buffer.push(
            ctx.trace_id,
            TraceEvent::AssetRequested {
                span_id: ctx.span_id,
                trace_id: ctx.trace_id,
                parent_span_id: ctx.parent_span_id,
                asset_key: asset.debug_repr(),
                state: state.into(),
                duration_ms,
            },
        );
    }
}
