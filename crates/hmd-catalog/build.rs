// SPDX-License-Identifier: GPL-3.0-or-later
//! Embeds `catalog/` into the binary (spec 003 FR-005): generates a list of `include_str!`
//! entries so the reviewed files, and only them, ship inside the signed executable.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let catalog_dir = manifest_dir.join("../../catalog");
    let caps_dir = catalog_dir.join("capabilities");
    println!("cargo:rerun-if-changed={}", catalog_dir.display());
    println!("cargo:rerun-if-changed={}", caps_dir.display());

    let mut files: Vec<PathBuf> = fs::read_dir(&caps_dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();

    let manifest = catalog_dir.join("catalog.toml").canonicalize()?;
    println!("cargo:rerun-if-changed={}", manifest.display());
    let mut out = String::new();
    writeln!(
        out,
        "pub(crate) const MANIFEST: &str = include_str!({:?});",
        manifest
    )?;
    writeln!(out, "pub(crate) const FILES: &[(&str, &str)] = &[")?;
    for path in files {
        let path = path.canonicalize()?;
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("non UTF-8 catalog file name")?;
        writeln!(out, "    ({name:?}, include_str!({path:?})),")?;
    }
    writeln!(out, "];")?;
    fs::write(PathBuf::from(env::var("OUT_DIR")?).join("embedded.rs"), out)?;
    Ok(())
}
