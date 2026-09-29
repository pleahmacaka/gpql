use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::engines::db::SessionConfig;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLogin {
    pub url: String,
    pub kind: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub path: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub tls: String,
    #[serde(default)]
    pub warehouse: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub tunnel: crate::net::tunnel::TunnelConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedLogin {
    #[serde(flatten)]
    login: SavedLogin,
    has_password: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credential {
    pub name: String,
    pub user: String,
    pub password: String,
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub key: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Vault {
    #[serde(default)]
    logins: Vec<SavedLogin>,
    #[serde(default)]
    presets: Vec<Credential>,
    #[serde(default)]
    providers: Vec<Provider>,
}

enum Trouble {
    Io(String),
    Broken(String),
}

static CHANGING: Mutex<()> = Mutex::new(());
static SET_ASIDE: Mutex<Option<String>> = Mutex::new(None);

impl SavedLogin {
    fn from(config: &SessionConfig) -> Self {
        SavedLogin {
            url: describe(config),
            kind: config.kind.clone(),
            host: config.host.clone(),
            port: config.port.clone(),
            user: config.user.clone(),
            password: config.password.clone(),
            database: config.database.clone(),
            path: config.path.clone(),
            endpoint: config.url.clone(),
            token: config.token.clone(),
            tls: config.tls.clone(),
            warehouse: config.warehouse.clone(),
            schema: config.schema.clone(),
            tunnel: config.tunnel.clone(),
        }
    }

    fn secrets(&mut self) -> [&mut String; 4] {
        [
            &mut self.password,
            &mut self.token,
            &mut self.tunnel.password,
            &mut self.tunnel.passphrase,
        ]
    }

    fn without_secrets(mut self) -> ListedLogin {
        let endpoint = strip_secrets(&self.endpoint);
        let mut has_password = endpoint != self.endpoint;

        self.endpoint = endpoint;

        for secret in self.secrets() {
            has_password |= !secret.is_empty();
            secret.clear();
        }

        ListedLogin {
            login: self,
            has_password,
        }
    }
}

pub fn builtin_credentials() -> Vec<Credential> {
    [
        ("postgres", "postgres", ""),
        ("postgres with password", "postgres", "postgres"),
    ]
    .into_iter()
    .map(|(name, user, password)| Credential {
        name: name.to_string(),
        user: user.to_string(),
        password: password.to_string(),
        builtin: true,
    })
    .collect()
}

pub fn describe(config: &SessionConfig) -> String {
    let kind = &config.kind;

    if !config.path.is_empty() {
        return format!("{kind}://{}", config.path);
    }

    if !config.url.is_empty() {
        let bare = config
            .url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');

        if config.database.is_empty() {
            return strip_secrets(&format!("{kind}://{bare}"));
        }

        return strip_secrets(&format!("{kind}://{bare}/{}", config.database));
    }

    let host = if config.host.is_empty() {
        "127.0.0.1"
    } else {
        &config.host
    };
    let port = if config.port.is_empty() {
        "5432"
    } else {
        &config.port
    };

    format!("{kind}://{}@{host}:{port}/{}", config.user, config.database)
}

const SECRET_PARAMS: [&str; 8] = [
    "token",
    "pass",
    "pwd",
    "secret",
    "key",
    "auth",
    "sig",
    "credential",
];

// mirrors stripSecrets in the frontend's commands.ts, which keys recents the same way
pub fn strip_secrets(text: &str) -> String {
    without_secret_params(&without_passwords(text))
}

// an unencoded password may hold / ? # or @, so only a ?name= starts the query
fn query_start(part: &str) -> usize {
    part.match_indices('?')
        .map(|(at, _)| at)
        .find(|&at| {
            let rest = &part[at + 1..];

            match (rest.find('='), rest.find('@')) {
                (Some(equals), Some(sign)) => equals < sign,
                (Some(_), None) => true,
                (None, _) => false,
            }
        })
        .unwrap_or(part.len())
}

fn without_passwords(text: &str) -> String {
    text.split("://")
        .enumerate()
        .map(|(index, part)| {
            let head = &part[..query_start(part)];

            match head.rfind('@') {
                Some(sign) if index > 0 && head[..sign].contains(':') => &part[sign + 1..],
                _ => part,
            }
        })
        .collect::<Vec<_>>()
        .join("://")
}

fn without_secret_params(text: &str) -> String {
    let Some(at) = text.find('?') else {
        return text.to_string();
    };

    let secret = |name: &str| {
        let name = name.to_lowercase();

        SECRET_PARAMS.iter().any(|word| name.contains(word))
    };

    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(&text.as_bytes()[at + 1..])
        .into_owned()
        .collect();

    if !pairs.iter().any(|(name, _)| secret(name)) {
        return text.to_string();
    }

    let rest = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs.iter().filter(|(name, _)| !secret(name)))
        .finish();

    if rest.is_empty() {
        text[..at].to_string()
    } else {
        format!("{}?{rest}", &text[..at])
    }
}

fn home_file(name: &str) -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(name))
        .ok_or_else(|| "no home folder on this machine".to_string())
}

pub fn logins_path() -> Result<PathBuf, String> {
    home_file(".gpql-logins")
}

fn account_path() -> Result<PathBuf, String> {
    home_file(".gpql-account")
}

fn read_at(path: &Path) -> Result<Vault, Trouble> {
    let sealed = match fs::read(path) {
        Ok(sealed) => sealed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vault::default());
        }
        Err(error) => {
            return Err(Trouble::Io(format!(
                "the saved logins at {} cannot be read: {error}",
                path.display()
            )));
        }
    };

    let plain = unseal(&sealed).map_err(Trouble::Broken)?;

    let vault = match serde_json::from_slice::<Vault>(&plain) {
        Ok(vault) => vault,
        Err(_) => serde_json::from_slice(&plain)
            .map(|logins| Vault {
                logins,
                ..Vault::default()
            })
            .map_err(|error| Trouble::Broken(error.to_string()))?,
    };

    Ok(rekeyed(vault))
}

// older logins were keyed by a url that still carried its password; the next write saves the new key
fn rekeyed(mut vault: Vault) -> Vault {
    let mut seen = std::collections::HashSet::new();

    vault.logins.retain_mut(|login| {
        login.url = strip_secrets(&login.url);

        seen.insert(login.url.clone())
    });

    vault
}

// an unreadable file moves aside first, so a write never lands on top of it
fn load(path: &Path) -> Result<Vault, Trouble> {
    match read_at(path) {
        Err(Trouble::Broken(reason)) => {
            let backup = set_aside(path, &reason).map_err(Trouble::Broken)?;

            *SET_ASIDE.lock().unwrap_or_else(PoisonError::into_inner) = Some(backup);

            Ok(Vault::default())
        }
        other => other,
    }
}

fn read() -> Result<Vault, String> {
    let _held = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);

    match load(&logins_path()?) {
        Ok(vault) => Ok(vault),
        Err(Trouble::Io(message)) => Err(message),
        Err(Trouble::Broken(_)) => Ok(Vault::default()),
    }
}

pub fn set_aside_notice() -> Option<String> {
    SET_ASIDE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
}

fn set_aside(path: &Path, reason: &str) -> Result<String, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let mut backup = path.as_os_str().to_owned();

    backup.push(format!(".unreadable-{stamp}"));

    let backup = PathBuf::from(backup);

    match fs::rename(path, &backup) {
        Ok(()) => Ok(backup.display().to_string()),
        Err(error) => Err(format!(
            "the saved logins at {} cannot be opened ({reason}) and could not be moved aside: {error}",
            path.display()
        )),
    }
}

fn change(edit: impl FnOnce(&mut Vault)) -> Result<(), String> {
    let _held = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);
    let path = logins_path()?;

    let mut vault = match load(&path) {
        Ok(vault) => vault,
        Err(Trouble::Io(message) | Trouble::Broken(message)) => return Err(message),
    };

    edit(&mut vault);

    let plain = serde_json::to_vec(&vault).map_err(|e| e.to_string())?;

    store(&path, &seal(&plain)?)
}

fn store(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut staged = path.as_os_str().to_owned();

    staged.push(".tmp");

    let staged = PathBuf::from(staged);
    let _ = fs::remove_file(&staged);

    let mut options = fs::OpenOptions::new();

    options.write(true).create_new(true);

    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);

    let written = options.open(&staged).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });

    if let Err(error) = written.and_then(|()| fs::rename(&staged, path)) {
        let _ = fs::remove_file(&staged);

        return Err(format!("{} could not be written: {error}", path.display()));
    }

    Ok(())
}

pub fn list() -> Result<Vec<SavedLogin>, String> {
    read().map(|vault| vault.logins)
}

pub fn listed() -> Result<Vec<ListedLogin>, String> {
    list().map(|logins| {
        logins
            .into_iter()
            .map(SavedLogin::without_secrets)
            .collect()
    })
}

pub fn find(url: &str) -> Result<Option<SavedLogin>, String> {
    let url = strip_secrets(url);

    list().map(|logins| logins.into_iter().find(|saved| saved.url == url))
}

pub fn credentials() -> Result<Vec<Credential>, String> {
    let mut out = builtin_credentials();
    out.extend(read()?.presets);

    Ok(out)
}

pub fn save_credential(credential: Credential) -> Result<(), String> {
    change(|vault| {
        vault.presets.retain(|saved| saved.name != credential.name);
        vault.presets.push(Credential {
            builtin: false,
            ..credential
        });
    })
}

pub fn forget_credential(name: &str) -> Result<(), String> {
    change(|vault| vault.presets.retain(|saved| saved.name != name))
}

pub fn providers() -> Result<Vec<Provider>, String> {
    read().map(|vault| vault.providers)
}

pub fn save_provider(provider: Provider) -> Result<(), String> {
    change(|vault| {
        vault.providers.retain(|saved| saved.id != provider.id);
        vault.providers.push(provider);
    })
}

pub fn forget_provider(id: &str) -> Result<(), String> {
    change(|vault| vault.providers.retain(|saved| saved.id != id))
}

pub fn remember(config: &SessionConfig) -> Result<(), String> {
    let entry = SavedLogin::from(config);

    change(|vault| {
        vault.logins.retain(|saved| saved.url != entry.url);
        vault.logins.insert(0, entry);
    })
}

pub fn forget(url: &str) -> Result<(), String> {
    let url = strip_secrets(url);

    change(|vault| vault.logins.retain(|saved| saved.url != url))
}

pub fn forget_all() -> Result<(), String> {
    change(|vault| vault.logins.clear())
}

pub fn account_token() -> Option<String> {
    let path = account_path().ok()?;
    let raw = fs::read(path).ok()?;

    let token = match unseal(&raw) {
        Ok(plain) => String::from_utf8(plain).ok()?,
        Err(_) => {
            let legacy = String::from_utf8(raw).ok()?;

            if !legacy.trim().chars().all(|c| c.is_ascii_graphic()) {
                return None;
            }

            let _ = set_account_token(&legacy);

            legacy
        }
    };

    let token = token.trim().to_string();

    (!token.is_empty()).then_some(token)
}

pub fn set_account_token(token: &str) -> Result<(), String> {
    store(&account_path()?, &seal(token.trim().as_bytes())?)
}

pub fn clear_account() -> Result<(), String> {
    let path = account_path()?;

    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(windows)]
fn seal(plain: &[u8]) -> Result<Vec<u8>, String> {
    windows_dpapi::protect(plain)
}

#[cfg(windows)]
fn unseal(sealed: &[u8]) -> Result<Vec<u8>, String> {
    windows_dpapi::unprotect(sealed)
}

#[cfg(not(windows))]
fn seal(plain: &[u8]) -> Result<Vec<u8>, String> {
    // ponytail: plaintext off Windows, swap in the OS keyring before shipping mac/linux builds
    return Ok(plain.to_vec());
}

#[cfg(not(windows))]
fn unseal(sealed: &[u8]) -> Result<Vec<u8>, String> {
    return Ok(sealed.to_vec());
}

#[cfg(windows)]
mod windows_dpapi {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    fn blob(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let copied =
            unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        let _ = unsafe { LocalFree(Some(HLOCAL(out.pbData as *mut _))) };

        copied
    }

    pub fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(plain);
        let mut output = CRYPT_INTEGER_BLOB::default();

        unsafe {
            CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output)
                .map_err(|e| e.message())?;

            Ok(take(output))
        }
    }

    pub fn unprotect(sealed: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(sealed);
        let mut output = CRYPT_INTEGER_BLOB::default();

        unsafe {
            CryptUnprotectData(&input, None, None, None, None, 0, &mut output)
                .map_err(|e| e.message())?;

            Ok(take(output))
        }
    }
}

#[cfg(test)]
mod saving {
    use super::*;

    #[test]
    fn a_saved_login_carries_its_jump_host_back() {
        let mut config = SessionConfig {
            kind: "postgres".into(),
            host: "10.0.0.9".into(),
            port: "5432".into(),
            user: "app".into(),
            database: "app".into(),
            ..Default::default()
        };

        config.tunnel.host = "jump.example.com".into();
        config.tunnel.port = "50022".into();
        config.tunnel.user = "ops".into();
        config.tunnel.key_path = "C:/keys/id_ed25519".into();
        config.tunnel.local_port = "5433".into();

        let entry = SavedLogin::from(&config);
        let carried: SavedLogin =
            serde_json::from_str(&serde_json::to_string(&entry).unwrap()).unwrap();

        assert_eq!(carried.tunnel.host, "jump.example.com");
        assert_eq!(carried.tunnel.port, "50022");
        assert_eq!(carried.tunnel.user, "ops");
        assert_eq!(carried.tunnel.key_path, "C:/keys/id_ed25519");
        assert_eq!(carried.tunnel.local_port, "5433");
        assert!(carried.tunnel.wanted());
    }

    #[test]
    fn a_direct_login_asks_for_no_jump_host() {
        let config = SessionConfig {
            kind: "sqlite".into(),
            path: "C:/app.db".into(),
            ..Default::default()
        };

        assert!(!SavedLogin::from(&config).tunnel.wanted());
    }
}
