// SPDX-License-Identifier: GPL-3.0-or-later
//! Spec 018: connecting the AI, the vault and the classification, with a fake transport.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;

use hmd_ai::{AiError, Transport};
use hmd_app::App;
use hmd_app::ai::{KeyVault, MemoryVault};
use hmd_domain::ProblemFamily;

const NOW: i64 = 1_760_400_000;

struct Fake {
    status: u16,
    content: &'static str,
    sent: RefCell<Vec<String>>,
}

impl Fake {
    fn ok(content: &'static str) -> Fake {
        Fake {
            status: 200,
            content,
            sent: RefCell::new(vec![]),
        }
    }
}

impl Transport for Fake {
    fn post(&self, _bearer: &str, body: &str) -> Result<(u16, String), AiError> {
        self.sent.borrow_mut().push(body.to_string());
        let body = serde_json::json!({"choices": [{"message": {"content": self.content}}]});
        Ok((self.status, body.to_string()))
    }
}

const NET: &str = r#"{"family": "connectivity", "confidence": "high", "question": null}"#;

fn app() -> App {
    App::open_in_memory(NOW).unwrap().0
}

#[test]
fn a_refused_key_is_not_kept() {
    let mut a = app();
    let vault = MemoryVault::default();
    let fake = Fake {
        status: 401,
        content: NET,
        sent: RefCell::new(vec![]),
    };
    let err = a.ai_connect(&vault, &fake, "wrong", NOW).unwrap_err();
    assert_eq!(err.reason_code(), "AI_KEY_REFUSED");
    assert_eq!(vault.get().unwrap(), None);
    assert!(!a.ai_status(&vault).unwrap().connected);
}

#[test]
fn connect_classify_and_disconnect() {
    let mut a = app();
    let vault = MemoryVault::default();
    let fake = Fake::ok(NET);
    // Not connected: nothing leaves the computer.
    assert_eq!(
        a.ai_classify(&vault, &fake, "a internet caiu", NOW)
            .unwrap_err()
            .reason_code(),
        "AI_NOT_CONNECTED"
    );
    assert!(fake.sent.borrow().is_empty());

    a.ai_connect(&vault, &fake, "  sk-good  ", NOW).unwrap();
    assert_eq!(vault.get().unwrap().as_deref(), Some("sk-good"));
    assert!(a.ai_status(&vault).unwrap().connected);

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let text = format!("a internet caiu no PC do {user}");
    let c = a.ai_classify(&vault, &fake, &text, NOW + 1).unwrap();
    assert_eq!(c.family, Some(ProblemFamily::Connectivity));
    let sent = fake.sent.borrow().last().unwrap().clone();
    if user.chars().count() >= 3 {
        assert!(!sent.contains(&user), "user name redacted before sending");
    }

    let audit = a.export_all(NOW + 2).unwrap();
    assert!(audit.contains("ai.classify"), "every call is recorded");
    assert!(!audit.contains("internet caiu"), "never the text itself");

    a.ai_disconnect(&vault, NOW + 3).unwrap();
    assert_eq!(vault.get().unwrap(), None);
    assert!(!a.ai_status(&vault).unwrap().connected);
}

#[test]
fn a_bad_answer_is_an_error_not_a_family() {
    let mut a = app();
    let vault = MemoryVault::default();
    a.ai_connect(&vault, &Fake::ok(NET), "k", NOW).unwrap();
    let hostile = Fake::ok(
        r#"{"family": "connectivity", "confidence": "high", "question": null, "run": "rm -rf /"}"#,
    );
    assert_eq!(
        a.ai_classify(&vault, &hostile, "x", NOW)
            .unwrap_err()
            .reason_code(),
        "AI_BAD_ANSWER"
    );
}
