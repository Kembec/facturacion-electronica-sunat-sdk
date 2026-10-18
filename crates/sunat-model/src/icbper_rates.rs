//! ICBPER per bag by date of issue: versioned data of the SDK, not a loose constant. The schedule is
//! Ley 30884, art. 12 (S/ 0.10 in 2019 rising S/ 0.10 a year to S/ 0.50 from 2023 «y años
//! subsiguientes»), and SUNAT's own table in catalog 05, row 7152, of `estructuras_sfs` (June 2026:
//! «Desde 2023 0.5»). A change of the law is a new row and a new `ICBPER_RATES_VERSION`.

/// The day this table was last checked against its sources (`YYYY-MM-DD`).
pub(crate) const ICBPER_RATES_VERSION: &str = "2026-10-17";

/// One period of the schedule: `from` and `to` inclusive, `to` open when `None`.
pub(crate) struct IcbperRate {
    pub(crate) from: &'static str,
    pub(crate) to: Option<&'static str>,
    /// PEN per bag, as SUNAT's `PerUnitAmount`.
    pub(crate) value: &'static str,
    /// `verified`, or `verified-partial` while no later rule changing art. 12 has been ruled out.
    pub(crate) state: &'static str,
}

/// In force from 01/08/2019 (art. 12.10): before it the tax does not exist (SUNAT 2949).
pub(crate) const ICBPER_RATES: &[IcbperRate] = &[
    IcbperRate { from: "2019-08-01", to: Some("2019-12-31"), value: "0.10", state: "verified" },
    IcbperRate { from: "2020-01-01", to: Some("2020-12-31"), value: "0.20", state: "verified" },
    IcbperRate { from: "2021-01-01", to: Some("2021-12-31"), value: "0.30", state: "verified" },
    IcbperRate { from: "2022-01-01", to: Some("2022-12-31"), value: "0.40", state: "verified" },
    IcbperRate { from: "2023-01-01", to: None, value: "0.50", state: "verified-partial" },
];

/// The rate for a Lima date `YYYY-MM-DD`; `None` before the tax came into force.
pub(crate) fn rate_on(date: &str) -> Option<&'static IcbperRate> {
    ICBPER_RATES.iter().find(|rate| rate.from <= date && rate.to.map_or(true, |to| date <= to))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schedule_covers_every_day_from_its_start_without_gaps() {
        assert!(rate_on("2019-07-31").is_none());
        assert_eq!(rate_on("2019-08-01").map(|r| r.value), Some("0.10"));
        assert_eq!(rate_on("2022-12-31").map(|r| r.value), Some("0.40"));
        assert_eq!(rate_on("2023-01-01").map(|r| r.value), Some("0.50"));
        assert_eq!(rate_on("2026-12-31").map(|r| r.value), Some("0.50"));
        assert!(ICBPER_RATES.iter().all(|r| matches!(r.state, "verified" | "verified-partial")));
        assert_eq!(ICBPER_RATES_VERSION.len(), 10);
    }
}
