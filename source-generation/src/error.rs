use std::io;

#[derive(Debug)]
pub enum SourceGenerationError {
    CodeFileExists,
    CodeFileAlreadyGenerated,
    CodeFileCanNotBeAbsolutePath,
    IoError(io::Error),
    InvalidUtf8,
    SymbolAlreadyRegistered,
    NestedSymbolGenerationNotSupported,
}

impl From<io::Error> for SourceGenerationError {
    fn from(err: io::Error) -> Self {
        SourceGenerationError::IoError(err)
    }
}

impl PartialEq for SourceGenerationError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SourceGenerationError::CodeFileExists, SourceGenerationError::CodeFileExists)
            | (
                SourceGenerationError::CodeFileAlreadyGenerated,
                SourceGenerationError::CodeFileAlreadyGenerated,
            )
            | (
                SourceGenerationError::CodeFileCanNotBeAbsolutePath,
                SourceGenerationError::CodeFileCanNotBeAbsolutePath,
            )
            | (SourceGenerationError::InvalidUtf8, SourceGenerationError::InvalidUtf8)
            | (
                SourceGenerationError::SymbolAlreadyRegistered,
                SourceGenerationError::SymbolAlreadyRegistered,
            )
            | (
                SourceGenerationError::NestedSymbolGenerationNotSupported,
                SourceGenerationError::NestedSymbolGenerationNotSupported,
            ) => true,
            (SourceGenerationError::IoError(err1), SourceGenerationError::IoError(err2)) => {
                err1.kind() == err2.kind()
            }
            _ => false,
        }
    }
}

impl Eq for SourceGenerationError {}
