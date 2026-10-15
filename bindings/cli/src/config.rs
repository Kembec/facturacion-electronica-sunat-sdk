//! Endpoint and SOL credentials, from flags or environment. Nothing here is ever printed.

use std::fmt;

pub const ENV_ENDPOINT: &str = "SUNAT_ENDPOINT";
pub const ENV_STATUS_ENDPOINT: &str = "SUNAT_STATUS_ENDPOINT";
pub const ENV_USER: &str = "SUNAT_SOL_USER";
pub const ENV_PASSWORD: &str = "SUNAT_SOL_PASSWORD";
pub const ENV_CERT_PATH: &str = "SUNAT_CERT_PATH";
pub const ENV_KEY_PATH: &str = "SUNAT_KEY_PATH";

const CONNECTION_FLAGS: &[&str] = &["endpoint", "user", "password"];
const SIGNING_FLAGS: &[&str] = &["cert", "key"];

/// Where and as whom to call SUNAT. `user` is the RUC followed by the SOL user.
pub struct Connection {
    pub endpoint: String,
    pub user: String,
    pub password: String,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("endpoint", &self.endpoint)
            .field("user", &"<redacted>")
            .field("password", &"<redacted>")
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
    Ok(Connection { endpoint, user: pick("user", ENV_USER)?, password: pick("password", ENV_PASSWORD)? })
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
        let c = Connection { endpoint: "https://e".into(), user: "20100066603SECRETU".into(), password: "s3cr3t".into() };
        let shown = format!("{c:?}");
        assert!(!shown.contains("s3cr3t") && !shown.contains("SECRETU"));
    }
}
