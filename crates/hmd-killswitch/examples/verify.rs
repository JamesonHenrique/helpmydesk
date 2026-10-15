// SPDX-License-Identifier: GPL-3.0-or-later
//! Checks a kill switch file before publishing it (docs/ops/kill-switch.md):
//! `cargo run -q -p hmd-killswitch --example verify -- <public-key-hex> <file.json> <file.json.sig>`

fn hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [key, file, sig] = args.as_slice() else {
        eprintln!("usage: verify <public-key-hex> <file.json> <file.json.sig>");
        std::process::exit(2);
    };
    let (Some(key), Ok(bytes), Ok(sig)) = (
        hex32(key),
        std::fs::read(file),
        std::fs::read_to_string(sig),
    ) else {
        eprintln!("cannot read the key, the file or the signature");
        std::process::exit(2);
    };
    match hmd_killswitch::verify(&bytes, &sig, &[key]) {
        Ok(ks) => println!(
            "OK: issued_at={} expires_at={} disabled_capabilities={:?} disabled_models={:?}",
            ks.issued_at, ks.expires_at, ks.disabled_capabilities, ks.disabled_models
        ),
        Err(e) => {
            eprintln!("REFUSED: {e}");
            std::process::exit(1);
        }
    }
}
