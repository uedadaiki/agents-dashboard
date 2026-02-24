use super::jsonl_parser::{parse_jsonl_chunk, RawEntry};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::sync::mpsc;
use tracing::{debug, error};

pub struct FileWatcher {
    file_path: PathBuf,
    tx: mpsc::UnboundedSender<Vec<RawEntry>>,
    shutdown: tokio::sync::watch::Sender<bool>,
}

impl FileWatcher {
    pub fn new(file_path: PathBuf, tx: mpsc::UnboundedSender<Vec<RawEntry>>) -> Self {
        let (shutdown, _) = tokio::sync::watch::channel(false);
        Self {
            file_path,
            tx,
            shutdown,
        }
    }

    pub async fn start(&self) {
        let file_path = self.file_path.clone();
        let tx = self.tx.clone();
        let mut shutdown_rx = self.shutdown.subscribe();

        tokio::spawn(async move {
            let mut offset: u64 = 0;
            let mut remainder = String::new();

            // Initial read
            if let Err(e) = read_new_content(&file_path, &mut offset, &mut remainder, &tx).await {
                error!("Initial read error for {}: {}", file_path.display(), e);
            }

            // Set up notify watcher
            let (notify_tx, mut notify_rx) = mpsc::unbounded_channel();

            let _watcher = {
                use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
                let notify_tx_clone = notify_tx.clone();
                let mut watcher = RecommendedWatcher::new(
                    move |res: Result<Event, notify::Error>| {
                        if let Ok(_event) = res {
                            let _ = notify_tx_clone.send(());
                        }
                    },
                    Config::default(),
                )
                .ok();

                if let Some(ref mut w) = watcher {
                    if let Err(e) = w.watch(&file_path, RecursiveMode::NonRecursive) {
                        error!("Failed to watch {}: {}", file_path.display(), e);
                    }
                }
                watcher
            };

            // Polling interval (2s fallback)
            let mut poll_interval = tokio::time::interval(std::time::Duration::from_secs(2));
            poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        debug!("FileWatcher shutting down for {}", file_path.display());
                        break;
                    }
                    _ = notify_rx.recv() => {
                        if let Err(e) = read_new_content(&file_path, &mut offset, &mut remainder, &tx).await {
                            error!("Read error for {}: {}", file_path.display(), e);
                        }
                    }
                    _ = poll_interval.tick() => {
                        if let Err(e) = read_new_content(&file_path, &mut offset, &mut remainder, &tx).await {
                            error!("Poll read error for {}: {}", file_path.display(), e);
                        }
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        let _ = self.shutdown.send(true);
    }
}

async fn read_new_content(
    file_path: &Path,
    offset: &mut u64,
    remainder: &mut String,
    tx: &mpsc::UnboundedSender<Vec<RawEntry>>,
) -> Result<(), std::io::Error> {
    let metadata = match tokio::fs::metadata(file_path).await {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    let size = metadata.len();
    if size <= *offset {
        return Ok(());
    }

    let mut file = File::open(file_path).await?;
    file.seek(SeekFrom::Start(*offset)).await?;

    let to_read = (size - *offset) as usize;
    let mut buf = vec![0u8; to_read];
    file.read_exact(&mut buf).await?;
    *offset = size;

    let text = String::from_utf8_lossy(&buf);
    let full_chunk = format!("{}{}", remainder, text);
    let result = parse_jsonl_chunk(&full_chunk);
    *remainder = result.remainder;

    if !result.entries.is_empty() {
        let _ = tx.send(result.entries);
    }

    Ok(())
}
