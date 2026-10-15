// SPDX-License-Identifier: GPL-3.0-or-later
//! The AI key in the operating system's vault (spec 018): Secret Service on Linux, Windows
//! Credential Manager on Windows (`keyring`, ADR-028).

use hmd_ipc::KeyVault;

const SERVICE: &str = "io.github.jamesonhenrique.helpmydesk";
const ACCOUNT: &str = "opencode-api-key";

pub struct SystemVault {
    entry: keyring::Entry,
}

impl SystemVault {
    pub fn new() -> Result<SystemVault, String> {
        keyring::Entry::store_status()
            .as_ref()
            .map_err(ToString::to_string)?;
        let entry = keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| e.to_string())?;
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
