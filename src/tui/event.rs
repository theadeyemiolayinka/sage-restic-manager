use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    BackgroundTask(BackgroundEvent),
}

#[derive(Debug, Clone)]
pub enum BackgroundEvent {
    DockerDiscoveryFound {
        container: crate::config::BackupSource,
        children: Vec<crate::config::BackupSource>,
    },
    DockerPathMissing {
        searched: std::path::PathBuf,
    },
    ContainerChildrenScanned {
        container_path: std::path::PathBuf,
        children: Vec<crate::config::BackupSource>,
    },
    BackupProgress(crate::restic::ProgressEvent),
    SnapshotsLoaded(Vec<crate::restic::Snapshot>),
    StatsLoaded(crate::restic::ResticStats),
    Error(String),
    OperationComplete(String),
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let tick_duration = Duration::from_millis(tick_rate_ms);
            loop {
                if event::poll(tick_duration).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            let _ = tx_clone.send(Event::Key(key));
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            let _ = tx_clone.send(Event::Mouse(mouse));
                        }
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            let _ = tx_clone.send(Event::Resize(w, h));
                        }
                        _ => {}
                    }
                } else {
                    let _ = tx_clone.send(Event::Tick);
                }
            }
        });

        Self { tx, rx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }
}
