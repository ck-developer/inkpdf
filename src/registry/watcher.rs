//! Surveillance du volume : événements `notify` (anti-rebond de 500 ms), rescan périodique de
//! secours (volumes réseau, ConfigMaps) et observation de confirmation 1 s après un changement.

use std::sync::Arc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};

use super::Registry;

/// Délai de regroupement des événements du système de fichiers.
const DEBOUNCE: Duration = Duration::from_millis(500);
/// Délai de l'observation qui confirme la stabilité d'un changement.
const CONFIRMATION_DELAY: Duration = Duration::from_secs(1);

/// Garde la surveillance active ; l'arrête quand elle est abandonnée.
pub struct WatcherHandle {
    _watcher: Option<RecommendedWatcher>,
    task: JoinHandle<()>,
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Démarre la surveillance du volume du registre. Doit être appelé dans un runtime tokio.
pub fn spawn(registry: Arc<Registry>, rescan_interval: Duration) -> WatcherHandle {
    let (events_tx, events_rx) = mpsc::unbounded_channel::<()>();
    let watcher = start_watcher(&registry, events_tx);
    let task = tokio::spawn(run(registry, rescan_interval, events_rx));
    WatcherHandle {
        _watcher: watcher,
        task,
    }
}

fn start_watcher(
    registry: &Registry,
    events: mpsc::UnboundedSender<()>,
) -> Option<RecommendedWatcher> {
    let dir = registry.templates_dir();
    let result = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.is_ok() {
            let _ = events.send(());
        }
    })
    .and_then(|mut watcher| {
        watcher.watch(dir, RecursiveMode::Recursive)?;
        Ok(watcher)
    });
    match result {
        Ok(watcher) => Some(watcher),
        Err(e) => {
            tracing::warn!(
                event = "registry.watcher_unavailable",
                dir = %dir.display(),
                error = %e,
                "file watching unavailable, relying on periodic rescan"
            );
            None
        }
    }
}

async fn run(
    registry: Arc<Registry>,
    rescan_interval: Duration,
    events: mpsc::UnboundedReceiver<()>,
) {
    let mut events = Some(events);
    let mut rescan = tokio::time::interval_at(Instant::now() + rescan_interval, rescan_interval);
    rescan.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut confirm_at: Option<Instant> = None;

    loop {
        let confirmation = async {
            match confirm_at {
                Some(at) => tokio::time::sleep_until(at).await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            event = next_event(&mut events) => {
                if event.is_none() {
                    // Plus d'émetteur (watcher indisponible) : rescan seul.
                    events = None;
                    continue;
                }
                tokio::time::sleep(DEBOUNCE).await;
                if let Some(events) = events.as_mut() {
                    while events.try_recv().is_ok() {}
                }
            }
            _ = rescan.tick() => {}
            () = confirmation => {}
        }

        let registry = registry.clone();
        let outcome = tokio::task::spawn_blocking(move || registry.refresh()).await;
        confirm_at = match outcome {
            Ok(outcome) if outcome.needs_confirmation => Some(Instant::now() + CONFIRMATION_DELAY),
            Ok(_) => None,
            Err(e) => {
                tracing::error!(event = "registry.refresh_failed", error = %e);
                None
            }
        };
    }
}

async fn next_event(events: &mut Option<mpsc::UnboundedReceiver<()>>) -> Option<()> {
    match events {
        Some(events) => events.recv().await,
        None => std::future::pending().await,
    }
}
