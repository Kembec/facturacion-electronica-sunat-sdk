//! The dispatch of the `sunat-sdk` commands (docs/cli.md), shared by the binary and the C ABI
//! (`bindings/ffi`): both run the same flag parsing, validation, network calls and outcomes, and
//! differ only in where the input and the environment come from ([`Io`]).
//!
//! Commands: `build`, `sign`, `zip`, `cdr` (offline); `send`, `summary`, `ticket`, `status`,
//! `validity`, `gre-send`, `gre-ticket` (network, HTTPS with verification). `sign` reads
//! `--cert`/SUNAT_CERT_PATH and `--key`/SUNAT_KEY_PATH (PEM files). `send`, `summary` and `ticket`
//! read `--endpoint` or SUNAT_ENDPOINT, `status` reads `--endpoint` or SUNAT_STATUS_ENDPOINT; both
//! read `--user`/SUNAT_SOL_USER (RUC + SOL user) and `--password`/SUNAT_SOL_PASSWORD, and
//! `--timeout`/SUNAT_HTTP_TIMEOUT_SECONDS (whole seconds, 1 to 300, default 90) for the HTTP wait.
//! `validity` reads `--client-id`/SUNAT_API_CLIENT_ID and `--client-secret`/SUNAT_API_CLIENT_SECRET
//! (Credenciales de API SUNAT) and the same `--timeout` for its two calls together. `gre-send` and
//! `gre-ticket` (despatch advice, REST) read the same client pair, the SOL `--user` and
//! `--password`, and `--token-endpoint`/SUNAT_GRE_TOKEN_ENDPOINT and `--endpoint`/SUNAT_GRE_ENDPOINT,
//! both required.

mod commands;
mod config;
mod document;
mod gre;
mod http;
mod validity;

use serde_json::Value;

pub const USAGE: &str = "usage: sunat-sdk <build|sign|zip|send|summary|ticket|status|validity|gre-send|gre-ticket|cdr> [--cert PEM --key PEM] [--endpoint URL] [--user RUCUSER] [--password PASS] [--client-id ID --client-secret SECRET] [--token-endpoint URL] [--timeout SECONDS] < input.json";

/// What a command takes from outside its flags. The binary reads stdin and the process
/// environment; the C ABI passes the request's `input` and no environment.
pub struct Io<'a> {
    /// The command's input as JSON text (the binary's stdin). Read once, and only after the
    /// flags pass: a command that fails on its flags never consumes it.
    pub input: &'a dyn Fn() -> Result<String, String>,
    /// The value of an environment variable, consulted only when its flag is absent.
    pub env: &'a dyn Fn(&str) -> Option<String>,
}

/// Runs `args` (`[command, flags...]`) and returns the JSON the binary prints on stdout, or the
/// message it prints as `{"error"}` on stderr with exit code 1.
pub fn run(args: &[String], io: &Io) -> Result<Value, String> {
    let (command, rest) = args.split_first().ok_or(USAGE)?;
    let env = io.env;
    let read_input = || -> Result<Value, String> {
        serde_json::from_str(&(io.input)()?).map_err(|e| format!("stdin is not JSON: {e}"))
    };
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
            document::build(&(io.input)()?)
        }
        "sign" => {
            let files = config::key_files(rest, env)?;
            let cert = read_pem(&files.cert, "--cert")?;
            let key = read_pem(&files.key, "--key")?;
            document::sign(&read_input()?, &cert, &key)
        }
        "zip" => commands::zip(&offline(rest)?),
        "cdr" => commands::cdr(&offline(rest)?),
        "send" => {
            let conn = config::connection(rest, config::ENV_ENDPOINT, env)?;
            let post = |endpoint: &str, action: &str, envelope: &str| http::post_soap(endpoint, action, envelope, conn.timeout);
            commands::send(&read_input()?, &conn, &post)
        }
        "summary" | "ticket" => {
            let conn = config::connection(rest, config::ENV_ENDPOINT, env)?;
            let post = |endpoint: &str, action: &str, envelope: &str| http::post_soap(endpoint, action, envelope, conn.timeout);
            let input = read_input()?;
            if command == "summary" { commands::summary(&input, &conn, &post) } else { commands::ticket(&input, &conn, &post) }
        }
        "status" => {
            let conn = config::connection(rest, config::ENV_STATUS_ENDPOINT, env)?;
            let post = |endpoint: &str, action: &str, envelope: &str| http::post_soap(endpoint, action, envelope, conn.timeout);
            commands::status(&read_input()?, &conn, &post)
        }
        "validity" => {
            let conn = config::api_connection(rest, env)?;
            let post = |request: &Value, timeout: std::time::Duration| http::post_rest(request, timeout);
            validity::validity(&read_input()?, &conn, &post)
        }
        "gre-send" | "gre-ticket" => {
            let conn = config::gre_connection(rest, env)?;
            let post = |request: &Value, timeout: std::time::Duration| http::post_rest(request, timeout);
            let input = read_input()?;
            if command == "gre-send" { gre::send(&input, &conn, &post) } else { gre::ticket(&input, &conn, &post) }
        }
        _ => Err(USAGE.into()),
    }
}

/// Reads a PEM file; the error names the flag and the I/O cause, never the content.
fn read_pem(path: &str, flag: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("reading {flag} file: {}", e.kind()))
}
