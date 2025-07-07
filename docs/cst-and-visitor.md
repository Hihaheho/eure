# CST and Visitor

## Overview

The `eure-tree` crate provides a Concrete Syntax Tree (CST) representation for Eure documents, along with a powerful Visitor pattern API for traversing and analyzing the tree. This structure is generated automatically from the Eure grammar definition (`eure.parol`) using the `eure-parol-gen` tool, ensuring that the API always stays synchronized with the language grammar.

This document focuses on the design of the visitor API, the generated code structure, and what you can achieve using these tools.

## Generated Code (`eure-tree`)

The `eure-parol-gen` crate generates two key files within `eure-tree/src`:

1. `nodes.rs`: Defines the structural representation of the CST nodes.
2. `visitor.rs`: Defines the `CstVisitor` trait and related components.

### Node Handles and Views (`nodes.rs`)

For each element defined in the Eure grammar (`eure.parol`), `eure-parol-gen` generates corresponding types in `nodes.rs`:

* **Terminal Nodes:** For each terminal symbol (like `Ident`, `Integer`, `LBrace`), a simple struct wrapping a `CstNodeId` is generated (e.g., `struct Ident(CstNodeId)`). These structs implement the `TerminalHandle` trait, providing access to the node's ID and `TerminalKind`. The actual text content associated with the terminal can be retrieved using the `CstFacade` (like the `Cst` itself) and the node ID.
* **Non-Terminal Nodes:** For each non-terminal rule (like `Array`, `Binding`, `Value`), two types are generated:
  * **Handle:** A struct named `<RuleName>Handle` (e.g., `ArrayHandle(CstNodeId)`). This is a lightweight identifier for the specific node in the tree and implements the `NonTerminalHandle` trait.
  * **View:** A struct or enum named `<RuleName>View` (e.g., `ArrayView`, `ValueView`). This provides structured access to the children of the non-terminal node, typically containing Handles for those children. The structure of the View directly reflects the grammar rule:
    * **Sequences:** The View is a struct with fields for each element in the sequence (e.g., `BindingView { keys: KeysHandle, binding_rhs: BindingRhsHandle }`).
    * **Choices (Alternation):** The View is an enum with variants for each possible choice (e.g., `ValueView::Object(ObjectHandle)`).
    * **Repetitions/Lists (Recursion):** The View is often a struct containing a handle to the item and a handle to the next element in the list (e.g., `ArrayListView { value: ValueHandle, array_opt: ArrayOptHandle, array_list: ArrayListHandle }`). These views often implement the `RecursiveView` trait, providing helper methods like `get_all_with_visit` for easy iteration over list items.
    * **Options:** The corresponding `Handle`'s associated `View` type is an `Option` wrapping the handle of the optional child (e.g., `ArrayOptHandle` has `type View = Option<CommaHandle>`).

This generation ensures that the Rust types accurately represent the Eure grammar structure.

### Visitor Traits (`visitor.rs`)

The `eure-parol-gen` tool also generates the core visitor traits:

* **`CstVisitor<F: CstFacade>`:** This is the main trait users implement to traverse the CST. It contains methods for visiting each node type:
  * `visit_<RuleName>(&mut self, handle: <RuleName>Handle, view: <RuleName>View, tree: &F)`: Called when entering a non-terminal node. Receives the node's `Handle` and its `View` (providing access to children).
  * `visit_<TerminalName>_terminal(&mut self, terminal: <TerminalName>, data: TerminalData, tree: &F)`: Called when visiting a terminal node. Receives the terminal's `Handle` and associated data.
  * Generic methods like `visit_non_terminal`, `visit_terminal`, and `visit_non_terminal_close`.
* **`CstVisitorSuper<F: CstFacade, E>`:** This trait defines the default traversal logic. The default implementations of the `visit_*` methods in `CstVisitor` call corresponding `visit_*_super` methods defined in this trait. This allows users to override specific `visit_*` methods in their `CstVisitor` implementation without needing to manually implement the logic for descending into children.

### Error Handling and Recovery

When a view constructor (`get_view_with_visit`) fails (e.g., due to missing children), the generated visitor infrastructure invokes `then_construct_error(...)` on your `CstVisitor` implementation. The default implementation of `then_construct_error` simply calls `self.recover_error(...)`.

The `recover_error` method, defined in the `CstVisitorSuper` trait, attempts to continue traversal by visiting any children found directly under the problematic node (using `tree.children(id)`), even if the expected structure for the `View` was not met. You can customize the behavior upon construction failure by overriding `then_construct_error` in your visitor:

```rust, ignore
impl MyVisitor { // Assuming `MyVisitor` implements `CstVisitor<Cst>`
    fn then_construct_error(
        &mut self,
        node_data: Option<CstNode>,
        parent: CstNodeId,
        kind: NodeKind,
        error: CstConstructError,
        tree: &Cst,
    ) -> Result<(), Self::Error> {
        eprintln!("CST construction error for {:?} at {:?}: {:?}", kind, parent, error);
        // Choose to stop traversal by returning the error, or recover:
        // return Err(error.into()); // Example: Stop traversal
        // Or call recover_error explicitly or do custom recovery
        // self.recover_error(node_data, parent, kind, tree) // Default recovery behavior
        Ok(()) // Example: Log and continue (default effective behavior)
    }
}
```

### Collecting Nodes with `collect_nodes`

The generated `Handle` types internally use the `CstFacade::collect_nodes` method (defined in the `CstFacade` trait) to gather expected child node IDs based on their `NodeKind`. For example, `ArrayHandle` uses it to find the `ArrayBegin`, `ArrayList`, and `ArrayEnd` children needed to construct its `ArrayView`.

```rust, ignore
// Example internal call within ArrayHandle::get_view_with_visit
tree.collect_nodes(
    self.0, // parent node ID
    [
        NodeKind::NonTerminal(NonTerminalKind::ArrayBegin),
        NodeKind::NonTerminal(NonTerminalKind::ArrayList),
        NodeKind::NonTerminal(NonTerminalKind::ArrayEnd),
    ], // Expected child kinds in order
    // Closure to construct the View from found child IDs
    |[begin_id, list_id, end_id], visitor| {
        let view = ArrayView {
            array_begin: ArrayBeginHandle(begin_id),
            array_list: ArrayListHandle(list_id),
            array_end: ArrayEndHandle(end_id),
        };
        Ok((view, visitor))
    },
    visitor, // The visitor being used (passed through)
)
```

While typically used indirectly via `Handle` and `View` types, you can call `tree.collect_nodes(...)` directly if you need to implement custom node collection logic or bypass the generated view structures for specific purposes.

### Handling Whitespace, Newlines, and Comment Tokens

The grammar includes explicit terminals for whitespace, newlines, and comments. The visitor API provides methods:

* `visit_ws_terminal(&mut self, terminal: Ws, ...)`
* `visit_new_line_terminal(&mut self, terminal: NewLine, ...)`
* `visit_line_comment_terminal(&mut self, terminal: LineComment, ...)`
* `visit_block_comment_terminal(&mut self, terminal: BlockComment, ...)`

Additionally, if these terminals are wrapped in non-terminal rules (e.g., a rule `OptionalWhitespace: Ws?`), corresponding `visit_optional_whitespace(...)` methods will also exist.

You can override these specific `visit_*` methods in your `CstVisitor` implementation to process, analyze, or ignore these tokens. The default implementations provided via `CstVisitorSuper` generally do nothing besides potentially calling the generic `visit_terminal_super` or `visit_non_terminal_super`, effectively skipping them unless you provide an override.

```rust, ignore
impl MyVisitor {
    fn visit_line_comment_terminal(
        &mut self,
        comment: LineComment,
        data: TerminalData,
        tree: &Cst,
    ) -> Result<(), Self::Error> {
        let full_text = tree.text(comment.node_id()); // e.g., "# This is a comment\n"
        let content = full_text.trim_start_matches('#').trim_end();
        println!("Found comment: {}", content);
        // No need to call _super if you don't want default behavior (which is usually none)
        Ok(())
    }

    fn visit_ws(&mut self, handle: WsHandle, view: WsView, tree: &Cst) -> Result<(), Self::Error> {
        // Example: Completely ignore Ws non-terminals by doing nothing
        // Do not call self.visit_ws_super(handle, view, tree)
        Ok(())
    }
}
```

This allows fine-grained control over how syntactically-present but often semantically-ignored tokens are handled during traversal.

## API Design and Capabilities

The visitor pattern implemented in `eure-tree` allows for robust and type-safe traversal of the Eure CST.

### How to Use

1. **Create a struct:** Define your custom visitor struct.
2. **Implement `CstVisitor<Cst>`:** Implement the trait for your struct. You only need to override the `visit_*` methods for the node types you are interested in. The default methods handle the tree traversal.
3. **Implement `CstVisitorSuper` (implicitly):** By implementing `CstVisitor`, your struct implicitly gains the default traversal logic defined in `CstVisitorSuper`.
4. **Run the visitor:** Instantiate your visitor and call the appropriate traversal method on the `Cst` object (e.g., using methods provided by `Cst` or helper functions that take a visitor).

```rust, ignore
use eure_tree::{Cst, CstVisitor, CstVisitorSuper, nodes::*, tree::{CstFacade, TerminalData, NonTerminalData, CstNodeId}, node_kind::{TerminalKind, NonTerminalKind}, CstConstructError};

struct MyVisitor {
    // Visitor state, e.g., collected data
    ident_count: usize,
}

impl CstVisitor<Cst> for MyVisitor {
    type Error = std::convert::Infallible; // Or your custom error type

    // Override visit method for Ident terminal
    fn visit_ident_terminal(
        &mut self,
        terminal: Ident, // Handle for the Ident terminal
        data: TerminalData,
        tree: &Cst,
    ) -> Result<(), Self::Error> {
        self.ident_count += 1;
        println!("Found identifier: {}", tree.text(terminal.node_id()));
        // Call super to maintain default behavior (if any)
        self.visit_ident_terminal_super(terminal, data, tree)
    }

    // Override visit method for Array non-terminal
    fn visit_array(
        &mut self,
        handle: ArrayHandle, // Handle for the Array node
        view: ArrayView,     // View providing access to children (ArrayBegin, ArrayList, ArrayEnd)
        tree: &Cst,
    ) -> Result<(), Self::Error> {
        println!("Entering an array");
        // Default behavior: visit children. Call the _super method.
        self.visit_array_super(handle, view, tree)
        // If you wanted to *stop* traversal into arrays, you would *not* call visit_array_super.
    }

    // You don't need to implement methods for nodes you don't care about.
    // The default implementation from CstVisitorSuper will handle traversal.
}

fn visit(cst: &Cst) -> Result<(), CstConstructError> {
    let mut visitor = MyVisitor { ident_count: 0 };
    cst.root().visit(&cst, &mut visitor).unwrap(); // Start traversal from the root
    println!("Total identifiers found: {}", visitor.ident_count);
    Ok(())
}
```

### What Can Be Achieved?

* **Code Analysis:** Count occurrences of specific nodes, check for specific patterns, validate semantic rules.
* **Data Extraction:** Extract all key-value pairs, string literals, or specific sections from a Eure document.
* **Transformation:** Convert the CST into an Abstract Syntax Tree (AST), JSON, or any other format.
* **Code Generation:** Generate code or configuration based on the Eure input.
* **Linting/Formatting:** Implement custom linting rules or code formatting logic by analyzing node positions and content.

The combination of auto-generated, type-safe node access (`Handle` and `View`) and the flexible `CstVisitor` trait provides a powerful foundation for interacting with Eure CSTs.

## ValueVisitor: Extracting Values from CST

While the `CstVisitor` trait provides a generic way to traverse the CST, the `ValueVisitor` struct (in `value_visitor.rs`) provides a specialized implementation for extracting structured data from Eure documents.

### Overview

`ValueVisitor` implements `CstVisitor` to build an `EureDocument` - a path-based document structure that represents the hierarchical data in your Eure file. The key architectural point is that **`ValueVisitor` produces `EureDocument`, not `Value` directly**. This design preserves CST handles for span tracking and error reporting.

The visitor handles all the complexity of:

- Building paths from keys (including array syntax like `items[0]`)
- Managing nested sections and bindings
- Handling extensions (`$variant`, `$$meta`)
- Converting CST nodes to appropriate value types
- Variant transformation for external representation
- **Preserving CST handles** in the `EureDocument` for span tracking

### Architecture

The correct data flow is:

1. **ValueVisitor traverses the CST** and builds an `EureDocument`
2. **EureDocument preserves CST handles** for span tracking and diagnostics
3. **EureDocument can be converted to Value** when spans aren't needed

This architecture ensures that:
- Error messages can point to exact locations in the source
- The schema validator can track field spans for diagnostics
- The conversion to `Value` is explicit and intentional

### Basic Usage

```rust
use eure_tree::value_visitor::ValueVisitor;

// Create a visitor with your input text
let mut visitor = ValueVisitor::new(input);

// Visit the CST (typically done by the parser)
visitor.visit_eure(eure_handle, eure_view, &tree)?;

// Extract the final document (NOT a Value!)
let document: EureDocument = visitor.into_document();

// When you need a Value (losing span information):
let value = document_to_value(document);

// Or use the document directly for operations that need spans:
schema_validator.validate(&document); // Can report exact error locations
```

### Key Features

1. **Path-Based Construction**: Instead of manually building nested structures, `ValueVisitor` uses `EureDocument`'s path-based API:
   ```rust
   // Internally, bindings like `user.name = "Alice"` become:
   document.insert_node(vec![PathSegment::Ident("user"), PathSegment::Ident("name")], Value::String("Alice"))
   ```

2. **Array Syntax Handling**: Array access syntax `items[0]` is automatically converted to path segments:
   ```rust
   // `items[0] = "first"` becomes:
   vec![PathSegment::Ident("items"), PathSegment::ArrayIndex(0)]
   ```

3. **Section Management**: Nested sections are handled with a document stack:
   ```rust
   // @ database.postgres { ... } creates a nested document
   // that gets merged into the parent at the appropriate path
   ```

4. **Variant Transformation**: Values with `$variant` fields are automatically transformed:
   ```rust
   // { $variant: "Point", x: 1, y: 2 } becomes:
   // { Point: { x: 1, y: 2 } }
   ```

### Implementation Details

The new `ValueVisitor` is significantly simpler than previous implementations:

- **Single Struct Design**: Everything is contained in `ValueVisitor` - no separate `Values` struct
- **Direct Document Building**: Uses `EureDocument::insert_node()` for all insertions
- **Minimal State**: Only caches values when needed for references
- **No Post-Processing**: Variants and extensions are handled during traversal

### Comparison with Direct CstVisitor Implementation

While you could implement `CstVisitor` directly for value extraction, `ValueVisitor` provides:

- Pre-built path segment construction from keys
- Automatic handling of array syntax
- Section and binding context management
- Value type conversion (strings, numbers, arrays, etc.)
- Extension and variant handling

This makes `ValueVisitor` the recommended approach for extracting structured data from Eure documents.

### Advanced Use Case: Custom Facade and Dependency Graphs

The `CstVisitor` trait is generic over `F: CstFacade`. While `Cst` is typically used, you can implement `CstFacade` on your own type for advanced scenarios, like building a dependency graph during traversal.

Since visitor methods receive `&F` (immutable) but the visitor is `&mut self`, direct mutation of facade state isn't possible. However, using interior mutability (e.g., `RefCell`) within the custom facade allows indirect mutation.

## Modifying the CST with the Action Module

The `action` module in `eure-tree` provides a command pattern for modifying the Concrete Syntax Tree. This allows you to collect a series of modifications and apply them all at once, which can be useful for complex transformations or when you need to track and potentially revert changes.

### Key Components

* **`NodeTarget`:** An enum that can target either a CST node (`CstNodeId`) or a command node (`CommandNodeId`).
* **`CommandNodeId`:** A struct representing a node ID created by a command but not yet applied to the CST.
* **`CstCommands`:** A struct that collects commands to be applied to a CST.
* **`Command`:** An enum representing different operations that can be performed on the CST.

### How to Use

1. **Create a `CstCommands` instance:** This will collect the commands you want to apply.
2. **Add commands:** Use methods like `delete_node`, `insert_node`, and `update_node` to build up a set of changes.
3. **Apply the commands:** Call `apply_to` with a mutable reference to your CST to apply all the collected commands at once.

```rust, ignore
use eure_tree::prelude::*;
use eure_tree::action::{CstCommands, Command};
use eure_tree::{Cst, CstNode};

fn modify_cst(cst: &mut Cst) {
    // Create a new CstCommands instance
    let mut commands = CstCommands::new();

    // Get a node ID to modify (e.g., from a visitor)
    let node_id = /* ... */;

    // Delete a node
    commands.delete_node(node_id);

    // Insert a new node under a parent
    let parent_id = /* ... */;
    let new_node_data = /* ... */;
    let new_node_id = commands.insert_node(parent_id, new_node_data);

    // Update a node's data
    let node_to_update = /* ... */;
    let updated_data = /* ... */;
    commands.update_node(node_to_update, updated_data);

    // Apply all commands to the CST
    commands.apply_to(cst);
}
```

### Available Commands

* **`delete_node(id)`:** Delete a single node.
* **`delete_recursive(id)`:** Delete a node and all its descendants.
* **`insert_node(parent, data)`:** Insert a new node under a parent, returning a `CommandNodeId` that can be used in subsequent commands.
* **`update_node(id, data)`:** Update a node's data.

The command pattern allows for complex transformations to be built up and applied atomically, making it easier to reason about and potentially revert changes to the CST.
