//! Campaign slice experiment lists.

use anyhow::{bail, Result};

use crate::hardware::all_phase1_profiles;

pub fn slice_experiments(slice: &str) -> Result<Vec<String>> {
    match slice {
        "pls-substrate" => Ok(vec!["bm-pls0".into()]),
        "pls-connection" => Ok(vec!["bm-pls0".into(), "bm-pls1".into()]),
        "pls-hub" => Ok(vec![
            "bm-pls0".into(),
            "bm-pls5".into(),
            "bm-pls0-hub".into(),
            "bm-pls5-hub".into(),
        ]),
        "pls-client" => Ok(vec!["bm-pls2".into(), "bm-pls3".into()]),
        "pls-shape" => Ok(vec![
            "bm-pls4".into(),
            "bm-pls5".into(),
            "bm-pls6".into(),
            "bm-pls7".into(),
        ]),
        "pls-soak" => Ok(vec!["bm-pls8".into()]),
        "pls-fleet" => Ok(vec!["bm-pls9".into()]),
        "pls-hardware" => {
            let profiles = all_phase1_profiles()?;
            let mut exps = Vec::new();
            for profile in profiles {
                exps.push(format!("bm-pls0:{profile}"));
                exps.push(format!("bm-pls1:{profile}"));
            }
            Ok(exps)
        }
        other => bail!("unknown campaign slice: {other}"),
    }
}

pub fn report_path(
    reports_dir: &std::path::Path,
    experiment: &str,
    hardware: &str,
) -> std::path::PathBuf {
    reports_dir.join(format!("{experiment}-mem-embedded-{hardware}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pls_hub_slice_lists_modes_and_key_sweep() {
        let exps = slice_experiments("pls-hub").unwrap();
        assert!(exps.contains(&"bm-pls0".into()));
        assert!(exps.contains(&"bm-pls0-hub".into()));
        assert!(exps.contains(&"bm-pls5".into()));
        assert!(exps.contains(&"bm-pls5-hub".into()));
    }

    #[test]
    fn pls_shape_includes_pls5() {
        let exps = slice_experiments("pls-shape").unwrap();
        assert!(exps.contains(&"bm-pls5".into()));
    }
}
