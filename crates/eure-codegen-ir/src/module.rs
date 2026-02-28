use indexmap::IndexMap;

use crate::codegen::{CodegenDefaultsIr, RootCodegenIr, TypeCodegenIr};
use crate::emission::EmissionDefaultsIr;
use crate::error::IrBuildError;
use crate::ids::{QualifiedTypeName, SchemaNodeIrId, TypeId};
use crate::rust_binding::RustBindingIr;
use crate::schema::SchemaNodeIr;

#[derive(Debug, Clone, PartialEq, bon::Builder, Default)]
#[builder(finish_fn = build_unchecked)]
pub struct IrModule {
    #[builder(default)]
    types: IndexMap<TypeId, TypeDefIr>,
    #[builder(default)]
    name_index: IndexMap<QualifiedTypeName, TypeId>,
    #[builder(default)]
    roots: Vec<TypeId>,
    #[builder(default)]
    root_codegen: RootCodegenIr,
    #[builder(default)]
    codegen_defaults: CodegenDefaultsIr,
    #[builder(default)]
    emission_defaults: EmissionDefaultsIr,
}

impl IrModule {
    pub fn new(
        types: IndexMap<TypeId, TypeDefIr>,
        name_index: IndexMap<QualifiedTypeName, TypeId>,
        roots: Vec<TypeId>,
        root_codegen: RootCodegenIr,
        codegen_defaults: CodegenDefaultsIr,
        emission_defaults: EmissionDefaultsIr,
    ) -> Result<Self, IrBuildError> {
        Self {
            types,
            name_index,
            roots,
            root_codegen,
            codegen_defaults,
            emission_defaults,
        }
        .into_checked()
    }

    pub fn into_checked(self) -> Result<Self, IrBuildError> {
        self.validate()?;
        Ok(self)
    }

    pub fn types(&self) -> &IndexMap<TypeId, TypeDefIr> {
        &self.types
    }

    pub fn types_mut(&mut self) -> &mut IndexMap<TypeId, TypeDefIr> {
        &mut self.types
    }

    pub fn name_index(&self) -> &IndexMap<QualifiedTypeName, TypeId> {
        &self.name_index
    }

    pub fn name_index_mut(&mut self) -> &mut IndexMap<QualifiedTypeName, TypeId> {
        &mut self.name_index
    }

    pub fn roots(&self) -> &[TypeId] {
        &self.roots
    }

    pub fn roots_mut(&mut self) -> &mut Vec<TypeId> {
        &mut self.roots
    }

    pub fn root_codegen(&self) -> &RootCodegenIr {
        &self.root_codegen
    }

    pub fn root_codegen_mut(&mut self) -> &mut RootCodegenIr {
        &mut self.root_codegen
    }

    pub fn codegen_defaults(&self) -> &CodegenDefaultsIr {
        &self.codegen_defaults
    }

    pub fn codegen_defaults_mut(&mut self) -> &mut CodegenDefaultsIr {
        &mut self.codegen_defaults
    }

    pub fn emission_defaults(&self) -> &EmissionDefaultsIr {
        &self.emission_defaults
    }

    pub fn emission_defaults_mut(&mut self) -> &mut EmissionDefaultsIr {
        &mut self.emission_defaults
    }

    pub(crate) fn validate(&self) -> Result<(), IrBuildError> {
        crate::build_check::ensure_module_invariants(self)
    }

    pub fn get_type(&self, id: &TypeId) -> Option<&TypeDefIr> {
        self.types.get(id)
    }

    pub fn get_type_by_name(&self, name: &QualifiedTypeName) -> Option<&TypeDefIr> {
        self.name_index.get(name).and_then(|id| self.types.get(id))
    }

    pub fn insert_type(&mut self, id: TypeId, ty: TypeDefIr) -> Option<TypeDefIr> {
        self.types.insert(id, ty)
    }

    pub fn insert_name_index(&mut self, name: QualifiedTypeName, id: TypeId) -> Option<TypeId> {
        self.name_index.insert(name, id)
    }

    pub fn push_root(&mut self, id: TypeId) {
        self.roots.push(id);
    }

    pub fn set_roots(&mut self, roots: Vec<TypeId>) {
        self.roots = roots;
    }

    pub fn set_root_codegen(&mut self, root_codegen: RootCodegenIr) {
        self.root_codegen = root_codegen;
    }

    pub fn set_codegen_defaults(&mut self, codegen_defaults: CodegenDefaultsIr) {
        self.codegen_defaults = codegen_defaults;
    }

    pub fn set_emission_defaults(&mut self, emission_defaults: EmissionDefaultsIr) {
        self.emission_defaults = emission_defaults;
    }
}

impl IrModuleBuilder {
    pub fn build(self) -> Result<IrModule, IrBuildError> {
        self.build_unchecked().into_checked()
    }
}

#[derive(Debug, Clone, PartialEq, bon::Builder)]
#[builder(finish_fn = build_unchecked)]
pub struct TypeDefIr {
    id: TypeId,
    names: TypeNamesIr,
    schema_nodes: IndexMap<SchemaNodeIrId, SchemaNodeIr>,
    semantic_root: SchemaNodeIrId,
    rust_binding: RustBindingIr,
    type_codegen: TypeCodegenIr,
    origin: TypeOriginIr,
}

impl TypeDefIr {
    pub fn new(
        id: TypeId,
        names: TypeNamesIr,
        schema_nodes: IndexMap<SchemaNodeIrId, SchemaNodeIr>,
        semantic_root: SchemaNodeIrId,
        rust_binding: RustBindingIr,
        type_codegen: TypeCodegenIr,
        origin: TypeOriginIr,
    ) -> Self {
        Self {
            id,
            names,
            schema_nodes,
            semantic_root,
            rust_binding,
            type_codegen,
            origin,
        }
    }

    pub fn id(&self) -> &TypeId {
        &self.id
    }

    pub fn names(&self) -> &TypeNamesIr {
        &self.names
    }

    pub fn schema_nodes(&self) -> &IndexMap<SchemaNodeIrId, SchemaNodeIr> {
        &self.schema_nodes
    }

    pub fn schema_nodes_mut(&mut self) -> &mut IndexMap<SchemaNodeIrId, SchemaNodeIr> {
        &mut self.schema_nodes
    }

    pub fn semantic_root(&self) -> SchemaNodeIrId {
        self.semantic_root
    }

    pub fn rust_binding(&self) -> &RustBindingIr {
        &self.rust_binding
    }

    pub fn rust_binding_mut(&mut self) -> &mut RustBindingIr {
        &mut self.rust_binding
    }

    pub fn type_codegen(&self) -> &TypeCodegenIr {
        &self.type_codegen
    }

    pub fn type_codegen_mut(&mut self) -> &mut TypeCodegenIr {
        &mut self.type_codegen
    }

    pub fn origin(&self) -> &TypeOriginIr {
        &self.origin
    }
}

#[derive(Debug, Clone, PartialEq, Eq, bon::Builder)]
#[builder(finish_fn = build_unchecked)]
pub struct TypeNamesIr {
    rust_name: String,
    schema_name: Option<QualifiedTypeName>,
}

impl TypeNamesIr {
    pub fn new(rust_name: String, schema_name: Option<QualifiedTypeName>) -> Self {
        Self {
            rust_name,
            schema_name,
        }
    }

    pub fn rust_name(&self) -> &str {
        &self.rust_name
    }

    pub fn schema_name(&self) -> Option<&QualifiedTypeName> {
        self.schema_name.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeOriginIr {
    Derive,
    Schema,
    Mixed,
}
