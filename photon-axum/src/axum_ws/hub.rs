//! Process-local broadcast hub for WebSocket fanout.
//!
//! One Photon `subscribe` + one JSON serialize per `(topic, key_filter)` group,
//! then fan-out of shared `Arc<str>` frames to per-socket bounded queues.
//! Distinct key filters remain separate groups — hub gains collapse as
//! cardinality approaches connection count.
//!
//! Group cleanup is generation-aware: a reader may remove only the group
//! instance it owns, so an obsolete reader cannot delete a replacement group
//! created under the same `(topic, key_filter)`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use photon_backend::instrumentation::log_ops;
use photon_runtime::Photon;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::task::JoinHandle;

/// Default per-socket queue depth before a slow client is disconnected.
pub const HUB_QUEUE_CAPACITY: usize = 64;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
struct GroupKey {
    topic: String,
    key_filter: Option<String>,
}

struct Member {
    id: u64,
    tx: mpsc::Sender<Arc<str>>,
}

struct Group {
    /// Monotonic instance id; readers capture this at spawn and remove only on match.
    generation: u64,
    members: Vec<Member>,
    reader: JoinHandle<()>,
}

struct HubInner {
    groups: HashMap<GroupKey, Group>,
}

/// Shared in-process registry: one subscribe pipeline per `(topic, key_filter)`.
pub struct WsBroadcastHub {
    inner: Mutex<HubInner>,
    next_id: AtomicU64,
    next_generation: AtomicU64,
}

impl std::fmt::Debug for WsBroadcastHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsBroadcastHub")
            .field("groups", &self.group_count())
            .finish()
    }
}

/// Live membership in a hub group; dropping leaves the group.
pub struct HubSubscription {
    /// Frames from the shared group reader (pre-serialized JSON text).
    pub rx: mpsc::Receiver<Arc<str>>,
    hub: Arc<WsBroadcastHub>,
    key: GroupKey,
    member_id: u64,
}

impl std::fmt::Debug for HubSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HubSubscription")
            .field("member_id", &self.member_id)
            .field("topic", &self.key.topic)
            .field("key_filter", &self.key.key_filter)
            .finish_non_exhaustive()
    }
}

impl Drop for HubSubscription {
    fn drop(&mut self) {
        self.hub.leave(&self.key, self.member_id);
    }
}

impl Default for WsBroadcastHub {
    fn default() -> Self {
        Self::new()
    }
}

impl WsBroadcastHub {
    /// Create an empty hub.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HubInner {
                groups: HashMap::new(),
            }),
            next_id: AtomicU64::new(1),
            next_generation: AtomicU64::new(1),
        }
    }

    /// Join (or create) the hub group for `(topic, key_filter)`.
    ///
    /// The first joiner starts a Photon subscribe reader for that group.
    #[must_use]
    pub fn join(
        self: &Arc<Self>,
        photon: Arc<Photon>,
        topic: String,
        key_filter: Option<String>,
    ) -> HubSubscription {
        let (tx, rx) = mpsc::channel(HUB_QUEUE_CAPACITY);
        let member_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let key = GroupKey {
            topic: topic.clone(),
            key_filter: key_filter.clone(),
        };

        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(group) = inner.groups.get_mut(&key) {
                group.members.push(Member { id: member_id, tx });
            } else {
                let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
                let hub = Arc::clone(self);
                let reader_key = key.clone();
                let reader_photon = Arc::clone(&photon);
                let reader = tokio::spawn(async move {
                    run_group_reader(hub, reader_photon, reader_key, generation).await;
                });
                inner.groups.insert(
                    key.clone(),
                    Group {
                        generation,
                        members: vec![Member { id: member_id, tx }],
                        reader,
                    },
                );
                log_ops(
                    "axum_ws_hub",
                    "group_start",
                    "broadcast hub group started",
                    &topic,
                    key_filter.as_deref().unwrap_or(""),
                    "",
                );
            }
        }

        HubSubscription {
            rx,
            hub: Arc::clone(self),
            key,
            member_id,
        }
    }

    fn leave(&self, key: &GroupKey, member_id: u64) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(group) = inner.groups.get_mut(key) else {
            return;
        };
        group.members.retain(|m| m.id != member_id);
        if group.members.is_empty() {
            if let Some(g) = inner.groups.remove(key) {
                g.reader.abort();
                log_ops(
                    "axum_ws_hub",
                    "group_stop",
                    "broadcast hub group stopped",
                    &key.topic,
                    key.key_filter.as_deref().unwrap_or(""),
                    "",
                );
            }
        }
    }

    /// Remove `key` only if the map still holds the same generation the reader owns.
    fn remove_if_generation(&self, key: &GroupKey, generation: u64) -> bool {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let matches = inner
            .groups
            .get(key)
            .is_some_and(|g| g.generation == generation);
        if matches {
            if let Some(g) = inner.groups.remove(key) {
                // Reader is exiting; drop the join handle without awaiting.
                drop(g.reader);
                log_ops(
                    "axum_ws_hub",
                    "group_stop",
                    "broadcast hub group stopped",
                    &key.topic,
                    key.key_filter.as_deref().unwrap_or(""),
                    "",
                );
            }
            true
        } else {
            false
        }
    }

    /// Number of active hub groups (test / diagnostics).
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .groups
            .len()
    }

    /// Member count for a group (test / diagnostics).
    #[must_use]
    pub fn member_count(&self, topic: &str, key_filter: Option<&str>) -> usize {
        let key = GroupKey {
            topic: topic.to_string(),
            key_filter: key_filter.map(str::to_string),
        };
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .groups
            .get(&key)
            .map(|g| g.members.len())
            .unwrap_or(0)
    }

    /// Generation of the current group for `(topic, key_filter)`, if any (diagnostics / tests).
    #[must_use]
    pub fn group_generation(&self, topic: &str, key_filter: Option<&str>) -> Option<u64> {
        let key = GroupKey {
            topic: topic.to_string(),
            key_filter: key_filter.map(str::to_string),
        };
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .groups
            .get(&key)
            .map(|g| g.generation)
    }

    /// Remove the group only if it still has `generation` (diagnostics / tests).
    ///
    /// Used to verify obsolete readers cannot delete a replacement group.
    #[must_use]
    pub fn try_remove_generation(
        &self,
        topic: &str,
        key_filter: Option<&str>,
        generation: u64,
    ) -> bool {
        let key = GroupKey {
            topic: topic.to_string(),
            key_filter: key_filter.map(str::to_string),
        };
        self.remove_if_generation(&key, generation)
    }
}

async fn run_group_reader(
    hub: Arc<WsBroadcastHub>,
    photon: Arc<Photon>,
    key: GroupKey,
    generation: u64,
) {
    let mut stream = photon.subscribe(&key.topic, key.key_filter.as_deref(), None);

    while let Some(ev) = stream.next().await {
        match ev {
            Ok(event) => {
                let json = match serde_json::to_string(&event) {
                    Ok(s) => Arc::<str>::from(s),
                    Err(e) => {
                        log_ops(
                            "axum_ws_hub",
                            "serialize_error",
                            "failed to serialize event for hub",
                            &key.topic,
                            "",
                            &e.to_string(),
                        );
                        continue;
                    }
                };

                // Snapshot senders under the registry lock, fan out outside it.
                let senders = {
                    let inner = hub.inner.lock().unwrap_or_else(|e| e.into_inner());
                    let Some(group) = inner.groups.get(&key) else {
                        return;
                    };
                    if group.generation != generation {
                        return;
                    }
                    group
                        .members
                        .iter()
                        .map(|m| (m.id, m.tx.clone()))
                        .collect::<Vec<_>>()
                };

                let mut drop_ids = Vec::new();
                for (id, tx) in &senders {
                    match tx.try_send(Arc::clone(&json)) {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {
                            log_ops(
                                "axum_ws_hub",
                                "slow_client",
                                "disconnecting slow WS client (queue full)",
                                &key.topic,
                                key.key_filter.as_deref().unwrap_or(""),
                                &id.to_string(),
                            );
                            drop_ids.push(*id);
                        }
                        Err(TrySendError::Closed(_)) => {
                            drop_ids.push(*id);
                        }
                    }
                }

                if !drop_ids.is_empty() {
                    let empty = {
                        let mut inner = hub.inner.lock().unwrap_or_else(|e| e.into_inner());
                        let Some(group) = inner.groups.get_mut(&key) else {
                            return;
                        };
                        if group.generation != generation {
                            return;
                        }
                        group.members.retain(|m| !drop_ids.contains(&m.id));
                        group.members.is_empty()
                    };
                    if empty {
                        hub.remove_if_generation(&key, generation);
                        return;
                    }
                }
            }
            Err(e) => {
                log_ops(
                    "axum_ws_hub",
                    "subscription_error",
                    "photon subscription error in hub",
                    &key.topic,
                    "",
                    &e.to_string(),
                );
                break;
            }
        }
    }

    // Stream ended or subscription error — remove only our generation.
    hub.remove_if_generation(&key, generation);
}
