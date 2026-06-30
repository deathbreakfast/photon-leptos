//! Automated knee / degradation detection.

use crate::stats::MetricStats;

#[derive(Debug, Clone, Copy)]
pub struct DegradationThresholds {
    pub p99_ws_delivery_ms: f64,
    pub error_rate: f64,
    pub connect_fail_rate: f64,
}

impl Default for DegradationThresholds {
    fn default() -> Self {
        Self {
            p99_ws_delivery_ms: 500.0,
            error_rate: 0.001,
            connect_fail_rate: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepVerdict {
    Pass,
    Fail,
}

pub fn evaluate_step(
    delivery: &MetricStats,
    error_rate: f64,
    connect_fail_rate: f64,
    thresholds: &DegradationThresholds,
) -> StepVerdict {
    if delivery.count == 0 {
        return StepVerdict::Fail;
    }
    if delivery.p99 > thresholds.p99_ws_delivery_ms {
        return StepVerdict::Fail;
    }
    if error_rate > thresholds.error_rate {
        return StepVerdict::Fail;
    }
    if connect_fail_rate > thresholds.connect_fail_rate {
        return StepVerdict::Fail;
    }
    StepVerdict::Pass
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_under_thresholds() {
        let delivery = MetricStats {
            p99: 100.0,
            count: 10,
            ..MetricStats::empty()
        };
        assert_eq!(
            evaluate_step(&delivery, 0.0, 0.0, &DegradationThresholds::default()),
            StepVerdict::Pass
        );
    }
}
