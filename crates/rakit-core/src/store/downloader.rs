use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use reqwest::StatusCode;

use crate::models::{InstallProgress, InstallState};
use crate::store::StoreError;

pub struct DownloadOptions {
    pub url: String,
    pub destination_file: PathBuf,
    pub install_id: String,
    pub catalog_id: String,
}

pub struct ChunkDownloader {
    client: reqwest::Client,
}

impl Default for ChunkDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Downloads the file at `options.url` into `options.destination_file`.
    /// Streams into `.part` first with Range resume, retries on network error with exponential backoff,
    /// throttles progress notifications, and supports graceful cancellation.
    pub async fn download<F>(
        &self,
        options: DownloadOptions,
        cancel_token: Arc<AtomicBool>,
        mut on_progress: F,
    ) -> Result<PathBuf, StoreError>
    where
        F: FnMut(InstallProgress) + Send + 'static,
    {
        // 1. Verify HTTPS-only requirement
        if !options.url.starts_with("https://") {
            return Err(StoreError::InvalidUrl(format!(
                "Non-HTTPS download URL rejected: {}",
                options.url
            )));
        }

        let part_file = match options.destination_file.extension() {
            Some(ext) => options
                .destination_file
                .with_extension(format!("{}.part", ext.to_string_lossy())),
            None => options.destination_file.with_extension("part"),
        };

        if let Some(parent) = part_file.parent() {
            fs::create_dir_all(parent).map_err(StoreError::Io)?;
        }

        let mut backoff = Duration::from_secs(1);
        let max_retries = 5;
        let mut attempts = 0;

        loop {
            if cancel_token.load(Ordering::SeqCst) {
                return Err(StoreError::Cancelled);
            }

            match self
                .stream_single_attempt(&options, &part_file, &cancel_token, &mut on_progress)
                .await
            {
                Ok(total_bytes) => {
                    // Success: rename .part to destination_file
                    fs::rename(&part_file, &options.destination_file).map_err(StoreError::Io)?;

                    // Emit finished progress
                    on_progress(InstallProgress {
                        install_id: options.install_id.clone(),
                        catalog_id: options.catalog_id.clone(),
                        state: InstallState::VerifyingSha256,
                        bytes_downloaded: total_bytes,
                        total_bytes,
                        speed_bps: 0,
                        eta_secs: Some(0),
                        stripped_bytes: 0,
                        message: Some("Download complete, verifying SHA-256...".to_string()),
                        error: None,
                    });

                    return Ok(options.destination_file);
                }
                Err(err) => {
                    if cancel_token.load(Ordering::SeqCst) || matches!(err, StoreError::Cancelled) {
                        return Err(StoreError::Cancelled);
                    }

                    attempts += 1;
                    if attempts >= max_retries {
                        return Err(StoreError::Network(format!(
                            "Failed to download after {} attempts: {}",
                            attempts, err
                        )));
                    }

                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(16));
                }
            }
        }
    }

    async fn stream_single_attempt<F>(
        &self,
        options: &DownloadOptions,
        part_file: &Path,
        cancel_token: &Arc<AtomicBool>,
        on_progress: &mut F,
    ) -> Result<u64, StoreError>
    where
        F: FnMut(InstallProgress) + Send + 'static,
    {
        let mut existing_bytes = 0u64;
        if part_file.exists() {
            existing_bytes = fs::metadata(part_file).map(|m| m.len()).unwrap_or(0);
        }

        let mut headers = HeaderMap::new();
        if existing_bytes > 0 {
            let range_val = format!("bytes={}-", existing_bytes);
            if let Ok(val) = HeaderValue::from_str(&range_val) {
                headers.insert(RANGE, val);
            }
        }

        let response = self
            .client
            .get(&options.url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| StoreError::Network(e.to_string()))?;

        let status = response.status();
        let (mut file, mut downloaded_bytes, total_bytes) = if status == StatusCode::PARTIAL_CONTENT
        {
            // Server honors Range -> append
            let content_len = response.content_length().unwrap_or(0);
            let total = existing_bytes + content_len;
            let mut f = OpenOptions::new()
                .write(true)
                .open(part_file)
                .map_err(StoreError::Io)?;
            f.seek(SeekFrom::End(0)).map_err(StoreError::Io)?;
            (f, existing_bytes, total)
        } else if status.is_success() {
            // Server does not support range or fresh download -> truncate/create
            let total = response.content_length().unwrap_or(0);
            let f = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(part_file)
                .map_err(StoreError::Io)?;
            (f, 0u64, total)
        } else {
            return Err(StoreError::Network(format!(
                "HTTP error response: {}",
                status
            )));
        };

        let mut stream = response.bytes_stream();
        let mut last_progress_tick = Instant::now();
        let mut speed_window: VecDeque<(Instant, u64)> = VecDeque::new();

        while let Some(chunk_res) = stream.next().await {
            if cancel_token.load(Ordering::SeqCst) {
                return Err(StoreError::Cancelled);
            }

            let chunk = chunk_res.map_err(|e| StoreError::Network(e.to_string()))?;
            file.write_all(&chunk).map_err(StoreError::Io)?;
            downloaded_bytes += chunk.len() as u64;

            let now = Instant::now();
            speed_window.push_back((now, downloaded_bytes));
            // Keep speed samples within last 2 seconds
            while let Some((t, _)) = speed_window.front() {
                if now.duration_since(*t) > Duration::from_secs(2) && speed_window.len() > 1 {
                    speed_window.pop_front();
                } else {
                    break;
                }
            }

            // Progress throttling: ~30-50ms (or on completion)
            if now.duration_since(last_progress_tick) >= Duration::from_millis(32)
                || downloaded_bytes == total_bytes
            {
                last_progress_tick = now;

                // Calculate speed
                let speed_bps = if let (Some((first_t, first_b)), Some((last_t, last_b))) =
                    (speed_window.front(), speed_window.back())
                {
                    let elapsed = last_t.duration_since(*first_t).as_secs_f64();
                    if elapsed > 0.05 {
                        ((last_b - first_b) as f64 / elapsed) as u64
                    } else {
                        0
                    }
                } else {
                    0
                };

                let eta_secs = if speed_bps > 0 && total_bytes > downloaded_bytes {
                    Some((total_bytes - downloaded_bytes) / speed_bps)
                } else {
                    None
                };

                on_progress(InstallProgress {
                    install_id: options.install_id.clone(),
                    catalog_id: options.catalog_id.clone(),
                    state: InstallState::Downloading,
                    bytes_downloaded: downloaded_bytes,
                    total_bytes,
                    speed_bps,
                    eta_secs,
                    stripped_bytes: 0,
                    message: None,
                    error: None,
                });
            }
        }

        file.flush().map_err(StoreError::Io)?;
        Ok(downloaded_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_downloader_rejects_http() {
        let downloader = ChunkDownloader::new();
        let options = DownloadOptions {
            url: "http://example.com/runtime.tar.gz".to_string(),
            destination_file: PathBuf::from("/tmp/test.tar.gz"),
            install_id: "test_inst".to_string(),
            catalog_id: "php@8.3".to_string(),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let res = downloader.download(options, cancel, |_| {}).await;

        assert!(matches!(res, Err(StoreError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn test_downloader_cancelled() {
        let downloader = ChunkDownloader::new();
        let options = DownloadOptions {
            url: "https://example.com/runtime.tar.gz".to_string(),
            destination_file: PathBuf::from("/tmp/test_cancel.tar.gz"),
            install_id: "test_cancel".to_string(),
            catalog_id: "php@8.3".to_string(),
        };
        let cancel = Arc::new(AtomicBool::new(true)); // cancelled immediately
        let res = downloader.download(options, cancel, |_| {}).await;

        assert!(matches!(res, Err(StoreError::Cancelled)));
    }
}
