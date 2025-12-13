//! Formatter configuration.

/// Configuration options for the Eure formatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatConfig {
    /// Maximum line width (guideline, not strict limit).
    /// Default: 100
    pub max_width: usize,

    /// Number of spaces per indentation level.
    /// Default: 2
    pub indent_width: usize,

    /// Use tabs instead of spaces for indentation.
    /// Default: false
    pub use_tabs: bool,

    /// Trailing comma policy for multiline arrays/objects.
    /// Default: TrailingComma::Always
    pub trailing_comma: TrailingComma,

    /// Newline style.
    /// Default: NewlineStyle::Lf
    pub newline: NewlineStyle,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            max_width: 100,
            indent_width: 2,
            use_tabs: false,
            trailing_comma: TrailingComma::Always,
            newline: NewlineStyle::Lf,
        }
    }
}

impl FormatConfig {
    /// Create a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max width.
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }

    /// Set indent width.
    pub fn with_indent_width(mut self, width: usize) -> Self {
        self.indent_width = width;
        self
    }

    /// Set tab usage.
    pub fn with_tabs(mut self, use_tabs: bool) -> Self {
        self.use_tabs = use_tabs;
        self
    }

    /// Set trailing comma policy.
    pub fn with_trailing_comma(mut self, policy: TrailingComma) -> Self {
        self.trailing_comma = policy;
        self
    }

    /// Set newline style.
    pub fn with_newline(mut self, style: NewlineStyle) -> Self {
        self.newline = style;
        self
    }
}

/// Trailing comma policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrailingComma {
    /// Always add trailing commas in multiline contexts.
    #[default]
    Always,
    /// Never add trailing commas.
    Never,
}

/// Newline style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewlineStyle {
    /// Unix-style line endings (LF).
    #[default]
    Lf,
    /// Windows-style line endings (CRLF).
    Crlf,
}

impl NewlineStyle {
    /// Get the newline string.
    pub fn as_str(self) -> &'static str {
        match self {
            NewlineStyle::Lf => "\n",
            NewlineStyle::Crlf => "\r\n",
        }
    }
}
