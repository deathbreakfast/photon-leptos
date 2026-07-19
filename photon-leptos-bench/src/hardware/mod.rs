//! AWS hardware profile registry (Phase 1 small–medium + Phase 2 stubs).

mod profiles;

pub use profiles::{
    all_phase1_profiles, load_profiles, validate_hardware, HardwareProfile, ProfilesFile,
};
