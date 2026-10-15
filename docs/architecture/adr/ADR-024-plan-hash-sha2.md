# ADR-024 — Hash do plano com `sha2`

- **Status:** Aceita (2026-10-15)
- **Contexto:** a spec 008 prende a aprovação ao plano exato (`plan_hash`) e impede execução
  repetida (`idempotency_key`), como pedem as seções V.7 e V.10 do plano v1. Isso exige um hash
  criptográfico; a biblioteca padrão do Rust não tem.

## Decisão

- **`sha2` 0.11** (MIT OR Apache-2.0; projeto RustCrypto, repositório `RustCrypto/hashes` ativo,
  último push em 2026-10-14), usado no crate `hmd-executor`.
- Dependências que vêm junto, todas MIT OR Apache-2.0: `digest`, `block-buffer`,
  `crypto-common`, `hybrid-array`, `typenum`, `const-oid`, `cpufeatures`, `cfg-if`.
- O texto que entra no hash é uma codificação canônica própria (`hmd-plan@1`, documentada em
  `crates/hmd-executor/src/canonical.rs`), e não JSON. Isso evita uma dependência de serialização
  e torna explícita a ordem dos campos.
- O nonce da aprovação é o ID sequencial da linha no SQLite. Localmente o que importa é ele ser
  único, não secreto; por isso não entra gerador de números aleatórios.

## Consequências

Quando houver `AuditEvent` com cadeia de hash, ele usa o mesmo `sha2`. A mesma ressalva de cadeia
de suprimentos da ADR-022 vale aqui.

## Revisar se

O `sha2` tiver advisory, ou se a biblioteca padrão passar a oferecer SHA-256.
