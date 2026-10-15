// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 018: "Ligar a ajuda inteligente". The key lives only in the system vault (the shell
//! provides it); the app keeps the consent and an audit line per call (codes only).

use std::sync::Mutex;

use hmd_ai::{AiError, Classification, DEFAULT_MODEL, Transport};
use hmd_domain::Timestamp;

use crate::{App, AppError, Result};

/// Consent code for the AI (catalog `consents.ai_opencode`).
pub const AI_CONSENT: &str = "ai_opencode";

/// The person's key, in the operating system's vault (Secret Service, Windows Credential
/// Manager). Never stored in the history file.
pub trait KeyVault: Send {
    fn get(&self) -> std::result::Result<Option<String>, String>;
    fn set(&self, key: &str) -> std::result::Result<(), String>;
    fn delete(&self) -> std::result::Result<(), String>;
}

/// In-memory vault: tests, and when the system vault is unavailable (the key is then forgotten
/// when the app closes).
#[derive(Default)]
pub struct MemoryVault(Mutex<Option<String>>);

impl KeyVault for MemoryVault {
    fn get(&self) -> std::result::Result<Option<String>, String> {
        Ok(self.0.lock().map_err(|e| e.to_string())?.clone())
    }
    fn set(&self, key: &str) -> std::result::Result<(), String> {
        *self.0.lock().map_err(|e| e.to_string())? = Some(key.to_string());
        Ok(())
    }
    fn delete(&self) -> std::result::Result<(), String> {
        *self.0.lock().map_err(|e| e.to_string())? = None;
        Ok(())
    }
}

/// What the screens need to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiStatus {
    pub connected: bool,
}

/// Text sent to test a key: says nothing about the person.
const CONNECTION_TEST: &str = "teste de conexão do HelpMyDesk";

impl App {
    pub fn ai_status(&self, vault: &dyn KeyVault) -> Result<AiStatus> {
        let consent = self.consents()?.contains(AI_CONSENT);
        let key = vault.get().map_err(AppError::Vault)?.is_some();
        Ok(AiStatus {
            connected: consent && key,
        })
    }

    /// The person pasted a key and agreed: one test call; only a key the service accepts is
    /// kept (in the vault), together with the consent.
    pub fn ai_connect(
        &mut self,
        vault: &dyn KeyVault,
        transport: &dyn Transport,
        key: &str,
        now: Timestamp,
    ) -> Result<()> {
        let key = key.trim();
        match hmd_ai::classify(transport, key, DEFAULT_MODEL, CONNECTION_TEST) {
            // A 200 with an odd answer still proves the key works.
            Ok(_) | Err(AiError::BadAnswer(_)) => {}
            Err(e) => {
                self.store
                    .record_ai_call(DEFAULT_MODEL, Err(e.reason_code()), now)?;
                return Err(AppError::Ai(e));
            }
        }
        self.store
            .record_ai_call(DEFAULT_MODEL, Ok("connection_test"), now)?;
        vault.set(key).map_err(AppError::Vault)?;
        self.set_consent(AI_CONSENT, true, now)
    }

    /// "Desligar a IA": removes the key and the consent.
    pub fn ai_disconnect(&mut self, vault: &dyn KeyVault, now: Timestamp) -> Result<()> {
        vault.delete().map_err(AppError::Vault)?;
        self.set_consent(AI_CONSENT, false, now)
    }

    /// Classifies what the person wrote. The text is redacted (user and host names) before it
    /// leaves the computer; only a session the person started calls this.
    pub fn ai_classify(
        &mut self,
        vault: &dyn KeyVault,
        transport: &dyn Transport,
        text: &str,
        now: Timestamp,
    ) -> Result<Classification> {
        if !self.consents()?.contains(AI_CONSENT) {
            return Err(AppError::AiNotConnected);
        }
        let key = vault
            .get()
            .map_err(AppError::Vault)?
            .ok_or(AppError::AiNotConnected)?;
        if self.disabled_models.contains(DEFAULT_MODEL) {
            return Err(AppError::Ai(AiError::ModelNotAllowed));
        }
        let text = hmd_core::report::redact_free_text(text);
        let result = hmd_ai::classify(transport, &key, DEFAULT_MODEL, &text);
        let outcome = match &result {
            Ok(c) => Ok(c.family.map_or("unknown", |f| f.code())),
            Err(e) => Err(e.reason_code()),
        };
        self.store.record_ai_call(DEFAULT_MODEL, outcome, now)?;
        result.map_err(AppError::Ai)
    }
}
