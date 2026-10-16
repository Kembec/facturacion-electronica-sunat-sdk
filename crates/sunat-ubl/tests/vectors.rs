//! Every fixture (`fixtures/*.json`: vouchers 01, 03, 07, 08, daily summaries `rc-` and void
//! communications `ra-`) rendered by `build` equals `tests/vectors/<case>.unsigned.xml` under
//! inclusive Canonical XML 1.0.

use std::fs;
use std::path::{Path, PathBuf};

use sunat_model::Document;

const EXPECTED_CASES: usize = 28;
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cases() -> Vec<String> {
    let mut cases: Vec<String> = fs::read_dir(repository().join("fixtures"))
        .expect("fixtures directory")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            name.strip_suffix(".json").map(str::to_owned)
        })
        .collect();
    cases.sort();
    cases
}

fn build(case: &str) -> Vec<u8> {
    let json = fs::read_to_string(repository().join("fixtures").join(format!("{case}.json"))).expect("fixture");
    let document = Document::from_json(&json).unwrap_or_else(|e| panic!("{case}: {e}"));
    sunat_ubl::build(&document).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn canonical(case: &str, xml: &[u8]) -> Vec<u8> {
    sunat_xmldsig::canonicalize(xml).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn first_difference(actual: &[u8], expected: &[u8]) -> String {
    let at = actual.iter().zip(expected).position(|(a, b)| a != b).unwrap_or(actual.len().min(expected.len()));
    let window = |bytes: &[u8]| {
        let start = at.saturating_sub(120);
        let end = (at + 120).min(bytes.len());
        String::from_utf8_lossy(&bytes[start..end]).into_owned()
    };
    format!("byte {at}\n  built:    …{}…\n  expected: …{}…", window(actual), window(expected))
}

#[test]
fn every_fixture_matches_its_vector_under_c14n() {
    let cases = cases();
    assert_eq!(cases.len(), EXPECTED_CASES, "{cases:?}");
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| {
            let vector = fs::read(repository().join("tests/vectors").join(format!("{case}.unsigned.xml")))
                .unwrap_or_else(|e| panic!("{case}: {e}"));
            let built = canonical(case, &build(case));
            let expected = canonical(case, &vector);
            (built != expected).then(|| format!("{case}: {}", first_difference(&built, &expected)))
        })
        .collect();
    assert!(failures.is_empty(), "{} of {} cases differ:\n{}", failures.len(), cases.len(), failures.join("\n"));
}

#[test]
fn output_after_the_declaration_is_already_canonical() {
    for case in cases() {
        let built = build(&case);
        let body = built.strip_prefix(DECLARATION).unwrap_or_else(|| panic!("{case}: declaration"));
        assert!(body == canonical(&case, &built).as_slice(), "{case}");
    }
}
