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
    /// kept (in the vault), together with the consent. The shell runs the call itself, outside
    /// the backend lock, and then calls [`App::ai_connect_finish`].
    pub fn ai_connect(
        &mut self,
        vault: &dyn KeyVault,
        transport: &dyn Transport,
        key: &str,
        now: Timestamp,
    ) -> Result<()> {
        let key = key.trim();
        let result = hmd_ai::classify(transport, key, DEFAULT_MODEL, CONNECTION_TEST);
        self.ai_connect_finish(vault, key, result, now)
    }

    /// The text sent to test a key (it says nothing about the person).
    pub fn ai_connection_test() -> &'static str {
        CONNECTION_TEST
    }

    /// Second half of [`App::ai_connect`]: keeps the key only if the test call proved it works.
    pub fn ai_connect_finish(
        &mut self,
        vault: &dyn KeyVault,
        key: &str,
        result: std::result::Result<Classification, AiError>,
        now: Timestamp,
    ) -> Result<()> {
        match result {
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
        vault.set(key.trim()).map_err(AppError::Vault)?;
        self.set_consent(AI_CONSENT, true, now)
    }

    /// "Desligar a IA": removes the key and the consent.
    pub fn ai_disconnect(&mut self, vault: &dyn KeyVault, now: Timestamp) -> Result<()> {
        vault.delete().map_err(AppError::Vault)?;
        self.set_consent(AI_CONSENT, false, now)
    }

    /// Classifies what the person wrote: [`App::ai_prepare`], the call, [`App::ai_finish`].
    pub fn ai_classify(
        &mut self,
        vault: &dyn KeyVault,
        transport: &dyn Transport,
        text: &str,
        now: Timestamp,
    ) -> Result<Classification> {
        let call = self.ai_prepare(vault, text)?;
        let result = call.run(transport);
        self.ai_finish(call.model, result, now)
    }

    /// Everything checked before anything leaves the computer: consent, key, kill switch, and
    /// the text redacted (user and host names). Only a session the person started calls this.
    pub fn ai_prepare(&self, vault: &dyn KeyVault, text: &str) -> Result<AiCall> {
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
        Ok(AiCall {
            key,
            model: DEFAULT_MODEL,
            text: hmd_core::report::redact_free_text(text),
        })
    }

    /// F3-TRANSP: exactly the text that would leave the computer (after the redaction).
    pub fn ai_preview(text: &str) -> String {
        hmd_core::report::redact_free_text(text)
    }

    /// F3-TRANSP: the AI calls recorded in the audit (newest first): when, which model, outcome.
    /// The text is never stored, so it cannot be shown here.
    pub fn ai_log(&self, limit: usize) -> Result<Vec<AiLogEntry>> {
        let mut out: Vec<AiLogEntry> = self
            .store
            .audit_events()?
            .into_iter()
            .filter(|e| e.action == "ai.classify")
            .map(|e| AiLogEntry {
                at: e.occurred_at,
                model: e.subject_ref.trim_start_matches("model:").to_string(),
                outcome: e.reason_code.unwrap_or(e.outcome),
            })
            .collect();
        out.reverse();
        out.truncate(limit);
        Ok(out)
    }

    /// Records the call (codes only) and returns its result.
    pub fn ai_finish(
        &mut self,
        model: &str,
        result: std::result::Result<Classification, AiError>,
        now: Timestamp,
    ) -> Result<Classification> {
        let outcome = match &result {
            Ok(c) => Ok(c.family.map_or("unknown", |f| f.code())),
            Err(e) => Err(e.reason_code()),
        };
        self.store.record_ai_call(model, outcome, now)?;
        result.map_err(AppError::Ai)
    }
}

/// One recorded AI call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiLogEntry {
    pub at: Timestamp,
    pub model: String,
    /// `ok:<family>`, `ok:connection_test` or an error code.
    pub outcome: String,
}

/// A prepared AI call: can run without holding the app (the network may take seconds).
pub struct AiCall {
    key: String,
    pub model: &'static str,
    pub text: String,
}

impl AiCall {
    pub fn run(&self, transport: &dyn Transport) -> std::result::Result<Classification, AiError> {
        hmd_ai::classify(transport, &self.key, self.model, &self.text)
    }
}

impl std::fmt::Debug for AiCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the key.
        f.debug_struct("AiCall")
            .field("model", &self.model)
            .field("text", &self.text)
            .finish_non_exhaustive()
    }
}
