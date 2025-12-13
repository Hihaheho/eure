//! Wadler-style pretty printer.
//!
//! This module implements the document-to-string rendering algorithm
//! based on Wadler's "A Prettier Printer" paper.

use crate::config::FormatConfig;
use crate::doc::Doc;

/// Print mode determines how Line/SoftLine are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Flat mode: Line becomes space, SoftLine becomes empty
    Flat,
    /// Break mode: Line and SoftLine become newlines
    Break,
}

/// A command on the printer's work stack.
#[derive(Debug, Clone)]
struct PrintCommand<'a> {
    /// Current indentation level
    indent: usize,
    /// Current print mode
    mode: Mode,
    /// Document to print
    doc: &'a Doc,
}

/// Pretty printer that renders Doc to String.
pub struct Printer {
    config: FormatConfig,
}

impl Printer {
    /// Create a new printer with the given configuration.
    pub fn new(config: FormatConfig) -> Self {
        Self { config }
    }

    /// Print a document to a string.
    pub fn print(&self, doc: &Doc) -> String {
        let mut output = String::new();
        let mut pos = 0; // Current column position
        let mut stack = vec![PrintCommand {
            indent: 0,
            mode: Mode::Break,
            doc,
        }];

        while let Some(cmd) = stack.pop() {
            match cmd.doc {
                Doc::Nil => {}

                Doc::Text(s) => {
                    output.push_str(s);
                    pos += s.chars().count();
                }

                Doc::Line => {
                    if cmd.mode == Mode::Flat {
                        output.push(' ');
                        pos += 1;
                    } else {
                        output.push('\n');
                        output.push_str(&self.indent_string(cmd.indent));
                        pos = cmd.indent * self.config.indent_width;
                    }
                }

                Doc::SoftLine => {
                    if cmd.mode == Mode::Flat {
                        // Empty in flat mode
                    } else {
                        output.push('\n');
                        output.push_str(&self.indent_string(cmd.indent));
                        pos = cmd.indent * self.config.indent_width;
                    }
                }

                Doc::HardLine => {
                    output.push('\n');
                    output.push_str(&self.indent_string(cmd.indent));
                    pos = cmd.indent * self.config.indent_width;
                }

                Doc::Indent(inner) => {
                    stack.push(PrintCommand {
                        indent: cmd.indent + 1,
                        mode: cmd.mode,
                        doc: inner,
                    });
                }

                Doc::Concat(left, right) => {
                    // Push right first so left is processed first (stack is LIFO)
                    stack.push(PrintCommand {
                        indent: cmd.indent,
                        mode: cmd.mode,
                        doc: right,
                    });
                    stack.push(PrintCommand {
                        indent: cmd.indent,
                        mode: cmd.mode,
                        doc: left,
                    });
                }

                Doc::Group(inner) => {
                    // Try flat mode first
                    if self.fits(cmd.indent, pos, inner) {
                        stack.push(PrintCommand {
                            indent: cmd.indent,
                            mode: Mode::Flat,
                            doc: inner,
                        });
                    } else {
                        stack.push(PrintCommand {
                            indent: cmd.indent,
                            mode: Mode::Break,
                            doc: inner,
                        });
                    }
                }

                Doc::IfBreak { flat, broken } => {
                    let chosen = if cmd.mode == Mode::Flat { flat } else { broken };
                    stack.push(PrintCommand {
                        indent: cmd.indent,
                        mode: cmd.mode,
                        doc: chosen,
                    });
                }
            }
        }

        output
    }

    /// Check if a document fits within the remaining line width.
    fn fits(&self, indent: usize, current_pos: usize, doc: &Doc) -> bool {
        let remaining = self.config.max_width.saturating_sub(current_pos);
        self.fits_within(remaining, indent, doc)
    }

    /// Check if a document fits within the given width.
    fn fits_within(&self, mut remaining: usize, indent: usize, doc: &Doc) -> bool {
        let mut stack = vec![(indent, doc)];

        while let Some((ind, d)) = stack.pop() {
            match d {
                Doc::Nil => {}

                Doc::Text(s) => {
                    let len = s.chars().count();
                    if len > remaining {
                        return false;
                    }
                    remaining -= len;
                }

                Doc::Line => {
                    // In flat mode, Line becomes a space
                    if remaining == 0 {
                        return false;
                    }
                    remaining -= 1;
                }

                Doc::SoftLine => {
                    // In flat mode, SoftLine becomes empty
                }

                Doc::HardLine => {
                    // HardLine always breaks, so the group doesn't fit flat
                    return false;
                }

                Doc::Indent(inner) => {
                    stack.push((ind + 1, inner));
                }

                Doc::Concat(left, right) => {
                    stack.push((ind, right));
                    stack.push((ind, left));
                }

                Doc::Group(inner) => {
                    // Nested groups also try flat mode when checking fit
                    stack.push((ind, inner));
                }

                Doc::IfBreak { flat, .. } => {
                    // When checking fit, we're in flat mode
                    stack.push((ind, flat));
                }
            }
        }

        true
    }

    /// Generate the indentation string.
    fn indent_string(&self, level: usize) -> String {
        let width = level * self.config.indent_width;
        if self.config.use_tabs {
            "\t".repeat(level)
        } else {
            " ".repeat(width)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print(doc: &Doc) -> String {
        Printer::new(FormatConfig::default()).print(doc)
    }

    fn print_width(doc: &Doc, width: usize) -> String {
        Printer::new(FormatConfig {
            max_width: width,
            ..Default::default()
        })
        .print(doc)
    }

    #[test]
    fn test_text() {
        let doc = Doc::text("hello");
        assert_eq!(print(&doc), "hello");
    }

    #[test]
    fn test_concat() {
        let doc = Doc::text("hello").concat(Doc::text(" world"));
        assert_eq!(print(&doc), "hello world");
    }

    #[test]
    fn test_line_in_break_mode() {
        let doc = Doc::text("a").concat(Doc::line()).concat(Doc::text("b"));
        // Without group, always in break mode
        assert_eq!(print(&doc), "a\nb");
    }

    #[test]
    fn test_group_fits() {
        let doc = Doc::group(Doc::text("a").concat(Doc::line()).concat(Doc::text("b")));
        // Short enough to fit on one line
        assert_eq!(print_width(&doc, 80), "a b");
    }

    #[test]
    fn test_group_breaks() {
        let doc = Doc::group(
            Doc::text("hello")
                .concat(Doc::line())
                .concat(Doc::text("world")),
        );
        // Too narrow, must break
        assert_eq!(print_width(&doc, 5), "hello\nworld");
    }

    #[test]
    fn test_indent() {
        // Indent wraps the newline to get indented content
        let doc = Doc::text("a").concat(Doc::indent(Doc::hardline().concat(Doc::text("b"))));
        assert_eq!(print(&doc), "a\n  b");
    }

    #[test]
    fn test_softline_flat() {
        let doc = Doc::group(
            Doc::text("[")
                .concat(Doc::softline())
                .concat(Doc::text("]")),
        );
        // Fits, so softline becomes empty
        assert_eq!(print_width(&doc, 80), "[]");
    }

    #[test]
    fn test_softline_break() {
        let doc = Doc::group(
            Doc::text("[")
                .concat(Doc::softline())
                .concat(Doc::text("very_long_content"))
                .concat(Doc::softline())
                .concat(Doc::text("]")),
        );
        // Doesn't fit, softline becomes newline
        assert_eq!(print_width(&doc, 10), "[\nvery_long_content\n]");
    }

    #[test]
    fn test_hardline_forces_break() {
        let doc = Doc::group(
            Doc::text("a")
                .concat(Doc::hardline())
                .concat(Doc::text("b")),
        );
        // HardLine forces break even in group
        assert_eq!(print_width(&doc, 80), "a\nb");
    }

    #[test]
    fn test_if_break() {
        let trailing_comma = Doc::if_break(Doc::Nil, Doc::text(","));

        let doc = Doc::group(
            Doc::text("[")
                .concat(Doc::softline())
                .concat(Doc::text("a"))
                .concat(trailing_comma.clone())
                .concat(Doc::softline())
                .concat(Doc::text("]")),
        );

        // Fits: no trailing comma
        assert_eq!(print_width(&doc, 80), "[a]");

        // Doesn't fit: trailing comma added
        let doc = Doc::group(
            Doc::text("[")
                .concat(Doc::softline())
                .concat(Doc::text("very_long_item"))
                .concat(trailing_comma)
                .concat(Doc::softline())
                .concat(Doc::text("]")),
        );
        assert_eq!(print_width(&doc, 10), "[\nvery_long_item,\n]");
    }

    #[test]
    fn test_nested_indent() {
        // Nested indentation: each indent wraps its newline
        let inner = Doc::indent(Doc::hardline().concat(Doc::text("c")));
        let outer = Doc::indent(Doc::hardline().concat(Doc::text("b")).concat(inner));
        let doc = Doc::text("a").concat(outer);

        assert_eq!(print(&doc), "a\n  b\n    c");
    }
}
