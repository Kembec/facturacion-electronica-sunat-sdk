//! The request of `sunat_sdk_call`: `{"command", "flags"?, "input"?}`, the same command, flags and
//! stdin as one run of the CLI (docs/c-abi.md). Running it goes through the CLI's own dispatch.

use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::{json, Map, Value};
use sunat_sdk_cli::{run, Io};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request<'a> {
    command: String,
    #[serde(default)]
    flags: Map<String, Value>,
    #[serde(borrow, default)]
    input: Option<&'a RawValue>,
}

/// Runs one request and returns `(status, response JSON)`: `0` with the CLI's stdout, or `1` with
/// the CLI's `{"error"}`.
pub fn handle(request: &str) -> (i32, String) {
    match execute(request) {
        Ok(output) => (0, output.to_string()),
        Err(message) => (1, error(&message)),
    }
}

pub fn error(message: &str) -> String {
    json!({ "error": message }).to_string()
}

fn execute(raw: &str) -> Result<Value, String> {
    let request: Request = serde_json::from_str(raw).map_err(|e| format!("request is not valid: {e}"))?;
    let mut args = Vec::with_capacity(request.flags.len() + 1);
    args.push(request.command);
    for (name, value) in &request.flags {
        args.push(flag(name, value)?);
    }
    let input = request.input.map(RawValue::get);
    let read = || input.map(str::to_owned).ok_or_else(|| "missing `input`".to_string());
    // The library never reads the process environment: a host process (PHP-FPM, IIS, a VB6 app)
    // shares it with everything it loads, so every value comes in the request.
    let io = Io { input: &read, env: &|_: &str| None };
    run(&args, &io)
}

/// `"timeout": 30` → `--timeout=30`. The `=` form keeps a value that starts with `--` a value; the
/// CLI's parser then rejects unknown flags exactly as it does on the command line.
fn flag(name: &str, value: &Value) -> Result<String, String> {
    let valid_name = name.starts_with(|c: char| c.is_ascii_lowercase())
        && name.chars().all(|c| c.is_ascii_lowercase() || c == '-');
    if !valid_name {
        return Err(format!("unknown flag --{name}"));
    }
    match value {
        Value::String(text) => Ok(format!("--{name}={text}")),
        Value::Number(number) if number.is_u64() => Ok(format!("--{name}={number}")),
        _ => Err(format!("flag `{name}` must be a string or a whole number")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(response: &str) -> String {
        let value: Value = serde_json::from_str(response).unwrap();
        value["error"].as_str().unwrap().to_string()
    }

    #[test]
    fn flags_become_cli_flags_and_go_through_its_parser() {
        assert_eq!(flag("timeout", &json!(30)).unwrap(), "--timeout=30");
        assert_eq!(flag("endpoint", &json!("--x")).unwrap(), "--endpoint=--x");
        assert!(flag("Cert", &json!("a")).is_err());
        assert!(flag("cert=x", &json!("a")).is_err());
        assert!(flag("timeout", &json!(-1)).is_err());
        assert!(flag("timeout", &json!(1.5)).is_err());
        let (status, response) = handle(r#"{"command":"send","flags":{"cert":"x"},"input":{}}"#);
        assert_eq!((status, message(&response)), (1, "unknown flag --cert".to_string()));
    }

    #[test]
    fn malformed_requests_are_errors_with_the_cli_shape() {
        for raw in ["", "[]", r#"{"input":{}}"#, r#"{"command":"build","flag":{}}"#] {
            let (status, response) = handle(raw);
            assert_eq!(status, 1, "{raw}");
            assert!(message(&response).starts_with("request is not valid"), "{raw}: {response}");
        }
        let (status, response) = handle(r#"{"command":"zip"}"#);
        assert_eq!((status, message(&response)), (1, "missing `input`".to_string()));
        let (status, response) = handle(r#"{"command":"nope","input":{}}"#);
        assert_eq!((status, message(&response)), (1, sunat_sdk_cli::USAGE.to_string()));
    }

    #[test]
    fn the_environment_is_never_read() {
        // A flag absent from the request stays absent even if the process defines its variable.
        // No other test of this crate reads these two variables.
        std::env::set_var("SUNAT_CERT_PATH", "/env/cert.pem");
        std::env::set_var("SUNAT_KEY_PATH", "/env/key.pem");
        let (status, response) = handle(r#"{"command":"sign","input":{"xml":"<a/>"}}"#);
        assert_eq!((status, message(&response)), (1, "missing --cert or SUNAT_CERT_PATH".to_string()));
    }
}
