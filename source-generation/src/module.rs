#[derive(Debug, Clone)]
pub enum SymbolExportType {
    Use { as_name: Option<String> },
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SymbolExportTypeDiscriminant {
    Use,
    Mod,
}

impl SymbolExportType {
    pub(crate) fn kind(&self) -> SymbolExportTypeDiscriminant {
        match self {
            SymbolExportType::Use { .. } => SymbolExportTypeDiscriminant::Use,
            SymbolExportType::Mod => SymbolExportTypeDiscriminant::Mod,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExportSymbol {
    pub path: Vec<String>,
    pub export_type: SymbolExportType,
    pub public: bool,
}
