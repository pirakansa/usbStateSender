use std::io;
use std::path::PathBuf;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("{0}")]
    Usage(String),
    #[error("no matching Pico Inkpack Status HID device found under {0}. use --device PATH to select one explicitly")]
    DeviceNotFound(PathBuf),
    #[error("found multiple matching Pico Inkpack Status HID devices: {0}. use --device PATH to select one explicitly")]
    MultipleDevices(String),
    #[error("failed to enumerate {path}: {source}")]
    EnumeratePath { path: PathBuf, source: io::Error },
    #[error("failed to read {path}: {source}")]
    ReadPath { path: PathBuf, source: io::Error },
    #[error("failed to canonicalize {path}: {source}")]
    CanonicalizePath { path: PathBuf, source: io::Error },
    #[error("failed to write {path}: {source}")]
    WritePath { path: PathBuf, source: io::Error },
    #[error("failed to write command output: {0}")]
    Output(io::Error),
}
