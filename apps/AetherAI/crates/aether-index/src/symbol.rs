#![forbid(unsafe_code)]

use aether_core::SourceRange;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Module,
    Struct,
    Enum,
    Trait,
    Impl,
    Function,
    Method,
    Const,
    Static,
    Test,
    Macro,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSymbol {
    pub id: Uuid,
    pub file_id: Uuid,
    pub kind: SymbolKind,
    pub name: String,
    pub qualified_name: Option<String>,
    pub range: SourceRange,
    pub parent_symbol: Option<Uuid>,
}
