use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct DiscoveredSession {
    pub session_id: String,
    pub log_file: PathBuf,
    pub project_path: String,
    pub project_name: String,
}

pub enum DiscoveryEvent {
    Found(DiscoveredSession),
    Removed(String),
}

pub struct SessionDiscovery {
    claude_projects_dir: PathBuf,
    known_sessions: HashMap<String, DiscoveredSession>,
    tx: mpsc::UnboundedSender<DiscoveryEvent>,
    shutdown: tokio::sync::watch::Sender<bool>,
}

impl SessionDiscovery {
    pub fn new(tx: mpsc::UnboundedSender<DiscoveryEvent>) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let claude_projects_dir = home.join(".claude").join("projects");

        let (shutdown, _) = tokio::sync::watch::channel(false);

        Self {
            claude_projects_dir,
            known_sessions: HashMap::new(),
            tx,
            shutdown,
        }
    }

    pub async fn start(&mut self) {
        // Initial scan
        self.scan_all().await;

        let claude_projects_dir = self.claude_projects_dir.clone();
        let tx = self.tx.clone();
        let mut shutdown_rx = self.shutdown.subscribe();

        // Keep track of known sessions in the scan loop
        let mut known_sessions: HashMap<String, DiscoveredSession> = self.known_sessions.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        debug!("SessionDiscovery shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        scan_all_inner(&claude_projects_dir, &mut known_sessions, &tx).await;
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        let _ = self.shutdown.send(true);
    }

    async fn scan_all(&mut self) {
        scan_all_inner(
            &self.claude_projects_dir,
            &mut self.known_sessions,
            &self.tx,
        )
        .await;
    }
}

async fn scan_all_inner(
    claude_projects_dir: &Path,
    known_sessions: &mut HashMap<String, DiscoveredSession>,
    tx: &mpsc::UnboundedSender<DiscoveryEvent>,
) {
    let projects_dir = match tokio::fs::read_dir(claude_projects_dir).await {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut projects_dir = projects_dir;
    let now = std::time::SystemTime::now();
    let twenty_four_hours = std::time::Duration::from_secs(24 * 60 * 60);

    while let Ok(Some(project_entry)) = projects_dir.next_entry().await {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        // Decode project path from directory name
        let dir_name = project_entry.file_name().to_string_lossy().to_string();
        let decoded_project_path = decode_project_path(&dir_name);
        let project_name = decoded_project_path
            .split('/')
            .last()
            .unwrap_or(&dir_name)
            .to_string();

        let mut session_dir = match tokio::fs::read_dir(&project_path).await {
            Ok(d) => d,
            Err(_) => continue,
        };

        while let Ok(Some(file_entry)) = session_dir.next_entry().await {
            let file_path = file_entry.path();
            let file_name = file_entry.file_name().to_string_lossy().to_string();

            if !file_name.ends_with(".jsonl") {
                continue;
            }

            // Check if modified within last 24 hours
            if let Ok(metadata) = file_entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > twenty_four_hours {
                            continue;
                        }
                    }
                }
            }

            let session_id = file_name.trim_end_matches(".jsonl").to_string();

            if known_sessions.contains_key(&session_id) {
                continue;
            }

            let discovered = DiscoveredSession {
                session_id: session_id.clone(),
                log_file: file_path,
                project_path: decoded_project_path.clone(),
                project_name: project_name.clone(),
            };

            info!(
                "Discovered session: {} ({})",
                session_id, project_name
            );

            known_sessions.insert(session_id, discovered.clone());
            let _ = tx.send(DiscoveryEvent::Found(discovered));
        }
    }
}

/// Decode an encoded project path from the directory name.
/// Claude Code encodes paths like `-Users-daiki-Projects-foo` → `/Users/daiki/Projects/foo`
fn decode_project_path(encoded: &str) -> String {
    if encoded.starts_with('-') {
        // Replace leading dash and internal dashes with /
        format!("/{}", encoded[1..].replace('-', "/"))
    } else {
        encoded.replace('-', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_path() {
        assert_eq!(
            decode_project_path("-Users-daiki-Projects-foo"),
            "/Users/daiki/Projects/foo"
        );
    }

    #[test]
    fn test_decode_project_path_no_leading_dash() {
        assert_eq!(
            decode_project_path("Users-daiki-Projects-foo"),
            "Users/daiki/Projects/foo"
        );
    }
}
