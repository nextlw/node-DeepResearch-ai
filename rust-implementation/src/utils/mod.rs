// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// UTILITÁRIOS
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Utilitários compartilhados por todo o sistema:
// - Token tracking e budget management
// - Text processing
// - Timing e performance
// - File reading (PDFs, documents)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod file_reader;
mod text;
mod timing;
mod token_tracker;

pub use file_reader::{FileContent, FileReader, FileReaderError, FileType};
pub use text::*;
pub use timing::{ActionTimer, TimingStats};
pub use token_tracker::{TokenTracker, TrackerStats};
