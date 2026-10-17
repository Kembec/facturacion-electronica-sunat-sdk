//! `sunat-sdk` binary: one command, JSON on stdin, JSON on stdout, errors as JSON on stderr. The
//! commands and their flags are in the library (`src/lib.rs`), shared with the C ABI.

use std::io::Read;
use std::process::ExitCode;

use serde_json::json;
use sunat_sdk_cli::{run, Io};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let io = Io { input: &read_stdin, env: &|key: &str| std::env::var(key).ok() };
    match run(&args, &io) {
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

fn read_stdin() -> Result<String, String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).map_err(|e| format!("reading stdin: {e}"))?;
    Ok(raw)
}
