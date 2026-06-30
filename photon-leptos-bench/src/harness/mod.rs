//! Hardware introspection for report appendices.

mod capture;

pub use capture::HardwareDetail;

pub fn capture_hardware() -> anyhow::Result<HardwareDetail> {
    capture::capture()
}
