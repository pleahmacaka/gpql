use std::fs;
use std::path::PathBuf;

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

impl SavedLogin {
    fn from(config: &SessionConfig) -> Self {
        return SavedLogin {
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
        };
    }
}

pub fn builtin_credentials() -> Vec<Credential> {
    return [("postgres", "postgres", ""), ("postgres with password", "postgres", "postgres")]
        .into_iter()
        .map(|(name, user, password)| Credential {
            name: name.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            builtin: true,
        })
        .collect();
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
            return format!("{kind}://{bare}");
        }

        return format!("{kind}://{bare}/{}", config.database);
    }

    let host = if config.host.is_empty() { "127.0.0.1" } else { &config.host };
    let port = if config.port.is_empty() { "5432" } else { &config.port };

    return format!("{kind}://{}@{host}:{port}/{}", config.user, config.database);
}

fn home_file(name: &str) -> Result<PathBuf, String> {
    return dirs::home_dir()
        .map(|home| home.join(name))
        .ok_or_else(|| "no home folder on this machine".to_string());
}

pub fn logins_path() -> Result<PathBuf, String> {
    return home_file(".gpql-logins");
}

fn account_path() -> Result<PathBuf, String> {
    return home_file(".gpql-account");
}

fn read() -> Vault {
    let Ok(path) = logins_path() else {
        return Vault::default();
    };
    let Ok(sealed) = fs::read(&path) else {
        return Vault::default();
    };
    let Ok(plain) = unseal(&sealed) else {
        return Vault::default();
    };

    if let Ok(vault) = serde_json::from_slice::<Vault>(&plain) {
        return vault;
    }

    return Vault {
        logins: serde_json::from_slice(&plain).unwrap_or_default(),
        ..Vault::default()
    };
}

fn write(vault: &Vault) -> Result<(), String> {
    let path = logins_path()?;
    let plain = serde_json::to_vec(vault).map_err(|e| e.to_string())?;

    return fs::write(path, seal(&plain)?).map_err(|e| e.to_string());
}

pub fn list() -> Vec<SavedLogin> {
    return read().logins;
}

pub fn credentials() -> Vec<Credential> {
    let mut out = builtin_credentials();
    out.extend(read().presets);

    return out;
}

pub fn save_credential(credential: Credential) -> Result<(), String> {
    let mut vault = read();

    vault.presets.retain(|saved| saved.name != credential.name);
    vault.presets.push(Credential {
        builtin: false,
        ..credential
    });

    return write(&vault);
}

pub fn forget_credential(name: &str) -> Result<(), String> {
    let mut vault = read();
    vault.presets.retain(|saved| saved.name != name);

    return write(&vault);
}

pub fn providers() -> Vec<Provider> {
    return read().providers;
}

pub fn save_provider(provider: Provider) -> Result<(), String> {
    let mut vault = read();

    vault.providers.retain(|saved| saved.id != provider.id);
    vault.providers.push(provider);

    return write(&vault);
}

pub fn forget_provider(id: &str) -> Result<(), String> {
    let mut vault = read();
    vault.providers.retain(|saved| saved.id != id);

    return write(&vault);
}

pub fn remember(config: &SessionConfig) -> Result<(), String> {
    let entry = SavedLogin::from(config);
    let mut vault = read();

    vault.logins.retain(|saved| saved.url != entry.url);
    vault.logins.insert(0, entry);
    vault.logins.truncate(20);

    return write(&vault);
}

pub fn forget(url: &str) -> Result<(), String> {
    let mut vault = read();
    vault.logins.retain(|saved| saved.url != url);

    return write(&vault);
}

pub fn forget_all() -> Result<(), String> {
    let mut vault = read();
    vault.logins.clear();

    return write(&vault);
}

pub fn account_token() -> Option<String> {
    let path = account_path().ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let token = raw.trim().to_string();

    if token.is_empty() {
        return None;
    }

    return Some(token);
}

pub fn set_account_token(token: &str) -> Result<(), String> {
    return fs::write(account_path()?, token.trim()).map_err(|e| e.to_string());
}

pub fn clear_account() -> Result<(), String> {
    let path = account_path()?;

    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }

    return Ok(());
}

#[cfg(windows)]
fn seal(plain: &[u8]) -> Result<Vec<u8>, String> {
    return windows_dpapi::protect(plain);
}

#[cfg(windows)]
fn unseal(sealed: &[u8]) -> Result<Vec<u8>, String> {
    return windows_dpapi::unprotect(sealed);
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
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData,
    };
    use windows::core::PCWSTR;

    fn blob(bytes: &[u8]) -> CRYPT_INTEGER_BLOB {
        return CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let copied =
            unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        let _ = unsafe { LocalFree(Some(HLOCAL(out.pbData as *mut _))) };

        return copied;
    }

    pub fn protect(plain: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(plain);
        let mut output = CRYPT_INTEGER_BLOB::default();

        unsafe {
            CryptProtectData(&input, PCWSTR::null(), None, None, None, 0, &mut output)
                .map_err(|e| e.message())?;

            return Ok(take(output));
        }
    }

    pub fn unprotect(sealed: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(sealed);
        let mut output = CRYPT_INTEGER_BLOB::default();

        unsafe {
            CryptUnprotectData(&input, None, None, None, None, 0, &mut output)
                .map_err(|e| e.message())?;

            return Ok(take(output));
        }
    }
}
