//! Zip packaging as SUNAT expects it: one `<name>.xml` inside `<name>.zip`.

use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::TransportError;

/// Zip file name SUNAT expects for a document name (`RUC-TIPO-SERIE-NUMERO.zip`).
pub fn zip_file_name(name: &str) -> String {
    format!("{name}.zip")
}

/// Zips `xml` as `<name>.xml`. `name` must be `RUC-TIPO-SERIE-NUMERO`
/// (RUC of 11 digits, type of 2 characters, series of 4, number of 1 to 8 digits).
/// The entry carries the zip epoch timestamp, so the same input gives the same bytes.
pub fn zip_document(name: &str, xml: &[u8]) -> Result<Vec<u8>, TransportError> {
    validate_name(name)?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let zip_err = |e: zip::result::ZipError| TransportError::Zip(e.to_string());
    writer.start_file(format!("{name}.xml"), options).map_err(zip_err)?;
    writer.write_all(xml).map_err(|e| TransportError::Zip(e.to_string()))?;
    let cursor = writer.finish().map_err(zip_err)?;
    Ok(cursor.into_inner())
}

/// Returns the first `.xml` entry of a zip, skipping folders and other files
/// (SUNAT CDR zips may carry a `dummy` entry).
pub fn unzip_xml(zip_bytes: &[u8]) -> Result<Vec<u8>, TransportError> {
    let zip_err = |e: zip::result::ZipError| TransportError::Zip(e.to_string());
    let mut archive = ZipArchive::new(Cursor::new(zip_bytes)).map_err(zip_err)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_err)?;
        if entry.is_dir() || !entry.name().to_ascii_lowercase().ends_with(".xml") {
            continue;
        }
        let mut content = Vec::new();
        entry.read_to_end(&mut content).map_err(|e| TransportError::Zip(e.to_string()))?;
        return Ok(content);
    }
    Err(TransportError::NoXmlInZip)
}

fn validate_name(name: &str) -> Result<(), TransportError> {
    let parts: Vec<&str> = name.split('-').collect();
    let valid = parts.len() == 4
        && is_digits(parts[0], 11, 11)
        && is_alnum(parts[1], 2, 2)
        && is_alnum(parts[2], 4, 4)
        && is_digits(parts[3], 1, 8);
    if valid {
        return Ok(());
    }
    Err(TransportError::InvalidName(name.to_string()))
}

pub(crate) fn is_digits(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_digit())
}

pub(crate) fn is_alnum(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_alphanumeric())
}
