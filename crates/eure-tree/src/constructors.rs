use crate::builder::{CstBuilder, BuilderNodeId};
use crate::node_kind::{NonTerminalKind, TerminalKind};
///Branded type for Integer terminal
#[derive(Debug, Clone)]
pub struct IntegerToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl IntegerToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<IntegerToken> for BuilderNodeId {
    fn from(token: IntegerToken) -> Self {
        token.node_id
    }
}
///Branded type for True terminal
#[derive(Debug, Clone)]
pub struct TrueToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TrueToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TrueToken> for BuilderNodeId {
    fn from(token: TrueToken) -> Self {
        token.node_id
    }
}
///Branded type for False terminal
#[derive(Debug, Clone)]
pub struct FalseToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl FalseToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<FalseToken> for BuilderNodeId {
    fn from(token: FalseToken) -> Self {
        token.node_id
    }
}
///Branded type for Null terminal
#[derive(Debug, Clone)]
pub struct NullToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl NullToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<NullToken> for BuilderNodeId {
    fn from(token: NullToken) -> Self {
        token.node_id
    }
}
///Branded type for Hole terminal
#[derive(Debug, Clone)]
pub struct HoleToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl HoleToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<HoleToken> for BuilderNodeId {
    fn from(token: HoleToken) -> Self {
        token.node_id
    }
}
///Branded type for Str terminal
#[derive(Debug, Clone)]
pub struct StrToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl StrToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<StrToken> for BuilderNodeId {
    fn from(token: StrToken) -> Self {
        token.node_id
    }
}
///Branded type for Text terminal
#[derive(Debug, Clone)]
pub struct TextToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextToken> for BuilderNodeId {
    fn from(token: TextToken) -> Self {
        token.node_id
    }
}
///Branded type for CodeBlock terminal
#[derive(Debug, Clone)]
pub struct CodeBlockToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl CodeBlockToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<CodeBlockToken> for BuilderNodeId {
    fn from(token: CodeBlockToken) -> Self {
        token.node_id
    }
}
///Branded type for NamedCode terminal
#[derive(Debug, Clone)]
pub struct NamedCodeToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl NamedCodeToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<NamedCodeToken> for BuilderNodeId {
    fn from(token: NamedCodeToken) -> Self {
        token.node_id
    }
}
///Branded type for Code terminal
#[derive(Debug, Clone)]
pub struct CodeToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl CodeToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<CodeToken> for BuilderNodeId {
    fn from(token: CodeToken) -> Self {
        token.node_id
    }
}
///Branded type for GrammarNewline terminal
#[derive(Debug, Clone)]
pub struct GrammarNewlineToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl GrammarNewlineToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<GrammarNewlineToken> for BuilderNodeId {
    fn from(token: GrammarNewlineToken) -> Self {
        token.node_id
    }
}
///Branded type for Ws terminal
#[derive(Debug, Clone)]
pub struct WsToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl WsToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<WsToken> for BuilderNodeId {
    fn from(token: WsToken) -> Self {
        token.node_id
    }
}
///Branded type for At terminal
#[derive(Debug, Clone)]
pub struct AtToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl AtToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<AtToken> for BuilderNodeId {
    fn from(token: AtToken) -> Self {
        token.node_id
    }
}
///Branded type for DollarDollar terminal
#[derive(Debug, Clone)]
pub struct DollarDollarToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl DollarDollarToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<DollarDollarToken> for BuilderNodeId {
    fn from(token: DollarDollarToken) -> Self {
        token.node_id
    }
}
///Branded type for Dollar terminal
#[derive(Debug, Clone)]
pub struct DollarToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl DollarToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<DollarToken> for BuilderNodeId {
    fn from(token: DollarToken) -> Self {
        token.node_id
    }
}
///Branded type for Dot terminal
#[derive(Debug, Clone)]
pub struct DotToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl DotToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<DotToken> for BuilderNodeId {
    fn from(token: DotToken) -> Self {
        token.node_id
    }
}
///Branded type for LBrace terminal
#[derive(Debug, Clone)]
pub struct LBraceToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl LBraceToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<LBraceToken> for BuilderNodeId {
    fn from(token: LBraceToken) -> Self {
        token.node_id
    }
}
///Branded type for RBrace terminal
#[derive(Debug, Clone)]
pub struct RBraceToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl RBraceToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<RBraceToken> for BuilderNodeId {
    fn from(token: RBraceToken) -> Self {
        token.node_id
    }
}
///Branded type for LBracket terminal
#[derive(Debug, Clone)]
pub struct LBracketToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl LBracketToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<LBracketToken> for BuilderNodeId {
    fn from(token: LBracketToken) -> Self {
        token.node_id
    }
}
///Branded type for RBracket terminal
#[derive(Debug, Clone)]
pub struct RBracketToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl RBracketToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<RBracketToken> for BuilderNodeId {
    fn from(token: RBracketToken) -> Self {
        token.node_id
    }
}
///Branded type for LParen terminal
#[derive(Debug, Clone)]
pub struct LParenToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl LParenToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<LParenToken> for BuilderNodeId {
    fn from(token: LParenToken) -> Self {
        token.node_id
    }
}
///Branded type for RParen terminal
#[derive(Debug, Clone)]
pub struct RParenToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl RParenToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<RParenToken> for BuilderNodeId {
    fn from(token: RParenToken) -> Self {
        token.node_id
    }
}
///Branded type for Bind terminal
#[derive(Debug, Clone)]
pub struct BindToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl BindToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<BindToken> for BuilderNodeId {
    fn from(token: BindToken) -> Self {
        token.node_id
    }
}
///Branded type for Comma terminal
#[derive(Debug, Clone)]
pub struct CommaToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl CommaToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<CommaToken> for BuilderNodeId {
    fn from(token: CommaToken) -> Self {
        token.node_id
    }
}
///Branded type for Esc terminal
#[derive(Debug, Clone)]
pub struct EscToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl EscToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<EscToken> for BuilderNodeId {
    fn from(token: EscToken) -> Self {
        token.node_id
    }
}
///Branded type for TextStart terminal
#[derive(Debug, Clone)]
pub struct TextStartToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextStartToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextStartToken> for BuilderNodeId {
    fn from(token: TextStartToken) -> Self {
        token.node_id
    }
}
///Branded type for Ident terminal
#[derive(Debug, Clone)]
pub struct IdentToken {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl IdentToken {
    /// Consume this token and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<IdentToken> for BuilderNodeId {
    fn from(token: IdentToken) -> Self {
        token.node_id
    }
}
///Branded type for Array non-terminal
#[derive(Debug, Clone)]
pub struct ArrayNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayNode> for BuilderNodeId {
    fn from(node: ArrayNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayBegin non-terminal
#[derive(Debug, Clone)]
pub struct ArrayBeginNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayBeginNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayBeginNode> for BuilderNodeId {
    fn from(node: ArrayBeginNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayElements non-terminal
#[derive(Debug, Clone)]
pub struct ArrayElementsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayElementsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayElementsNode> for BuilderNodeId {
    fn from(node: ArrayElementsNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayElementsOpt non-terminal
#[derive(Debug, Clone)]
pub struct ArrayElementsOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayElementsOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayElementsOptNode> for BuilderNodeId {
    fn from(node: ArrayElementsOptNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayElementsTail non-terminal
#[derive(Debug, Clone)]
pub struct ArrayElementsTailNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayElementsTailNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayElementsTailNode> for BuilderNodeId {
    fn from(node: ArrayElementsTailNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayElementsTailOpt non-terminal
#[derive(Debug, Clone)]
pub struct ArrayElementsTailOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayElementsTailOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayElementsTailOptNode> for BuilderNodeId {
    fn from(node: ArrayElementsTailOptNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayEnd non-terminal
#[derive(Debug, Clone)]
pub struct ArrayEndNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayEndNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayEndNode> for BuilderNodeId {
    fn from(node: ArrayEndNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayMarker non-terminal
#[derive(Debug, Clone)]
pub struct ArrayMarkerNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayMarkerNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayMarkerNode> for BuilderNodeId {
    fn from(node: ArrayMarkerNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayMarkerOpt non-terminal
#[derive(Debug, Clone)]
pub struct ArrayMarkerOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayMarkerOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayMarkerOptNode> for BuilderNodeId {
    fn from(node: ArrayMarkerOptNode) -> Self {
        node.node_id
    }
}
///Branded type for ArrayOpt non-terminal
#[derive(Debug, Clone)]
pub struct ArrayOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ArrayOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ArrayOptNode> for BuilderNodeId {
    fn from(node: ArrayOptNode) -> Self {
        node.node_id
    }
}
///Branded type for At non-terminal
#[derive(Debug, Clone)]
pub struct AtNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl AtNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<AtNode> for BuilderNodeId {
    fn from(node: AtNode) -> Self {
        node.node_id
    }
}
///Branded type for Begin non-terminal
#[derive(Debug, Clone)]
pub struct BeginNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl BeginNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<BeginNode> for BuilderNodeId {
    fn from(node: BeginNode) -> Self {
        node.node_id
    }
}
///Branded type for Bind non-terminal
#[derive(Debug, Clone)]
pub struct BindNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl BindNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<BindNode> for BuilderNodeId {
    fn from(node: BindNode) -> Self {
        node.node_id
    }
}
///Branded type for Binding non-terminal
#[derive(Debug, Clone)]
pub struct BindingNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl BindingNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<BindingNode> for BuilderNodeId {
    fn from(node: BindingNode) -> Self {
        node.node_id
    }
}
///Branded type for BindingRhs non-terminal
#[derive(Debug, Clone)]
pub struct BindingRhsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl BindingRhsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<BindingRhsNode> for BuilderNodeId {
    fn from(node: BindingRhsNode) -> Self {
        node.node_id
    }
}
///Branded type for Boolean non-terminal
#[derive(Debug, Clone)]
pub struct BooleanNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl BooleanNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<BooleanNode> for BuilderNodeId {
    fn from(node: BooleanNode) -> Self {
        node.node_id
    }
}
///Branded type for Code non-terminal
#[derive(Debug, Clone)]
pub struct CodeNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl CodeNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<CodeNode> for BuilderNodeId {
    fn from(node: CodeNode) -> Self {
        node.node_id
    }
}
///Branded type for CodeBlock non-terminal
#[derive(Debug, Clone)]
pub struct CodeBlockNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl CodeBlockNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<CodeBlockNode> for BuilderNodeId {
    fn from(node: CodeBlockNode) -> Self {
        node.node_id
    }
}
///Branded type for Comma non-terminal
#[derive(Debug, Clone)]
pub struct CommaNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl CommaNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<CommaNode> for BuilderNodeId {
    fn from(node: CommaNode) -> Self {
        node.node_id
    }
}
///Branded type for Continue non-terminal
#[derive(Debug, Clone)]
pub struct ContinueNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ContinueNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ContinueNode> for BuilderNodeId {
    fn from(node: ContinueNode) -> Self {
        node.node_id
    }
}
///Branded type for Dot non-terminal
#[derive(Debug, Clone)]
pub struct DotNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl DotNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<DotNode> for BuilderNodeId {
    fn from(node: DotNode) -> Self {
        node.node_id
    }
}
///Branded type for End non-terminal
#[derive(Debug, Clone)]
pub struct EndNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl EndNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<EndNode> for BuilderNodeId {
    fn from(node: EndNode) -> Self {
        node.node_id
    }
}
///Branded type for Eure non-terminal
#[derive(Debug, Clone)]
pub struct EureNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl EureNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<EureNode> for BuilderNodeId {
    fn from(node: EureNode) -> Self {
        node.node_id
    }
}
///Branded type for EureBindings non-terminal
#[derive(Debug, Clone)]
pub struct EureBindingsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl EureBindingsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<EureBindingsNode> for BuilderNodeId {
    fn from(node: EureBindingsNode) -> Self {
        node.node_id
    }
}
///Branded type for EureSections non-terminal
#[derive(Debug, Clone)]
pub struct EureSectionsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl EureSectionsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<EureSectionsNode> for BuilderNodeId {
    fn from(node: EureSectionsNode) -> Self {
        node.node_id
    }
}
///Branded type for Ext non-terminal
#[derive(Debug, Clone)]
pub struct ExtNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ExtNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ExtNode> for BuilderNodeId {
    fn from(node: ExtNode) -> Self {
        node.node_id
    }
}
///Branded type for ExtensionNameSpace non-terminal
#[derive(Debug, Clone)]
pub struct ExtensionNameSpaceNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ExtensionNameSpaceNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ExtensionNameSpaceNode> for BuilderNodeId {
    fn from(node: ExtensionNameSpaceNode) -> Self {
        node.node_id
    }
}
///Branded type for False non-terminal
#[derive(Debug, Clone)]
pub struct FalseNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl FalseNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<FalseNode> for BuilderNodeId {
    fn from(node: FalseNode) -> Self {
        node.node_id
    }
}
///Branded type for GrammarNewline non-terminal
#[derive(Debug, Clone)]
pub struct GrammarNewlineNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl GrammarNewlineNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<GrammarNewlineNode> for BuilderNodeId {
    fn from(node: GrammarNewlineNode) -> Self {
        node.node_id
    }
}
///Branded type for Hole non-terminal
#[derive(Debug, Clone)]
pub struct HoleNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl HoleNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<HoleNode> for BuilderNodeId {
    fn from(node: HoleNode) -> Self {
        node.node_id
    }
}
///Branded type for Ident non-terminal
#[derive(Debug, Clone)]
pub struct IdentNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl IdentNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<IdentNode> for BuilderNodeId {
    fn from(node: IdentNode) -> Self {
        node.node_id
    }
}
///Branded type for Integer non-terminal
#[derive(Debug, Clone)]
pub struct IntegerNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl IntegerNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<IntegerNode> for BuilderNodeId {
    fn from(node: IntegerNode) -> Self {
        node.node_id
    }
}
///Branded type for Key non-terminal
#[derive(Debug, Clone)]
pub struct KeyNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl KeyNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<KeyNode> for BuilderNodeId {
    fn from(node: KeyNode) -> Self {
        node.node_id
    }
}
///Branded type for KeyBase non-terminal
#[derive(Debug, Clone)]
pub struct KeyBaseNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl KeyBaseNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<KeyBaseNode> for BuilderNodeId {
    fn from(node: KeyBaseNode) -> Self {
        node.node_id
    }
}
///Branded type for KeyOpt non-terminal
#[derive(Debug, Clone)]
pub struct KeyOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl KeyOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<KeyOptNode> for BuilderNodeId {
    fn from(node: KeyOptNode) -> Self {
        node.node_id
    }
}
///Branded type for Keys non-terminal
#[derive(Debug, Clone)]
pub struct KeysNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl KeysNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<KeysNode> for BuilderNodeId {
    fn from(node: KeysNode) -> Self {
        node.node_id
    }
}
///Branded type for KeysList non-terminal
#[derive(Debug, Clone)]
pub struct KeysListNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl KeysListNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<KeysListNode> for BuilderNodeId {
    fn from(node: KeysListNode) -> Self {
        node.node_id
    }
}
///Branded type for LParen non-terminal
#[derive(Debug, Clone)]
pub struct LParenNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl LParenNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<LParenNode> for BuilderNodeId {
    fn from(node: LParenNode) -> Self {
        node.node_id
    }
}
///Branded type for MetaExt non-terminal
#[derive(Debug, Clone)]
pub struct MetaExtNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl MetaExtNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<MetaExtNode> for BuilderNodeId {
    fn from(node: MetaExtNode) -> Self {
        node.node_id
    }
}
///Branded type for MetaExtKey non-terminal
#[derive(Debug, Clone)]
pub struct MetaExtKeyNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl MetaExtKeyNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<MetaExtKeyNode> for BuilderNodeId {
    fn from(node: MetaExtKeyNode) -> Self {
        node.node_id
    }
}
///Branded type for NamedCode non-terminal
#[derive(Debug, Clone)]
pub struct NamedCodeNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl NamedCodeNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<NamedCodeNode> for BuilderNodeId {
    fn from(node: NamedCodeNode) -> Self {
        node.node_id
    }
}
///Branded type for Null non-terminal
#[derive(Debug, Clone)]
pub struct NullNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl NullNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<NullNode> for BuilderNodeId {
    fn from(node: NullNode) -> Self {
        node.node_id
    }
}
///Branded type for Object non-terminal
#[derive(Debug, Clone)]
pub struct ObjectNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ObjectNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ObjectNode> for BuilderNodeId {
    fn from(node: ObjectNode) -> Self {
        node.node_id
    }
}
///Branded type for ObjectList non-terminal
#[derive(Debug, Clone)]
pub struct ObjectListNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ObjectListNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ObjectListNode> for BuilderNodeId {
    fn from(node: ObjectListNode) -> Self {
        node.node_id
    }
}
///Branded type for ObjectOpt non-terminal
#[derive(Debug, Clone)]
pub struct ObjectOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ObjectOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ObjectOptNode> for BuilderNodeId {
    fn from(node: ObjectOptNode) -> Self {
        node.node_id
    }
}
///Branded type for Path non-terminal
#[derive(Debug, Clone)]
pub struct PathNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl PathNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<PathNode> for BuilderNodeId {
    fn from(node: PathNode) -> Self {
        node.node_id
    }
}
///Branded type for RParen non-terminal
#[derive(Debug, Clone)]
pub struct RParenNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl RParenNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<RParenNode> for BuilderNodeId {
    fn from(node: RParenNode) -> Self {
        node.node_id
    }
}
///Branded type for Section non-terminal
#[derive(Debug, Clone)]
pub struct SectionNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl SectionNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<SectionNode> for BuilderNodeId {
    fn from(node: SectionNode) -> Self {
        node.node_id
    }
}
///Branded type for SectionBinding non-terminal
#[derive(Debug, Clone)]
pub struct SectionBindingNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl SectionBindingNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<SectionBindingNode> for BuilderNodeId {
    fn from(node: SectionBindingNode) -> Self {
        node.node_id
    }
}
///Branded type for SectionBody non-terminal
#[derive(Debug, Clone)]
pub struct SectionBodyNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl SectionBodyNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<SectionBodyNode> for BuilderNodeId {
    fn from(node: SectionBodyNode) -> Self {
        node.node_id
    }
}
///Branded type for SectionBodyList non-terminal
#[derive(Debug, Clone)]
pub struct SectionBodyListNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl SectionBodyListNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<SectionBodyListNode> for BuilderNodeId {
    fn from(node: SectionBodyListNode) -> Self {
        node.node_id
    }
}
///Branded type for Str non-terminal
#[derive(Debug, Clone)]
pub struct StrNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl StrNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<StrNode> for BuilderNodeId {
    fn from(node: StrNode) -> Self {
        node.node_id
    }
}
///Branded type for Strings non-terminal
#[derive(Debug, Clone)]
pub struct StringsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl StringsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<StringsNode> for BuilderNodeId {
    fn from(node: StringsNode) -> Self {
        node.node_id
    }
}
///Branded type for StringsList non-terminal
#[derive(Debug, Clone)]
pub struct StringsListNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl StringsListNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<StringsListNode> for BuilderNodeId {
    fn from(node: StringsListNode) -> Self {
        node.node_id
    }
}
///Branded type for Text non-terminal
#[derive(Debug, Clone)]
pub struct TextNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextNode> for BuilderNodeId {
    fn from(node: TextNode) -> Self {
        node.node_id
    }
}
///Branded type for TextBinding non-terminal
#[derive(Debug, Clone)]
pub struct TextBindingNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextBindingNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextBindingNode> for BuilderNodeId {
    fn from(node: TextBindingNode) -> Self {
        node.node_id
    }
}
///Branded type for TextBindingOpt non-terminal
#[derive(Debug, Clone)]
pub struct TextBindingOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextBindingOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextBindingOptNode> for BuilderNodeId {
    fn from(node: TextBindingOptNode) -> Self {
        node.node_id
    }
}
///Branded type for TextBindingOpt0 non-terminal
#[derive(Debug, Clone)]
pub struct TextBindingOpt0Node {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextBindingOpt0Node {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextBindingOpt0Node> for BuilderNodeId {
    fn from(node: TextBindingOpt0Node) -> Self {
        node.node_id
    }
}
///Branded type for TextStart non-terminal
#[derive(Debug, Clone)]
pub struct TextStartNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TextStartNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TextStartNode> for BuilderNodeId {
    fn from(node: TextStartNode) -> Self {
        node.node_id
    }
}
///Branded type for True non-terminal
#[derive(Debug, Clone)]
pub struct TrueNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TrueNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TrueNode> for BuilderNodeId {
    fn from(node: TrueNode) -> Self {
        node.node_id
    }
}
///Branded type for Tuple non-terminal
#[derive(Debug, Clone)]
pub struct TupleNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TupleNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TupleNode> for BuilderNodeId {
    fn from(node: TupleNode) -> Self {
        node.node_id
    }
}
///Branded type for TupleElements non-terminal
#[derive(Debug, Clone)]
pub struct TupleElementsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TupleElementsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TupleElementsNode> for BuilderNodeId {
    fn from(node: TupleElementsNode) -> Self {
        node.node_id
    }
}
///Branded type for TupleElementsOpt non-terminal
#[derive(Debug, Clone)]
pub struct TupleElementsOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TupleElementsOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TupleElementsOptNode> for BuilderNodeId {
    fn from(node: TupleElementsOptNode) -> Self {
        node.node_id
    }
}
///Branded type for TupleElementsTail non-terminal
#[derive(Debug, Clone)]
pub struct TupleElementsTailNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TupleElementsTailNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TupleElementsTailNode> for BuilderNodeId {
    fn from(node: TupleElementsTailNode) -> Self {
        node.node_id
    }
}
///Branded type for TupleElementsTailOpt non-terminal
#[derive(Debug, Clone)]
pub struct TupleElementsTailOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TupleElementsTailOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TupleElementsTailOptNode> for BuilderNodeId {
    fn from(node: TupleElementsTailOptNode) -> Self {
        node.node_id
    }
}
///Branded type for TupleOpt non-terminal
#[derive(Debug, Clone)]
pub struct TupleOptNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl TupleOptNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<TupleOptNode> for BuilderNodeId {
    fn from(node: TupleOptNode) -> Self {
        node.node_id
    }
}
///Branded type for Value non-terminal
#[derive(Debug, Clone)]
pub struct ValueNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ValueNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ValueNode> for BuilderNodeId {
    fn from(node: ValueNode) -> Self {
        node.node_id
    }
}
///Branded type for ValueBinding non-terminal
#[derive(Debug, Clone)]
pub struct ValueBindingNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl ValueBindingNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<ValueBindingNode> for BuilderNodeId {
    fn from(node: ValueBindingNode) -> Self {
        node.node_id
    }
}
///Branded type for Ws non-terminal
#[derive(Debug, Clone)]
pub struct WsNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl WsNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<WsNode> for BuilderNodeId {
    fn from(node: WsNode) -> Self {
        node.node_id
    }
}
///Branded type for Root non-terminal
#[derive(Debug, Clone)]
pub struct RootNode {
    pub(super) node_id: BuilderNodeId,
    pub(super) builder: CstBuilder,
}
impl RootNode {
    /// Consume this node and return its builder
    pub fn into_builder(self) -> CstBuilder {
        self.builder
    }
}
impl From<RootNode> for BuilderNodeId {
    fn from(node: RootNode) -> Self {
        node.node_id
    }
}
#[derive(bon::Builder)]
pub struct ArrayConstructor {
    array_begin: ArrayBeginNode,
    array_opt: ArrayOptNode,
    array_end: ArrayEndNode,
}
impl ArrayConstructor {
    pub fn build(self) -> ArrayNode {
        let mut builder = CstBuilder::new();
        let array_begin = builder.embed(self.array_begin.builder);
        let array_opt = builder.embed(self.array_opt.builder);
        let array_end = builder.embed(self.array_end.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::Array,
                vec![array_begin, array_opt, array_end],
            );
        ArrayNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ArrayBeginConstructor {
    l_bracket: LBracketToken,
}
impl ArrayBeginConstructor {
    pub fn build(self) -> ArrayBeginNode {
        let mut builder = CstBuilder::new();
        let l_bracket = builder.embed(self.l_bracket.builder);
        let node_id = builder.non_terminal(NonTerminalKind::ArrayBegin, vec![l_bracket]);
        ArrayBeginNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ArrayElementsConstructor {
    value: ValueNode,
    array_elements_opt: ArrayElementsOptNode,
}
impl ArrayElementsConstructor {
    pub fn build(self) -> ArrayElementsNode {
        let mut builder = CstBuilder::new();
        let value = builder.embed(self.value.builder);
        let array_elements_opt = builder.embed(self.array_elements_opt.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::ArrayElements,
                vec![value, array_elements_opt],
            );
        ArrayElementsNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ArrayElementsOptConstructor {
    array_elements_tail: Option<ArrayElementsTailNode>,
}
impl ArrayElementsOptConstructor {
    pub fn build(self) -> ArrayElementsOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.array_elements_tail {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::ArrayElementsOpt, children);
        ArrayElementsOptNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ArrayElementsTailConstructor {
    comma: CommaNode,
    array_elements_tail_opt: ArrayElementsTailOptNode,
}
impl ArrayElementsTailConstructor {
    pub fn build(self) -> ArrayElementsTailNode {
        let mut builder = CstBuilder::new();
        let comma = builder.embed(self.comma.builder);
        let array_elements_tail_opt = builder
            .embed(self.array_elements_tail_opt.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::ArrayElementsTail,
                vec![comma, array_elements_tail_opt],
            );
        ArrayElementsTailNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ArrayElementsTailOptConstructor {
    array_elements: Option<ArrayElementsNode>,
}
impl ArrayElementsTailOptConstructor {
    pub fn build(self) -> ArrayElementsTailOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.array_elements {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder
            .non_terminal(NonTerminalKind::ArrayElementsTailOpt, children);
        ArrayElementsTailOptNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ArrayEndConstructor {
    r_bracket: RBracketToken,
}
impl ArrayEndConstructor {
    pub fn build(self) -> ArrayEndNode {
        let mut builder = CstBuilder::new();
        let r_bracket = builder.embed(self.r_bracket.builder);
        let node_id = builder.non_terminal(NonTerminalKind::ArrayEnd, vec![r_bracket]);
        ArrayEndNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ArrayMarkerConstructor {
    array_begin: ArrayBeginNode,
    array_marker_opt: ArrayMarkerOptNode,
    array_end: ArrayEndNode,
}
impl ArrayMarkerConstructor {
    pub fn build(self) -> ArrayMarkerNode {
        let mut builder = CstBuilder::new();
        let array_begin = builder.embed(self.array_begin.builder);
        let array_marker_opt = builder.embed(self.array_marker_opt.builder);
        let array_end = builder.embed(self.array_end.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::ArrayMarker,
                vec![array_begin, array_marker_opt, array_end],
            );
        ArrayMarkerNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ArrayMarkerOptConstructor {
    integer: Option<IntegerNode>,
}
impl ArrayMarkerOptConstructor {
    pub fn build(self) -> ArrayMarkerOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.integer {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::ArrayMarkerOpt, children);
        ArrayMarkerOptNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ArrayOptConstructor {
    array_elements: Option<ArrayElementsNode>,
}
impl ArrayOptConstructor {
    pub fn build(self) -> ArrayOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.array_elements {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::ArrayOpt, children);
        ArrayOptNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct AtConstructor {
    at: AtToken,
}
impl AtConstructor {
    pub fn build(self) -> AtNode {
        let mut builder = CstBuilder::new();
        let at = builder.embed(self.at.builder);
        let node_id = builder.non_terminal(NonTerminalKind::At, vec![at]);
        AtNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct BeginConstructor {
    l_brace: LBraceToken,
}
impl BeginConstructor {
    pub fn build(self) -> BeginNode {
        let mut builder = CstBuilder::new();
        let l_brace = builder.embed(self.l_brace.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Begin, vec![l_brace]);
        BeginNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct BindConstructor {
    bind: BindToken,
}
impl BindConstructor {
    pub fn build(self) -> BindNode {
        let mut builder = CstBuilder::new();
        let bind = builder.embed(self.bind.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Bind, vec![bind]);
        BindNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct BindingConstructor {
    keys: KeysNode,
    binding_rhs: BindingRhsNode,
}
impl BindingConstructor {
    pub fn build(self) -> BindingNode {
        let mut builder = CstBuilder::new();
        let keys = builder.embed(self.keys.builder);
        let binding_rhs = builder.embed(self.binding_rhs.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Binding, vec![keys, binding_rhs]);
        BindingNode { node_id, builder }
    }
}
pub enum BindingRhsConstructor {
    ValueBinding(ValueBindingNode),
    SectionBinding(SectionBindingNode),
    TextBinding(TextBindingNode),
}
impl BindingRhsConstructor {
    pub fn build(self) -> BindingRhsNode {
        let mut builder = CstBuilder::new();
        let child_id = match self {
            Self::ValueBinding(node) => builder.embed(node.builder),
            Self::SectionBinding(node) => builder.embed(node.builder),
            Self::TextBinding(node) => builder.embed(node.builder),
        };
        let node_id = builder.non_terminal(NonTerminalKind::BindingRhs, vec![child_id]);
        BindingRhsNode { node_id, builder }
    }
}
pub enum BooleanConstructor {
    True(TrueNode),
    False(FalseNode),
}
impl BooleanConstructor {
    pub fn build(self) -> BooleanNode {
        let mut builder = CstBuilder::new();
        let child_id = match self {
            Self::True(node) => builder.embed(node.builder),
            Self::False(node) => builder.embed(node.builder),
        };
        let node_id = builder.non_terminal(NonTerminalKind::Boolean, vec![child_id]);
        BooleanNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct CodeConstructor {
    code: CodeToken,
}
impl CodeConstructor {
    pub fn build(self) -> CodeNode {
        let mut builder = CstBuilder::new();
        let code = builder.embed(self.code.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Code, vec![code]);
        CodeNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct CodeBlockConstructor {
    code_block: CodeBlockToken,
}
impl CodeBlockConstructor {
    pub fn build(self) -> CodeBlockNode {
        let mut builder = CstBuilder::new();
        let code_block = builder.embed(self.code_block.builder);
        let node_id = builder.non_terminal(NonTerminalKind::CodeBlock, vec![code_block]);
        CodeBlockNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct CommaConstructor {
    comma: CommaToken,
}
impl CommaConstructor {
    pub fn build(self) -> CommaNode {
        let mut builder = CstBuilder::new();
        let comma = builder.embed(self.comma.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Comma, vec![comma]);
        CommaNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ContinueConstructor {
    esc: EscToken,
}
impl ContinueConstructor {
    pub fn build(self) -> ContinueNode {
        let mut builder = CstBuilder::new();
        let esc = builder.embed(self.esc.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Continue, vec![esc]);
        ContinueNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct DotConstructor {
    dot: DotToken,
}
impl DotConstructor {
    pub fn build(self) -> DotNode {
        let mut builder = CstBuilder::new();
        let dot = builder.embed(self.dot.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Dot, vec![dot]);
        DotNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct EndConstructor {
    r_brace: RBraceToken,
}
impl EndConstructor {
    pub fn build(self) -> EndNode {
        let mut builder = CstBuilder::new();
        let r_brace = builder.embed(self.r_brace.builder);
        let node_id = builder.non_terminal(NonTerminalKind::End, vec![r_brace]);
        EndNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct EureConstructor {
    eure_bindings: EureBindingsNode,
    eure_sections: EureSectionsNode,
}
impl EureConstructor {
    pub fn build(self) -> EureNode {
        let mut builder = CstBuilder::new();
        let eure_bindings = builder.embed(self.eure_bindings.builder);
        let eure_sections = builder.embed(self.eure_sections.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Eure, vec![eure_bindings, eure_sections]);
        EureNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct EureBindingsConstructor {
    binding: BindingNode,
    eure_bindings: EureBindingsNode,
}
impl EureBindingsConstructor {
    /// Create an empty node (base case for recursion)
    pub fn empty() -> EureBindingsNode {
        let mut builder = CstBuilder::new();
        let node_id = builder
            .non_terminal(NonTerminalKind::EureList, Vec::<BuilderNodeId>::new());
        EureBindingsNode {
            node_id,
            builder,
        }
    }
    /// Create a node with children (recursive case)
    pub fn build(self) -> EureBindingsNode {
        let mut builder = CstBuilder::new();
        let binding = builder.embed(self.binding.builder);
        let eure_bindings = builder.embed(self.eure_bindings.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::EureList, vec![binding, eure_bindings]);
        EureBindingsNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct EureSectionsConstructor {
    section: SectionNode,
    eure_sections: EureSectionsNode,
}
impl EureSectionsConstructor {
    /// Create an empty node (base case for recursion)
    pub fn empty() -> EureSectionsNode {
        let mut builder = CstBuilder::new();
        let node_id = builder
            .non_terminal(NonTerminalKind::EureList0, Vec::<BuilderNodeId>::new());
        EureSectionsNode {
            node_id,
            builder,
        }
    }
    /// Create a node with children (recursive case)
    pub fn build(self) -> EureSectionsNode {
        let mut builder = CstBuilder::new();
        let section = builder.embed(self.section.builder);
        let eure_sections = builder.embed(self.eure_sections.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::EureList0, vec![section, eure_sections]);
        EureSectionsNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct ExtConstructor {
    dollar: DollarToken,
}
impl ExtConstructor {
    pub fn build(self) -> ExtNode {
        let mut builder = CstBuilder::new();
        let dollar = builder.embed(self.dollar.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Ext, vec![dollar]);
        ExtNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ExtensionNameSpaceConstructor {
    ext: ExtNode,
    ident: IdentNode,
}
impl ExtensionNameSpaceConstructor {
    pub fn build(self) -> ExtensionNameSpaceNode {
        let mut builder = CstBuilder::new();
        let ext = builder.embed(self.ext.builder);
        let ident = builder.embed(self.ident.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::ExtensionNameSpace, vec![ext, ident]);
        ExtensionNameSpaceNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct FalseConstructor {
    r#false: FalseToken,
}
impl FalseConstructor {
    pub fn build(self) -> FalseNode {
        let mut builder = CstBuilder::new();
        let r#false = builder.embed(self.r#false.builder);
        let node_id = builder.non_terminal(NonTerminalKind::False, vec![r#false]);
        FalseNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct GrammarNewlineConstructor {
    grammar_newline: GrammarNewlineToken,
}
impl GrammarNewlineConstructor {
    pub fn build(self) -> GrammarNewlineNode {
        let mut builder = CstBuilder::new();
        let grammar_newline = builder.embed(self.grammar_newline.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::GrammarNewline, vec![grammar_newline]);
        GrammarNewlineNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct HoleConstructor {
    hole: HoleToken,
}
impl HoleConstructor {
    pub fn build(self) -> HoleNode {
        let mut builder = CstBuilder::new();
        let hole = builder.embed(self.hole.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Hole, vec![hole]);
        HoleNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct IdentConstructor {
    ident: IdentToken,
}
impl IdentConstructor {
    pub fn build(self) -> IdentNode {
        let mut builder = CstBuilder::new();
        let ident = builder.embed(self.ident.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Ident, vec![ident]);
        IdentNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct IntegerConstructor {
    integer: IntegerToken,
}
impl IntegerConstructor {
    pub fn build(self) -> IntegerNode {
        let mut builder = CstBuilder::new();
        let integer = builder.embed(self.integer.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Integer, vec![integer]);
        IntegerNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct KeyConstructor {
    key_base: KeyBaseNode,
    key_opt: KeyOptNode,
}
impl KeyConstructor {
    pub fn build(self) -> KeyNode {
        let mut builder = CstBuilder::new();
        let key_base = builder.embed(self.key_base.builder);
        let key_opt = builder.embed(self.key_opt.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Key, vec![key_base, key_opt]);
        KeyNode { node_id, builder }
    }
}
pub enum KeyBaseConstructor {
    Ident(IdentNode),
    ExtensionNameSpace(ExtensionNameSpaceNode),
    Str(StrNode),
    Integer(IntegerNode),
    MetaExtKey(MetaExtKeyNode),
    Null(NullNode),
    True(TrueNode),
    False(FalseNode),
}
impl KeyBaseConstructor {
    pub fn build(self) -> KeyBaseNode {
        let mut builder = CstBuilder::new();
        let child_id = match self {
            Self::Ident(node) => builder.embed(node.builder),
            Self::ExtensionNameSpace(node) => builder.embed(node.builder),
            Self::Str(node) => builder.embed(node.builder),
            Self::Integer(node) => builder.embed(node.builder),
            Self::MetaExtKey(node) => builder.embed(node.builder),
            Self::Null(node) => builder.embed(node.builder),
            Self::True(node) => builder.embed(node.builder),
            Self::False(node) => builder.embed(node.builder),
        };
        let node_id = builder.non_terminal(NonTerminalKind::KeyBase, vec![child_id]);
        KeyBaseNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct KeyOptConstructor {
    array_marker: Option<ArrayMarkerNode>,
}
impl KeyOptConstructor {
    pub fn build(self) -> KeyOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.array_marker {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::KeyOpt, children);
        KeyOptNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct KeysConstructor {
    key: KeyNode,
    keys_list: KeysListNode,
}
impl KeysConstructor {
    pub fn build(self) -> KeysNode {
        let mut builder = CstBuilder::new();
        let key = builder.embed(self.key.builder);
        let keys_list = builder.embed(self.keys_list.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Keys, vec![key, keys_list]);
        KeysNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct KeysListConstructor {
    dot: DotNode,
    key: KeyNode,
    keys_list: KeysListNode,
}
impl KeysListConstructor {
    /// Create an empty node (base case for recursion)
    pub fn empty() -> KeysListNode {
        let mut builder = CstBuilder::new();
        let node_id = builder
            .non_terminal(NonTerminalKind::KeysList, Vec::<BuilderNodeId>::new());
        KeysListNode { node_id, builder }
    }
    /// Create a node with children (recursive case)
    pub fn build(self) -> KeysListNode {
        let mut builder = CstBuilder::new();
        let dot = builder.embed(self.dot.builder);
        let key = builder.embed(self.key.builder);
        let keys_list = builder.embed(self.keys_list.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::KeysList, vec![dot, key, keys_list]);
        KeysListNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct LParenConstructor {
    l_paren: LParenToken,
}
impl LParenConstructor {
    pub fn build(self) -> LParenNode {
        let mut builder = CstBuilder::new();
        let l_paren = builder.embed(self.l_paren.builder);
        let node_id = builder.non_terminal(NonTerminalKind::LParen, vec![l_paren]);
        LParenNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct MetaExtConstructor {
    dollar_dollar: DollarDollarToken,
}
impl MetaExtConstructor {
    pub fn build(self) -> MetaExtNode {
        let mut builder = CstBuilder::new();
        let dollar_dollar = builder.embed(self.dollar_dollar.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::MetaExt, vec![dollar_dollar]);
        MetaExtNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct MetaExtKeyConstructor {
    meta_ext: MetaExtNode,
    ident: IdentNode,
}
impl MetaExtKeyConstructor {
    pub fn build(self) -> MetaExtKeyNode {
        let mut builder = CstBuilder::new();
        let meta_ext = builder.embed(self.meta_ext.builder);
        let ident = builder.embed(self.ident.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::MetaExtKey, vec![meta_ext, ident]);
        MetaExtKeyNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct NamedCodeConstructor {
    named_code: NamedCodeToken,
}
impl NamedCodeConstructor {
    pub fn build(self) -> NamedCodeNode {
        let mut builder = CstBuilder::new();
        let named_code = builder.embed(self.named_code.builder);
        let node_id = builder.non_terminal(NonTerminalKind::NamedCode, vec![named_code]);
        NamedCodeNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct NullConstructor {
    null: NullToken,
}
impl NullConstructor {
    pub fn build(self) -> NullNode {
        let mut builder = CstBuilder::new();
        let null = builder.embed(self.null.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Null, vec![null]);
        NullNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ObjectConstructor {
    begin: BeginNode,
    object_list: ObjectListNode,
    end: EndNode,
}
impl ObjectConstructor {
    pub fn build(self) -> ObjectNode {
        let mut builder = CstBuilder::new();
        let begin = builder.embed(self.begin.builder);
        let object_list = builder.embed(self.object_list.builder);
        let end = builder.embed(self.end.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Object, vec![begin, object_list, end]);
        ObjectNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ObjectListConstructor {
    key: KeyNode,
    bind: BindNode,
    value: ValueNode,
    object_opt: ObjectOptNode,
    object_list: ObjectListNode,
}
impl ObjectListConstructor {
    /// Create an empty node (base case for recursion)
    pub fn empty() -> ObjectListNode {
        let mut builder = CstBuilder::new();
        let node_id = builder
            .non_terminal(NonTerminalKind::ObjectList, Vec::<BuilderNodeId>::new());
        ObjectListNode { node_id, builder }
    }
    /// Create a node with children (recursive case)
    pub fn build(self) -> ObjectListNode {
        let mut builder = CstBuilder::new();
        let key = builder.embed(self.key.builder);
        let bind = builder.embed(self.bind.builder);
        let value = builder.embed(self.value.builder);
        let object_opt = builder.embed(self.object_opt.builder);
        let object_list = builder.embed(self.object_list.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::ObjectList,
                vec![key, bind, value, object_opt, object_list],
            );
        ObjectListNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ObjectOptConstructor {
    comma: Option<CommaNode>,
}
impl ObjectOptConstructor {
    pub fn build(self) -> ObjectOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.comma {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::ObjectOpt, children);
        ObjectOptNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct PathConstructor {
    dot: DotNode,
    keys: KeysNode,
}
impl PathConstructor {
    pub fn build(self) -> PathNode {
        let mut builder = CstBuilder::new();
        let dot = builder.embed(self.dot.builder);
        let keys = builder.embed(self.keys.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Path, vec![dot, keys]);
        PathNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct RParenConstructor {
    r_paren: RParenToken,
}
impl RParenConstructor {
    pub fn build(self) -> RParenNode {
        let mut builder = CstBuilder::new();
        let r_paren = builder.embed(self.r_paren.builder);
        let node_id = builder.non_terminal(NonTerminalKind::RParen, vec![r_paren]);
        RParenNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct SectionConstructor {
    at: AtNode,
    keys: KeysNode,
    section_body: SectionBodyNode,
}
impl SectionConstructor {
    pub fn build(self) -> SectionNode {
        let mut builder = CstBuilder::new();
        let at = builder.embed(self.at.builder);
        let keys = builder.embed(self.keys.builder);
        let section_body = builder.embed(self.section_body.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Section, vec![at, keys, section_body]);
        SectionNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct SectionBindingConstructor {
    begin: BeginNode,
    eure: EureNode,
    end: EndNode,
}
impl SectionBindingConstructor {
    pub fn build(self) -> SectionBindingNode {
        let mut builder = CstBuilder::new();
        let begin = builder.embed(self.begin.builder);
        let eure = builder.embed(self.eure.builder);
        let end = builder.embed(self.end.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::SectionBinding, vec![begin, eure, end]);
        SectionBindingNode {
            node_id,
            builder,
        }
    }
}
pub enum SectionBodyConstructor {
    SectionBodyList(SectionBodyListNode),
    SectionBinding(SectionBindingNode),
    Bind(BindNode),
}
impl SectionBodyConstructor {
    pub fn build(self) -> SectionBodyNode {
        let mut builder = CstBuilder::new();
        let child_id = match self {
            Self::SectionBodyList(node) => builder.embed(node.builder),
            Self::SectionBinding(node) => builder.embed(node.builder),
            Self::Bind(node) => builder.embed(node.builder),
        };
        let node_id = builder.non_terminal(NonTerminalKind::SectionBody, vec![child_id]);
        SectionBodyNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct SectionBodyListConstructor {
    binding: BindingNode,
    section_body_list: SectionBodyListNode,
}
impl SectionBodyListConstructor {
    /// Create an empty node (base case for recursion)
    pub fn empty() -> SectionBodyListNode {
        let mut builder = CstBuilder::new();
        let node_id = builder
            .non_terminal(NonTerminalKind::SectionBodyList, Vec::<BuilderNodeId>::new());
        SectionBodyListNode {
            node_id,
            builder,
        }
    }
    /// Create a node with children (recursive case)
    pub fn build(self) -> SectionBodyListNode {
        let mut builder = CstBuilder::new();
        let binding = builder.embed(self.binding.builder);
        let section_body_list = builder.embed(self.section_body_list.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::SectionBodyList,
                vec![binding, section_body_list],
            );
        SectionBodyListNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct StrConstructor {
    str: StrToken,
}
impl StrConstructor {
    pub fn build(self) -> StrNode {
        let mut builder = CstBuilder::new();
        let str = builder.embed(self.str.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Str, vec![str]);
        StrNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct StringsConstructor {
    str: StrNode,
    strings_list: StringsListNode,
}
impl StringsConstructor {
    pub fn build(self) -> StringsNode {
        let mut builder = CstBuilder::new();
        let str = builder.embed(self.str.builder);
        let strings_list = builder.embed(self.strings_list.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Strings, vec![str, strings_list]);
        StringsNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct StringsListConstructor {
    r#continue: ContinueNode,
    str: StrNode,
    strings_list: StringsListNode,
}
impl StringsListConstructor {
    /// Create an empty node (base case for recursion)
    pub fn empty() -> StringsListNode {
        let mut builder = CstBuilder::new();
        let node_id = builder
            .non_terminal(NonTerminalKind::StringsList, Vec::<BuilderNodeId>::new());
        StringsListNode {
            node_id,
            builder,
        }
    }
    /// Create a node with children (recursive case)
    pub fn build(self) -> StringsListNode {
        let mut builder = CstBuilder::new();
        let r#continue = builder.embed(self.r#continue.builder);
        let str = builder.embed(self.str.builder);
        let strings_list = builder.embed(self.strings_list.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::StringsList,
                vec![r#continue, str, strings_list],
            );
        StringsListNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TextConstructor {
    text: TextToken,
}
impl TextConstructor {
    pub fn build(self) -> TextNode {
        let mut builder = CstBuilder::new();
        let text = builder.embed(self.text.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Text, vec![text]);
        TextNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct TextBindingConstructor {
    text_start: TextStartNode,
    text_binding_opt: TextBindingOptNode,
    text: TextNode,
    text_binding_opt_0: TextBindingOpt0Node,
}
impl TextBindingConstructor {
    pub fn build(self) -> TextBindingNode {
        let mut builder = CstBuilder::new();
        let text_start = builder.embed(self.text_start.builder);
        let text_binding_opt = builder.embed(self.text_binding_opt.builder);
        let text = builder.embed(self.text.builder);
        let text_binding_opt_0 = builder.embed(self.text_binding_opt_0.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::TextBinding,
                vec![text_start, text_binding_opt, text, text_binding_opt_0],
            );
        TextBindingNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TextBindingOptConstructor {
    ws: Option<WsNode>,
}
impl TextBindingOptConstructor {
    pub fn build(self) -> TextBindingOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.ws {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::TextBindingOpt, children);
        TextBindingOptNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TextBindingOpt0Constructor {
    grammar_newline: Option<GrammarNewlineNode>,
}
impl TextBindingOpt0Constructor {
    pub fn build(self) -> TextBindingOpt0Node {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.grammar_newline {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::TextBindingOpt0, children);
        TextBindingOpt0Node {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TextStartConstructor {
    text_start: TextStartToken,
}
impl TextStartConstructor {
    pub fn build(self) -> TextStartNode {
        let mut builder = CstBuilder::new();
        let text_start = builder.embed(self.text_start.builder);
        let node_id = builder.non_terminal(NonTerminalKind::TextStart, vec![text_start]);
        TextStartNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct TrueConstructor {
    r#true: TrueToken,
}
impl TrueConstructor {
    pub fn build(self) -> TrueNode {
        let mut builder = CstBuilder::new();
        let r#true = builder.embed(self.r#true.builder);
        let node_id = builder.non_terminal(NonTerminalKind::True, vec![r#true]);
        TrueNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct TupleConstructor {
    l_paren: LParenNode,
    tuple_opt: TupleOptNode,
    r_paren: RParenNode,
}
impl TupleConstructor {
    pub fn build(self) -> TupleNode {
        let mut builder = CstBuilder::new();
        let l_paren = builder.embed(self.l_paren.builder);
        let tuple_opt = builder.embed(self.tuple_opt.builder);
        let r_paren = builder.embed(self.r_paren.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::Tuple, vec![l_paren, tuple_opt, r_paren]);
        TupleNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct TupleElementsConstructor {
    value: ValueNode,
    tuple_elements_opt: TupleElementsOptNode,
}
impl TupleElementsConstructor {
    pub fn build(self) -> TupleElementsNode {
        let mut builder = CstBuilder::new();
        let value = builder.embed(self.value.builder);
        let tuple_elements_opt = builder.embed(self.tuple_elements_opt.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::TupleElements,
                vec![value, tuple_elements_opt],
            );
        TupleElementsNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TupleElementsOptConstructor {
    tuple_elements_tail: Option<TupleElementsTailNode>,
}
impl TupleElementsOptConstructor {
    pub fn build(self) -> TupleElementsOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.tuple_elements_tail {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::TupleElementsOpt, children);
        TupleElementsOptNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TupleElementsTailConstructor {
    comma: CommaNode,
    tuple_elements_tail_opt: TupleElementsTailOptNode,
}
impl TupleElementsTailConstructor {
    pub fn build(self) -> TupleElementsTailNode {
        let mut builder = CstBuilder::new();
        let comma = builder.embed(self.comma.builder);
        let tuple_elements_tail_opt = builder
            .embed(self.tuple_elements_tail_opt.builder);
        let node_id = builder
            .non_terminal(
                NonTerminalKind::TupleElementsTail,
                vec![comma, tuple_elements_tail_opt],
            );
        TupleElementsTailNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TupleElementsTailOptConstructor {
    tuple_elements: Option<TupleElementsNode>,
}
impl TupleElementsTailOptConstructor {
    pub fn build(self) -> TupleElementsTailOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.tuple_elements {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder
            .non_terminal(NonTerminalKind::TupleElementsTailOpt, children);
        TupleElementsTailOptNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct TupleOptConstructor {
    tuple_elements: Option<TupleElementsNode>,
}
impl TupleOptConstructor {
    pub fn build(self) -> TupleOptNode {
        let mut builder = CstBuilder::new();
        let children = if let Some(child) = self.tuple_elements {
            vec![builder.embed(child.builder)]
        } else {
            Vec::<BuilderNodeId>::new()
        };
        let node_id = builder.non_terminal(NonTerminalKind::TupleOpt, children);
        TupleOptNode { node_id, builder }
    }
}
pub enum ValueConstructor {
    Object(ObjectNode),
    Array(ArrayNode),
    Tuple(TupleNode),
    Integer(IntegerNode),
    Boolean(BooleanNode),
    Null(NullNode),
    Strings(StringsNode),
    Hole(HoleNode),
    CodeBlock(CodeBlockNode),
    NamedCode(NamedCodeNode),
    Code(CodeNode),
    Path(PathNode),
}
impl ValueConstructor {
    pub fn build(self) -> ValueNode {
        let mut builder = CstBuilder::new();
        let child_id = match self {
            Self::Object(node) => builder.embed(node.builder),
            Self::Array(node) => builder.embed(node.builder),
            Self::Tuple(node) => builder.embed(node.builder),
            Self::Integer(node) => builder.embed(node.builder),
            Self::Boolean(node) => builder.embed(node.builder),
            Self::Null(node) => builder.embed(node.builder),
            Self::Strings(node) => builder.embed(node.builder),
            Self::Hole(node) => builder.embed(node.builder),
            Self::CodeBlock(node) => builder.embed(node.builder),
            Self::NamedCode(node) => builder.embed(node.builder),
            Self::Code(node) => builder.embed(node.builder),
            Self::Path(node) => builder.embed(node.builder),
        };
        let node_id = builder.non_terminal(NonTerminalKind::Value, vec![child_id]);
        ValueNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct ValueBindingConstructor {
    bind: BindNode,
    value: ValueNode,
}
impl ValueBindingConstructor {
    pub fn build(self) -> ValueBindingNode {
        let mut builder = CstBuilder::new();
        let bind = builder.embed(self.bind.builder);
        let value = builder.embed(self.value.builder);
        let node_id = builder
            .non_terminal(NonTerminalKind::ValueBinding, vec![bind, value]);
        ValueBindingNode {
            node_id,
            builder,
        }
    }
}
#[derive(bon::Builder)]
pub struct WsConstructor {
    ws: WsToken,
}
impl WsConstructor {
    pub fn build(self) -> WsNode {
        let mut builder = CstBuilder::new();
        let ws = builder.embed(self.ws.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Ws, vec![ws]);
        WsNode { node_id, builder }
    }
}
#[derive(bon::Builder)]
pub struct RootConstructor {
    eure: EureNode,
}
impl RootConstructor {
    pub fn build(self) -> RootNode {
        let mut builder = CstBuilder::new();
        let eure = builder.embed(self.eure.builder);
        let node_id = builder.non_terminal(NonTerminalKind::Root, vec![eure]);
        RootNode { node_id, builder }
    }
}
pub mod terminals {
    use super::*;
    pub fn integer(value: &str) -> IntegerToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Integer, value);
        IntegerToken { node_id, builder }
    }
    pub fn r#true() -> TrueToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::True, "true");
        TrueToken { node_id, builder }
    }
    pub fn r#false() -> FalseToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::False, "false");
        FalseToken { node_id, builder }
    }
    pub fn null() -> NullToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Null, "null");
        NullToken { node_id, builder }
    }
    pub fn hole() -> HoleToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Hole, "!");
        HoleToken { node_id, builder }
    }
    pub fn str(value: &str) -> StrToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Str, value);
        StrToken { node_id, builder }
    }
    pub fn text(value: &str) -> TextToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Text, value);
        TextToken { node_id, builder }
    }
    pub fn code_block(value: &str) -> CodeBlockToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::CodeBlock, value);
        CodeBlockToken { node_id, builder }
    }
    pub fn named_code(value: &str) -> NamedCodeToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::NamedCode, value);
        NamedCodeToken { node_id, builder }
    }
    pub fn code(value: &str) -> CodeToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Code, value);
        CodeToken { node_id, builder }
    }
    pub fn grammar_newline() -> GrammarNewlineToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::GrammarNewline, "\n");
        GrammarNewlineToken {
            node_id,
            builder,
        }
    }
    pub fn ws() -> WsToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Ws, " ");
        WsToken { node_id, builder }
    }
    pub fn at() -> AtToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::At, "@");
        AtToken { node_id, builder }
    }
    pub fn dollar_dollar() -> DollarDollarToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::DollarDollar, "");
        DollarDollarToken {
            node_id,
            builder,
        }
    }
    pub fn dollar() -> DollarToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Dollar, "$");
        DollarToken { node_id, builder }
    }
    pub fn dot() -> DotToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Dot, ".");
        DotToken { node_id, builder }
    }
    pub fn l_brace() -> LBraceToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::LBrace, "{");
        LBraceToken { node_id, builder }
    }
    pub fn r_brace() -> RBraceToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::RBrace, "}");
        RBraceToken { node_id, builder }
    }
    pub fn l_bracket() -> LBracketToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::LBracket, "[");
        LBracketToken { node_id, builder }
    }
    pub fn r_bracket() -> RBracketToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::RBracket, "]");
        RBracketToken { node_id, builder }
    }
    pub fn l_paren() -> LParenToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::LParen, "");
        LParenToken { node_id, builder }
    }
    pub fn r_paren() -> RParenToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::RParen, "");
        RParenToken { node_id, builder }
    }
    pub fn bind() -> BindToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Bind, "=");
        BindToken { node_id, builder }
    }
    pub fn comma() -> CommaToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Comma, ",");
        CommaToken { node_id, builder }
    }
    pub fn esc() -> EscToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Esc, "\\\\");
        EscToken { node_id, builder }
    }
    pub fn text_start() -> TextStartToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::TextStart, "");
        TextStartToken { node_id, builder }
    }
    pub fn ident(name: &str) -> IdentToken {
        let mut builder = CstBuilder::new();
        let node_id = builder.terminal(TerminalKind::Ident, name);
        IdentToken { node_id, builder }
    }
}
