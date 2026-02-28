use crate::emission::TypeEmissionConfigIr;
use crate::ids::{QualifiedTypeName, RustPathIr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustBindingIr {
    kind: RustTypeKindIr,
    container: ContainerAttrsIr,
    fields: Vec<RustFieldIr>,
    variants: Vec<RustVariantIr>,
    generics: RustGenericsIr,
    where_clause: WhereClauseIr,
    emission: TypeEmissionConfigIr,
}

impl Default for RustBindingIr {
    fn default() -> Self {
        Self {
            kind: RustTypeKindIr::Unit,
            container: ContainerAttrsIr::default(),
            fields: Vec::new(),
            variants: Vec::new(),
            generics: RustGenericsIr::default(),
            where_clause: WhereClauseIr::default(),
            emission: TypeEmissionConfigIr::default(),
        }
    }
}

impl RustBindingIr {
    pub fn new(
        kind: RustTypeKindIr,
        container: ContainerAttrsIr,
        fields: Vec<RustFieldIr>,
        variants: Vec<RustVariantIr>,
        generics: RustGenericsIr,
        where_clause: WhereClauseIr,
        emission: TypeEmissionConfigIr,
    ) -> Self {
        Self {
            kind,
            container,
            fields,
            variants,
            generics,
            where_clause,
            emission,
        }
    }

    pub fn kind(&self) -> &RustTypeKindIr {
        &self.kind
    }

    pub fn container(&self) -> &ContainerAttrsIr {
        &self.container
    }

    pub fn container_mut(&mut self) -> &mut ContainerAttrsIr {
        &mut self.container
    }

    pub fn fields(&self) -> &[RustFieldIr] {
        &self.fields
    }

    pub fn fields_mut(&mut self) -> &mut Vec<RustFieldIr> {
        &mut self.fields
    }

    pub fn variants(&self) -> &[RustVariantIr] {
        &self.variants
    }

    pub fn variants_mut(&mut self) -> &mut Vec<RustVariantIr> {
        &mut self.variants
    }

    pub fn generics(&self) -> &RustGenericsIr {
        &self.generics
    }

    pub fn generics_mut(&mut self) -> &mut RustGenericsIr {
        &mut self.generics
    }

    pub fn where_clause(&self) -> &WhereClauseIr {
        &self.where_clause
    }

    pub fn where_clause_mut(&mut self) -> &mut WhereClauseIr {
        &mut self.where_clause
    }

    pub fn emission(&self) -> &TypeEmissionConfigIr {
        &self.emission
    }

    pub fn emission_mut(&mut self) -> &mut TypeEmissionConfigIr {
        &mut self.emission
    }

    pub fn set_kind(&mut self, kind: RustTypeKindIr) {
        self.kind = kind;
    }

    pub fn push_field(&mut self, field: RustFieldIr) {
        self.fields.push(field);
    }

    pub fn push_variant(&mut self, variant: RustVariantIr) {
        self.variants.push(variant);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustTypeKindIr {
    Record,
    Newtype,
    Tuple,
    Unit,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerAttrsIr {
    crate_path: Option<RustPathIr>,
    rename_all: Option<RenameRuleIr>,
    rename_all_fields: Option<RenameRuleIr>,
    parse_ext: bool,
    allow_unknown_fields: bool,
    allow_unknown_extensions: bool,
    parse_error: Option<RustPathIr>,
    write_error: Option<RustPathIr>,
    type_name: Option<String>,
    non_exhaustive: bool,
    proxy_target: Option<RustPathIr>,
    opaque_target: Option<RustPathIr>,
}

impl ContainerAttrsIr {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        crate_path: Option<RustPathIr>,
        rename_all: Option<RenameRuleIr>,
        rename_all_fields: Option<RenameRuleIr>,
        parse_ext: bool,
        allow_unknown_fields: bool,
        allow_unknown_extensions: bool,
        parse_error: Option<RustPathIr>,
        write_error: Option<RustPathIr>,
        type_name: Option<String>,
        non_exhaustive: bool,
        proxy_target: Option<RustPathIr>,
        opaque_target: Option<RustPathIr>,
    ) -> Self {
        Self {
            crate_path,
            rename_all,
            rename_all_fields,
            parse_ext,
            allow_unknown_fields,
            allow_unknown_extensions,
            parse_error,
            write_error,
            type_name,
            non_exhaustive,
            proxy_target,
            opaque_target,
        }
    }

    pub fn crate_path(&self) -> Option<&RustPathIr> {
        self.crate_path.as_ref()
    }

    pub fn rename_all(&self) -> Option<RenameRuleIr> {
        self.rename_all.clone()
    }

    pub fn rename_all_fields(&self) -> Option<RenameRuleIr> {
        self.rename_all_fields.clone()
    }

    pub fn parse_ext(&self) -> bool {
        self.parse_ext
    }

    pub fn allow_unknown_fields(&self) -> bool {
        self.allow_unknown_fields
    }

    pub fn allow_unknown_extensions(&self) -> bool {
        self.allow_unknown_extensions
    }

    pub fn parse_error(&self) -> Option<&RustPathIr> {
        self.parse_error.as_ref()
    }

    pub fn write_error(&self) -> Option<&RustPathIr> {
        self.write_error.as_ref()
    }

    pub fn type_name(&self) -> Option<&str> {
        self.type_name.as_deref()
    }

    pub fn non_exhaustive(&self) -> bool {
        self.non_exhaustive
    }

    pub fn proxy_target(&self) -> Option<&RustPathIr> {
        self.proxy_target.as_ref()
    }

    pub fn opaque_target(&self) -> Option<&RustPathIr> {
        self.opaque_target.as_ref()
    }

    pub fn proxy_mode(&self) -> Option<ProxyModeIr> {
        match (&self.proxy_target, &self.opaque_target) {
            (Some(target), None) => Some(ProxyModeIr::Transparent(target.clone())),
            (None, Some(target)) => Some(ProxyModeIr::Opaque(target.clone())),
            _ => None,
        }
    }

    pub fn proxy_target_mut(&mut self) -> &mut Option<RustPathIr> {
        &mut self.proxy_target
    }

    pub fn opaque_target_mut(&mut self) -> &mut Option<RustPathIr> {
        &mut self.opaque_target
    }

    pub fn parse_ext_mut(&mut self) -> &mut bool {
        &mut self.parse_ext
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyModeIr {
    Transparent(RustPathIr),
    Opaque(RustPathIr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustFieldIr {
    rust_name: String,
    wire_name: String,
    mode: FieldModeIr,
    source_attrs: FieldSourceAttrsIr,
    ty: RustTypeExprIr,
    default: DefaultValueIr,
    via: Option<RustPathIr>,
}

impl RustFieldIr {
    pub fn new(
        rust_name: String,
        wire_name: String,
        mode: FieldModeIr,
        source_attrs: FieldSourceAttrsIr,
        ty: RustTypeExprIr,
        default: DefaultValueIr,
        via: Option<RustPathIr>,
    ) -> Self {
        Self {
            rust_name,
            wire_name,
            mode,
            source_attrs,
            ty,
            default,
            via,
        }
    }

    pub fn rust_name(&self) -> &str {
        &self.rust_name
    }

    pub fn wire_name(&self) -> &str {
        &self.wire_name
    }

    pub fn mode(&self) -> &FieldModeIr {
        &self.mode
    }

    pub fn source_attrs(&self) -> &FieldSourceAttrsIr {
        &self.source_attrs
    }

    pub fn ty(&self) -> &RustTypeExprIr {
        &self.ty
    }

    pub fn default(&self) -> &DefaultValueIr {
        &self.default
    }

    pub fn via(&self) -> Option<&RustPathIr> {
        self.via.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldModeIr {
    Record,
    Ext,
    Flatten,
    FlattenExt,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FieldSourceAttrsIr {
    pub ext: bool,
    pub flatten: bool,
    pub flatten_ext: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DefaultValueIr {
    #[default]
    None,
    DefaultTrait,
    Function(RustPathIr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustVariantIr {
    rust_name: String,
    wire_name: String,
    allow_unknown_fields: bool,
    shape: VariantShapeIr,
}

impl RustVariantIr {
    pub fn new(
        rust_name: String,
        wire_name: String,
        allow_unknown_fields: bool,
        shape: VariantShapeIr,
    ) -> Self {
        Self {
            rust_name,
            wire_name,
            allow_unknown_fields,
            shape,
        }
    }

    pub fn rust_name(&self) -> &str {
        &self.rust_name
    }

    pub fn wire_name(&self) -> &str {
        &self.wire_name
    }

    pub fn allow_unknown_fields(&self) -> bool {
        self.allow_unknown_fields
    }

    pub fn shape(&self) -> &VariantShapeIr {
        &self.shape
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantShapeIr {
    Unit,
    Newtype {
        ty: RustTypeExprIr,
        via: Option<RustPathIr>,
    },
    Tuple(Vec<TupleElementIr>),
    Record(Vec<RustFieldIr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleElementIr {
    pub ty: RustTypeExprIr,
    pub via: Option<RustPathIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustTypeExprIr {
    Primitive(PrimitiveRustTypeIr),
    Named(QualifiedTypeName),
    Path(RustPathIr),
    GenericParam(String),
    Option(Box<RustTypeExprIr>),
    Vec(Box<RustTypeExprIr>),
    Map {
        key: Box<RustTypeExprIr>,
        value: Box<RustTypeExprIr>,
        impl_type: MapImplTypeIr,
    },
    Tuple(Vec<RustTypeExprIr>),
    Result {
        ok: Box<RustTypeExprIr>,
        err: Box<RustTypeExprIr>,
    },
    Wrapper {
        inner: Box<RustTypeExprIr>,
        wrapper: WrapperKindIr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveRustTypeIr {
    String,
    Bool,
    Unit,
    Text,
    Any,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapImplTypeIr {
    HashMap,
    BTreeMap,
    IndexMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrapperKindIr {
    Box,
    Rc,
    Arc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameRuleIr {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    Cobol,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RustGenericsIr {
    pub type_params: Vec<TypeParamIr>,
    pub lifetime_params: Vec<LifetimeParamIr>,
    pub const_params: Vec<ConstParamIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParamIr {
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifetimeParamIr {
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstParamIr {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WhereClauseIr {
    pub predicates: Vec<String>,
}
