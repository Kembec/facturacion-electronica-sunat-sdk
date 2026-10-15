//! `sunat-sdk` binary: one command, JSON on stdin, JSON on stdout, errors as JSON on stderr.
//!
//! Commands: `build`, `sign`, `zip`, `cdr` (offline); `send`, `status` (network, HTTPS with
//! verification). `sign` reads `--cert`/SUNAT_CERT_PATH and `--key`/SUNAT_KEY_PATH (PEM files).
//! `send` reads `--endpoint` or SUNAT_ENDPOINT, `status` reads `--endpoint` or
//! SUNAT_STATUS_ENDPOINT; both read `--user`/SUNAT_SOL_USER (RUC + SOL user) and
//! `--password`/SUNAT_SOL_PASSWORD.

mod commands;
mod config;
mod document;
mod http;

use std::io::Read;
use std::process::ExitCode;

use serde_json::{json, Value};

const USAGE: &str = "usage: sunat-sdk <build|sign|zip|send|status|cdr> [--cert PEM --key PEM] [--endpoint URL] [--user RUCUSER] [--password PASS] < input.json";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{}", json!({ "error": message }));
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<Value, String> {
    let (command, rest) = args.split_first().ok_or(USAGE)?;
    let env = |key: &str| std::env::var(key).ok();
    let no_flags = |rest: &[String]| -> Result<(), String> {
        if rest.is_empty() {
            return Ok(());
        }
        Err(format!("`{command}` takes no flags"))
    };
    let offline = |rest: &[String]| -> Result<Value, String> {
        no_flags(rest)?;
        read_input()
    };
    match command.as_str() {
        "build" => {
            no_flags(rest)?;
            document::build(&read_raw()?)
        }
        "sign" => {
            let files = config::key_files(rest, &env)?;
            let cert = read_pem(&files.cert, "--cert")?;
            let key = read_pem(&files.key, "--key")?;
            document::sign(&read_input()?, &cert, &key)
        }
        "zip" => commands::zip(&offline(rest)?),
        "cdr" => commands::cdr(&offline(rest)?),
        "send" => {
            let conn = config::connection(rest, config::ENV_ENDPOINT, &env)?;
            commands::send(&read_input()?, &conn, &http::post_soap)
        }
        "status" => {
            let conn = config::connection(rest, config::ENV_STATUS_ENDPOINT, &env)?;
            commands::status(&read_input()?, &conn, &http::post_soap)
        }
        _ => Err(USAGE.into()),
    }
}

fn read_raw() -> Result<String, String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).map_err(|e| format!("reading stdin: {e}"))?;
    Ok(raw)
}

fn read_input() -> Result<Value, String> {
    serde_json::from_str(&read_raw()?).map_err(|e| format!("stdin is not JSON: {e}"))
}

/// Reads a PEM file; the error names the flag and the I/O cause, never the content.
fn read_pem(path: &str, flag: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("reading {flag} file: {}", e.kind()))
}
