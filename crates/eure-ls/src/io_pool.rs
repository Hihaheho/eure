//! IO thread pool for file reading.

use std::fs;
use std::thread::{self, JoinHandle};

#[cfg(feature = "http")]
use anyhow::anyhow;
use crossbeam_channel::{Receiver, Sender, unbounded};
#[cfg(feature = "http")]
use eure::query::fetch_url;
use eure::query::{TextFile, TextFileContent};

use crate::types::{IoRequest, IoResponse};

/// Thread pool for handling file I/O operations.
///
/// This allows the main event loop to remain responsive while
/// files are being read from disk.
pub struct IoPool {
    /// Channel to send file read requests to workers.
    request_tx: Sender<IoRequest>,
    /// Channel to receive file read responses from workers.
    response_rx: Receiver<IoResponse>,
    /// Worker thread handles (kept for cleanup).
    _workers: Vec<JoinHandle<()>>,
}

impl IoPool {
    /// Create a new IO pool with the specified number of worker threads.
    pub fn new(num_workers: usize) -> Self {
        let (request_tx, request_rx) = unbounded::<IoRequest>();
        let (response_tx, response_rx) = unbounded::<IoResponse>();

        let mut workers = Vec::with_capacity(num_workers);

        for i in 0..num_workers {
            let request_rx = request_rx.clone();
            let response_tx = response_tx.clone();

            let handle = thread::Builder::new()
                .name(format!("eure-ls-io-{}", i))
                .spawn(move || {
                    worker_loop(request_rx, response_tx);
                })
                .expect("failed to spawn IO worker thread");

            workers.push(handle);
        }

        Self {
            request_tx,
            response_rx,
            _workers: workers,
        }
    }

    /// Request a file to be read.
    ///
    /// The result will be available via `receiver()`.
    pub fn request_file(&self, file: TextFile) {
        // Ignore send errors - they only happen if all workers have died
        let _ = self.request_tx.send(IoRequest { file });
    }

    /// Get the receiver for file read responses.
    ///
    /// Use this with `crossbeam_channel::select!` to wait for responses.
    pub fn receiver(&self) -> &Receiver<IoResponse> {
        &self.response_rx
    }
}

/// Worker loop that reads files from disk.
fn worker_loop(request_rx: Receiver<IoRequest>, response_tx: Sender<IoResponse>) {
    for request in request_rx {
        let result = read_file(&request.file);
        let response = IoResponse {
            file: request.file,
            result,
        };

        // If the main thread has stopped listening, just exit
        if response_tx.send(response).is_err() {
            break;
        }
    }
}

/// Read a file from disk or fetch from URL and return its content.
fn read_file(file: &TextFile) -> Result<TextFileContent, anyhow::Error> {
    match file {
        TextFile::Local(path) => match fs::read_to_string(path.as_ref()) {
            Ok(content) => Ok(TextFileContent::Content(content)),
            Err(_) => Ok(TextFileContent::NotFound),
        },
        TextFile::Remote(url) => match fetch_url(url) {
            Ok(content) => Ok(content),
            Err(e) => Err(anyhow!("Failed to fetch {}: {}", url, e)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_read_nonexistent_file() {
        let file = TextFile::from_path(PathBuf::from("/nonexistent/path/to/file.eure"));
        let result = read_file(&file);
        assert!(matches!(result, Ok(TextFileContent::NotFound)));
    }
}
