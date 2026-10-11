//! The last connection probe of each source, reused while recent.

use chrono::{DateTime, Duration, Utc};

/// A probe result is reused for this long, up or down.
pub(crate) const PROBE_REUSE: Duration = Duration::minutes(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Probe {
    Lake,
    HomeAssistant,
    Forecast,
}

#[derive(Debug, Default)]
pub(crate) struct ProbeCache {
    results: Vec<(Probe, bool, DateTime<Utc>)>,
}

impl ProbeCache {
    /// The last result for `probe` while it is recent enough to reuse.
    // @spec SYNC-CONN-006
    pub fn recent(&self, probe: Probe, now: DateTime<Utc>) -> Option<bool> {
        self.results
            .iter()
            .find(|(p, _, at)| *p == probe && now - *at < PROBE_REUSE)
            .map(|(_, up, _)| *up)
    }

    pub fn record(&mut self, probe: Probe, up: bool, at: DateTime<Utc>) {
        self.results.retain(|(p, _, _)| *p != probe);
        self.results.push((probe, up, at));
    }
}
