//! Synthetic WebSocket load generator.

mod ws_load;

pub use ws_load::{
    connect_many, default_target, finish_session, result_stats, run_paced_publish,
    run_sustained_load, spawn_connections, ConnectOptions, LoadGenResult, PublishOptions,
    ServerTarget,
};
