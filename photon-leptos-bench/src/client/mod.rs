//! Synthetic WebSocket load generator.

mod ws_load;

pub use ws_load::{
    connect_many, default_target, ensure_ws_mode, finish_session, result_stats, run_keyed_load,
    run_paced_publish, run_sustained_load, spawn_connections, spawn_connections_key_groups,
    wait_for_health, ConnectOptions, LoadGenResult, PublishOptions, PublishStats, ServerTarget,
};
