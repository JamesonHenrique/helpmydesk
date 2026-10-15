// SPDX-License-Identifier: GPL-3.0-or-later
//! The AI key in the operating system's vault (spec 018): Secret Service on Linux, Windows
//! Credential Manager on Windows (`keyring`, ADR-028). Its own crate so CI can test it on a real
//! Windows machine.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use hmd_app::ai::KeyVault;

const SERVICE: &str = "io.github.jamesonhenrique.helpmydesk";
const ACCOUNT: &str = "opencode-api-key";

pub struct SystemVault {
    entry: keyring::Entry,
}

impl SystemVault {
    /// The app's entry.
    pub fn new() -> Result<SystemVault, String> {
        SystemVault::for_account(ACCOUNT)
    }

    fn for_account(account: &str) -> Result<SystemVault, String> {
        keyring::Entry::store_status()
            .as_ref()
            .map_err(ToString::to_string)?;
        let entry = keyring::Entry::new(SERVICE, account).map_err(|e| e.to_string())?;
        Ok(SystemVault { entry })
    }
}

impl KeyVault for SystemVault {
    fn get(&self) -> Result<Option<String>, String> {
        match self.entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, key: &str) -> Result<(), String> {
        self.entry.set_password(key).map_err(|e| e.to_string())
    }

    fn delete(&self) -> Result<(), String> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes, reads and deletes a **test** entry (never the app's key) in the real vault. Runs
    /// only with `HMD_VAULT_TEST=1` (set in CI), so a developer's keyring is not touched by a
    /// plain `cargo test`.
    #[test]
    fn round_trip_in_the_real_vault() {
        if std::env::var("HMD_VAULT_TEST").as_deref() != Ok("1") {
            return;
        }
        let account = format!("test-{}", std::process::id());
        let vault = match SystemVault::for_account(&account) {
            Ok(v) => v,
            // Linux CI has no Secret Service: report it and stop (Windows must work).
            Err(e) if cfg!(not(windows)) => {
                eprintln!("no system vault here: {e}");
                return;
            }
            Err(e) => panic!("Windows Credential Manager unavailable: {e}"),
        };
        assert_eq!(vault.get().unwrap(), None);
        vault.set("sk-test-value").unwrap();
        assert_eq!(vault.get().unwrap().as_deref(), Some("sk-test-value"));
        vault.delete().unwrap();
        assert_eq!(vault.get().unwrap(), None);
        vault.delete().unwrap();
    }
}
