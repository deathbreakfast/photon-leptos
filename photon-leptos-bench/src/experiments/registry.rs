//! Pre-registered BM-PLS* experiment metadata.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExperimentStatus {
    Ready,
    Deferred,
    Fleet,
}

#[derive(Debug, Clone, Copy)]
pub struct ExperimentMeta {
    pub id: &'static str,
    pub status: ExperimentStatus,
    pub summary: &'static str,
}

pub const REGISTRY: &[ExperimentMeta] = &[
    meta(
        "bm-pls0",
        ExperimentStatus::Ready,
        "WS subscriber connection sweep",
    ),
    meta(
        "bm-pls0-hub",
        ExperimentStatus::Ready,
        "PLS0 with broadcast_hub fanout (A/B vs per_subscribe)",
    ),
    meta(
        "bm-pls1",
        ExperimentStatus::Ready,
        "publish rate × N matrix",
    ),
    meta(
        "bm-pls2",
        ExperimentStatus::Ready,
        "multi-connection client sweep",
    ),
    meta(
        "bm-pls3",
        ExperimentStatus::Ready,
        "refetch vs replace latency",
    ),
    meta(
        "bm-pls4",
        ExperimentStatus::Ready,
        "payload scaling over WS",
    ),
    meta(
        "bm-pls5",
        ExperimentStatus::Ready,
        "N clients × G key-filter working-set sweep",
    ),
    meta(
        "bm-pls5-hub",
        ExperimentStatus::Ready,
        "PLS5 with broadcast_hub fanout",
    ),
    meta(
        "bm-pls6",
        ExperimentStatus::Ready,
        "keyed vs broadcast subscriptions",
    ),
    meta(
        "bm-pls7",
        ExperimentStatus::Ready,
        "reconnect storm recovery",
    ),
    meta("bm-pls8", ExperimentStatus::Ready, "1h soak at 80% knee"),
    meta(
        "bm-pls9",
        ExperimentStatus::Fleet,
        "ALB horizontal smoke (multi-server URLs)",
    ),
];

const fn meta(id: &'static str, status: ExperimentStatus, summary: &'static str) -> ExperimentMeta {
    ExperimentMeta {
        id,
        status,
        summary,
    }
}

pub fn status_label(status: ExperimentStatus) -> &'static str {
    match status {
        ExperimentStatus::Ready => "ready",
        ExperimentStatus::Deferred => "deferred",
        ExperimentStatus::Fleet => "fleet",
    }
}

pub fn find(id: &str) -> Option<&'static ExperimentMeta> {
    let key = id.to_ascii_lowercase();
    REGISTRY.iter().find(|m| m.id == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_includes_hub_and_key_working_set() {
        assert!(find("bm-pls0-hub").is_some());
        assert!(find("bm-pls5").is_some());
        assert_eq!(find("bm-pls0-hub").unwrap().status, ExperimentStatus::Ready);
    }
}
