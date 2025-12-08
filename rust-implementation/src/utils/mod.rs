// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// UTILITÁRIOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Utilitários compartilhados por todo o sistema:
// - Token tracking e budget management
// - Text processing
// - Timing e performance
// - File reading (PDFs, documents)
// - Text segmentation (chunking)
// - Semantic reference building
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Sistema de referências semânticas usando embeddings e cosine similarity.
pub mod build_ref;
mod file_reader;
/// Chunking de texto para processamento de referências.
pub mod segment;
mod text;
mod timing;
mod token_tracker;

pub use build_ref::{ReferenceBuilder, ReferenceBuilderConfig, ReferenceError, ReferenceResult};
pub use file_reader::{FileContent, FileReader, FileReaderError, FileType};
pub use segment::{chunk_text, ChunkOptions, ChunkResult, ChunkType};
pub use text::*;
pub use timing::{ActionTimer, TimingStats};
pub use token_tracker::{TokenTracker, TrackerStats};
