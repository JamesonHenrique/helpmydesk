# ADR-021 — Formato do catálogo (TOML) e dependências das sondas

- **Status:** Aceita (2026-10-14)
- **Contexto:** o plano v1 previa o catálogo em YAML. Em 2026-10-14, o crate `serde_yaml` está
  arquivado no GitHub e publicado como `0.9.34+deprecated`. As alternativas YAML em Rust são
  forks com manutenção incerta.

## Decisão

1. Catálogo em **TOML**, um arquivo por capacidade, com o crate `toml` 1.1 (MIT OR Apache-2.0,
   mantido; último push em 2026-10-09) e `serde` 1.0 (MIT OR Apache-2.0).
2. Sondas usam o **`sysinfo` 0.39** (MIT; último push em 2026-10-14), somente com as features
   necessárias. Ele oferece API segura, sem `unsafe` no nosso código, nas duas plataformas.

## Consequências

- O plano, que fala em `*.yaml`, passa a ler como `*.toml`. O conteúdo do cartão de capacidade
  não muda.
- O TOML aceita tabelas aninhadas (`[variants.linux]`) e é legível para quem contribui.
- `sysinfo` exige Rust ≥ 1.95 (o projeto usa 1.99).

## Revisar se

Algum dos crates for arquivado, ou surgir uma necessidade que o `sysinfo` não cubra sem `unsafe`
(nesse caso, um crate de FFI dedicado e isolado, com revisão).
