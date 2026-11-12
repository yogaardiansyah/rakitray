pub mod catalog;
pub mod downloader;
pub mod extractor;
pub mod sandbox;
pub mod strip;
pub mod verifier;

pub use catalog::{detect_host_platform, CatalogManager, InstallReceipt};
pub use downloader::{ChunkDownloader, DownloadOptions};
pub use extractor::{ExtractOptions, ExtractionSummary, LeanExtractor};
pub use sandbox::{validate_safe_relative_path, validate_symlink_target};
pub use strip::{should_strip_path, strip_components};
pub use verifier::verify_and_clean_file_sha256;

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("SHA-256 checksum mismatch (expected: {expected}, got: {actual})")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Installation cancelled")]
    Cancelled,

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

impl StoreError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidUrl(_) => "invalid_url",
            Self::Network(_) => "network",
            Self::ChecksumMismatch { .. } => "checksum_mismatch",
            Self::PathTraversal(_) => "path_traversal",
            Self::Extraction(_) => "extraction_error",
            Self::Cancelled => "cancelled",
            Self::Io(_) => "io_error",
        }
    }
}
