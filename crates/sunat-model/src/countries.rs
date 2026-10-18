//! Catalog 04 (country code). SUNAT points to ISO 3166-1 without listing it; `data/catalog-04.json`
//! holds the list the SDK accepts and where it comes from: ISO 3166-1 alpha-2 in force plus the two
//! withdrawn codes of the page the SUNAT sheet cites (`AN`, `CS`).

use std::collections::HashSet;
use std::sync::OnceLock;

const CATALOG_04: &str = include_str!("../data/catalog-04.json");

#[derive(serde::Deserialize)]
struct Catalog {
    codes: Vec<String>,
}

fn codes() -> &'static HashSet<String> {
    static CODES: OnceLock<HashSet<String>> = OnceLock::new();
    CODES.get_or_init(|| {
        let catalog: Catalog = serde_json::from_str(CATALOG_04).expect("data/catalog-04.json holds a codes array");
        catalog.codes.into_iter().collect()
    })
}

/// Whether `code` is in catalog 04 (`PE` included: each rule that excludes it says so).
pub(crate) fn is_country(code: &str) -> bool {
    codes().contains(code)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_catalog_is_iso_plus_the_two_withdrawn_codes() {
        assert_eq!(super::codes().len(), 251);
        for code in ["PE", "RS", "ME", "SS", "CW", "AN", "CS"] {
            assert!(super::is_country(code), "{code}");
        }
        for code in ["ZZ", "XK", "pe", "PER", ""] {
            assert!(!super::is_country(code), "{code}");
        }
    }
}
