//! Endpoint and SOL credentials, from flags or environment. Nothing here is ever printed.

use std::fmt;
use std::time::Duration;

pub const ENV_ENDPOINT: &str = "SUNAT_ENDPOINT";
pub const ENV_STATUS_ENDPOINT: &str = "SUNAT_STATUS_ENDPOINT";
pub const ENV_USER: &str = "SUNAT_SOL_USER";
pub const ENV_PASSWORD: &str = "SUNAT_SOL_PASSWORD";
pub const ENV_CERT_PATH: &str = "SUNAT_CERT_PATH";
pub const ENV_KEY_PATH: &str = "SUNAT_KEY_PATH";
pub const ENV_HTTP_TIMEOUT: &str = "SUNAT_HTTP_TIMEOUT_SECONDS";

/// The wait before this flag existed; kept as the default so current callers see no change.
pub const DEFAULT_HTTP_TIMEOUT_SECONDS: u64 = 90;
/// A queue worker that waits longer than this on SUNAT is already broken; above it is a typo.
pub const MAX_HTTP_TIMEOUT_SECONDS: u64 = 300;

const CONNECTION_FLAGS: &[&str] = &["endpoint", "user", "password", "timeout"];
const SIGNING_FLAGS: &[&str] = &["cert", "key"];

/// Where and as whom to call SUNAT. `user` is the RUC followed by the SOL user; `timeout` bounds
/// the whole HTTP exchange (connect, send, wait, read).
pub struct Connection {
    pub endpoint: String,
    pub user: String,
    pub password: String,
    pub timeout: Duration,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("endpoint", &self.endpoint)
            .field("user", &"<redacted>")
            .field("password", &"<redacted>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// Flags take precedence over the environment. `endpoint_env` is the variable for this command's
/// service (sendBill and getStatusCdr live at different URLs). Only `https://` endpoints pass.
pub fn connection(
    args: &[String],
    endpoint_env: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Connection, String> {
    let pick = |flag: &str, var: &str| -> Result<String, String> {
        flag_value(args, CONNECTION_FLAGS, flag)?
            .or_else(|| env(var))
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("missing --{flag} or {var}"))
    };
    let endpoint = pick("endpoint", endpoint_env)?;
    if !endpoint.starts_with("https://") {
        return Err("endpoint must use https://".into());
    }
    let timeout = http_timeout(flag_value(args, CONNECTION_FLAGS, "timeout")?.or_else(|| env(ENV_HTTP_TIMEOUT)))?;
    Ok(Connection { endpoint, user: pick("user", ENV_USER)?, password: pick("password", ENV_PASSWORD)?, timeout })
}

/// `--timeout`/SUNAT_HTTP_TIMEOUT_SECONDS: whole seconds from 1 to [`MAX_HTTP_TIMEOUT_SECONDS`];
/// absent or empty means [`DEFAULT_HTTP_TIMEOUT_SECONDS`].
fn http_timeout(value: Option<String>) -> Result<Duration, String> {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return Ok(Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECONDS));
    };
    value
        .parse::<u64>()
        .ok()
        .filter(|s| (1..=MAX_HTTP_TIMEOUT_SECONDS).contains(s))
        .map(Duration::from_secs)
        .ok_or_else(|| format!("--timeout must be whole seconds from 1 to {MAX_HTTP_TIMEOUT_SECONDS}"))
}

/// Paths of the PEM certificate and the PEM private key used by `sign`. The files are read by the
/// caller; their content is never printed.
#[derive(Debug)]
pub struct KeyFiles {
    pub cert: String,
    pub key: String,
}

/// `--cert`/SUNAT_CERT_PATH and `--key`/SUNAT_KEY_PATH; flags take precedence.
pub fn key_files(args: &[String], env: &dyn Fn(&str) -> Option<String>) -> Result<KeyFiles, String> {
    let pick = |flag: &str, var: &str| -> Result<String, String> {
        flag_value(args, SIGNING_FLAGS, flag)?
            .or_else(|| env(var))
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("missing --{flag} or {var}"))
    };
    Ok(KeyFiles { cert: pick("cert", ENV_CERT_PATH)?, key: pick("key", ENV_KEY_PATH)? })
}

/// Value of `--name value` or `--name=value`; a flag outside `allowed` is an error.
fn flag_value(args: &[String], allowed: &[&str], name: &str) -> Result<Option<String>, String> {
    let mut found = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let Some(flag) = arg.strip_prefix("--") else {
            return Err(format!("unexpected argument `{arg}`"));
        };
        let (key, inline) = match flag.split_once('=') {
            Some((key, value)) => (key, Some(value.to_string())),
            None => (flag, None),
        };
        if !allowed.contains(&key) {
            return Err(format!("unknown flag --{key}"));
        }
        let value = match inline {
            Some(value) => value,
            None => iter.next().cloned().ok_or_else(|| format!("--{key} needs a value"))?,
        };
        if key == name {
            found = Some(value);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |key| pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string())
    }

    #[test]
    fn flags_win_over_environment() {
        let env = env_of(&[(ENV_ENDPOINT, "https://env.example"), (ENV_USER, "envuser"), (ENV_PASSWORD, "envpass")]);
        let c = connection(&args(&["--endpoint=https://flag.example", "--user", "flaguser"]), ENV_ENDPOINT, &env).unwrap();
        assert_eq!((c.endpoint.as_str(), c.user.as_str(), c.password.as_str()), ("https://flag.example", "flaguser", "envpass"));
    }

    #[test]
    fn missing_values_and_plain_http_are_rejected() {
        let none = |_: &str| None;
        assert_eq!(connection(&[], ENV_ENDPOINT, &none).unwrap_err(), "missing --endpoint or SUNAT_ENDPOINT");
        let err = connection(&args(&["--endpoint", "http://x", "--user", "u", "--password", "p"]), ENV_ENDPOINT, &none);
        assert_eq!(err.unwrap_err(), "endpoint must use https://");
        assert!(connection(&args(&["--token", "x"]), ENV_ENDPOINT, &none).is_err());
    }

    #[test]
    fn http_timeout_comes_from_flag_or_environment_with_bounds() {
        let base = ["--endpoint", "https://e", "--user", "u", "--password", "p"];
        let with = |extra: &[&str], env: &dyn Fn(&str) -> Option<String>| {
            let mut list = base.to_vec();
            list.extend_from_slice(extra);
            connection(&args(&list), ENV_ENDPOINT, env).map(|c| c.timeout.as_secs())
        };
        let none = |_: &str| None;
        assert_eq!(with(&[], &none), Ok(DEFAULT_HTTP_TIMEOUT_SECONDS));
        assert_eq!(with(&["--timeout", "45"], &none), Ok(45));
        let env = env_of(&[(ENV_HTTP_TIMEOUT, "30")]);
        assert_eq!(with(&[], &env), Ok(30));
        assert_eq!(with(&["--timeout=20"], &env), Ok(20), "the flag wins");
        for bad in ["0", "301", "1.5", "-3", "abc"] {
            assert!(with(&["--timeout", bad], &none).is_err(), "{bad}");
        }
    }

    #[test]
    fn key_files_come_from_flags_or_environment() {
        let env = env_of(&[(ENV_CERT_PATH, "/env/cert.pem"), (ENV_KEY_PATH, "/env/key.pem")]);
        let files = key_files(&args(&["--key=/flag/key.pem"]), &env).unwrap();
        assert_eq!((files.cert.as_str(), files.key.as_str()), ("/env/cert.pem", "/flag/key.pem"));
        let none = |_: &str| None;
        assert_eq!(key_files(&args(&["--cert", "c.pem"]), &none).unwrap_err(), "missing --key or SUNAT_KEY_PATH");
        assert!(key_files(&args(&["--endpoint", "https://x"]), &none).is_err());
    }

    #[test]
    fn debug_never_shows_credentials() {
        let c = Connection {
            endpoint: "https://e".into(),
            user: "20100066603SECRETU".into(),
            password: "s3cr3t".into(),
            timeout: Duration::from_secs(90),
        };
        let shown = format!("{c:?}");
        assert!(!shown.contains("s3cr3t") && !shown.contains("SECRETU"));
    }
}
