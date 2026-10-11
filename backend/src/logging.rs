//! The log filter from `RUST_LOG`.

use tracing_subscriber::EnvFilter;

/// The filter `raw` describes, or `info` with a warning naming `RUST_LOG` when it can't
/// be parsed.
// @spec DEPLOY-LOG-001, DEPLOY-CONFIG-012
pub fn filter(raw: Option<&str>) -> (EnvFilter, Option<String>) {
    match raw.map(str::trim).filter(|r| !r.is_empty()) {
        None => (EnvFilter::new("info"), None),
        Some(raw) => match EnvFilter::try_new(raw) {
            Ok(filter) => (filter, None),
            Err(_) => (
                EnvFilter::new("info"),
                Some(format!(
                    "RUST_LOG={raw:?} isn't a valid filter; using info."
                )),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @spec DEPLOY-CONFIG-012, DEPLOY-LOG-001
    #[test]
    fn an_unparseable_rust_log_falls_back_to_info_with_a_warning() {
        let (_, warning) = filter(Some("turfops=loudly"));
        let warning = warning.expect("a warning");
        assert!(
            warning.contains("RUST_LOG") && warning.contains("info"),
            "{warning}"
        );
        assert!(filter(Some("turfops_backend=debug,info")).1.is_none());
        assert!(filter(None).1.is_none());
    }
}
