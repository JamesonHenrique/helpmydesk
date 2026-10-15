// SPDX-License-Identifier: GPL-3.0-or-later
//! G-SEC (docs/contributing/catalog.md): no fix runs in production until it has lab evidence.
//! When a fix is released, this test is updated in the same reviewed change.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use hmd_executor::Registry;

/// Fixes released for production, with the evidence that allowed them.
const RELEASED: &[&str] = &[];

#[test]
fn production_registry_matches_the_released_list() {
    let catalog = hmd_catalog::embedded().unwrap();
    let registry = Registry::production();
    for cap in catalog.catalog.iter() {
        assert_eq!(
            registry.contains(&cap.id),
            RELEASED.contains(&cap.id.as_str()),
            "{}: production implementation without a release entry (or the reverse)",
            cap.id
        );
    }
}
