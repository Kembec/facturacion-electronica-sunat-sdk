//! `sunat-sdk` binary: one command, JSON on stdin, JSON on stdout, errors as JSON on stderr.
//!
//! Commands: `zip`, `cdr` (offline); `send`, `status` (network, HTTPS with verification).
//! `send` reads `--endpoint` or SUNAT_ENDPOINT, `status` reads `--endpoint` or
//! SUNAT_STATUS_ENDPOINT; both read `--user`/SUNAT_SOL_USER (RUC + SOL user) and
//! `--password`/SUNAT_SOL_PASSWORD.

mod commands;
mod config;
mod http;

use std::io::Read;
use std::process::ExitCode;

use serde_json::{json, Value};

const USAGE: &str = "usage: sunat-sdk <zip|send|status|cdr> [--endpoint URL] [--user RUCUSER] [--password PASS] < input.json";

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
    let offline = |rest: &[String]| -> Result<Value, String> {
        if !rest.is_empty() {
            return Err(format!("`{command}` takes no flags"));
        }
        read_input()
    };
    match command.as_str() {
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

fn read_input() -> Result<Value, String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).map_err(|e| format!("reading stdin: {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("stdin is not JSON: {e}"))
}
