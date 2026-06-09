use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(()),
    Resize((), ()),
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
        children: Vec<crate::config::BackupSource>,
    },
    BackupProgress(crate::restic::ProgressEvent),
    SnapshotsLoaded(Vec<crate::restic::Snapshot>),
    StatsLoaded(crate::restic::ResticStats),
    StatsFailed,
    SchedulerStatus { active: bool, next_time: Option<String> },
    PruneComplete(String),
    ForgetComplete { kept: usize, removed: usize },
    RestoreComplete(String),
    Error(String),
    OperationComplete(String),
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    shutdown_tx: watch::Sender<bool>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

        tokio::spawn(async move {
            let tick_duration = Duration::from_millis(tick_rate_ms);
            loop {
                if *shutdown_rx.borrow() {
                    break;
                }
                if event::poll(tick_duration).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            let _ = tx_clone.send(Event::Key(key));
                        }
                        Ok(CrosstermEvent::Mouse(_mouse)) => {
                            let _ = tx_clone.send(Event::Mouse(()));
                        }
                        Ok(CrosstermEvent::Resize(_w, _h)) => {
                            let _ = tx_clone.send(Event::Resize((), ()));
                        }
                        _ => {}
                    }
                } else {
                    if shutdown_rx.has_changed().unwrap_or(false) {
                        let _ = shutdown_rx.changed().await;
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    let _ = tx_clone.send(Event::Tick);
                }
            }
        });

        Self { tx, rx, shutdown_tx }
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }
}
