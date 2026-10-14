# Guia de desenvolvimento

## Pré-requisitos

- Rust via [rustup](https://rustup.rs/). A versão é fixada em `rust-toolchain.toml` e o rustup
  instala a correta automaticamente.
- Spec Kit `specify-cli` **v1.2.0**:
  `uv tool install specify-cli --from git+https://github.com/github/spec-kit.git@v1.2.0`
- Para o app desktop (a partir da F1/F2): Node.js LTS e as dependências do Tauri v2 para o seu
  sistema (no Linux, WebKitGTK 4.1 e afins — ver documentação do Tauri).

## Comandos

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Fluxo SDD (Spec Kit)

As skills ficam em `.claude/skills/speckit-*`:
`/speckit-specify` → `/speckit-clarify` → `/speckit-plan` → `/speckit-checklist` → `/speckit-tasks` →
`/speckit-analyze` → `/speckit-implement` → `/speckit-converge`.

A feature ativa fica em `.specify/feature.json`. Specs em `specs/NNN-nome/`. Os IDs de requisito
(FR-/NFR-/SEC-) aparecem nos nomes dos testes (ex.: `fr002_sc001_exhaustive_table`).

## Estrutura

Veja o plano v1, VIII.4. Crates atuais:

| Crate | Spec | Conteúdo |
|---|---|---|
| `crates/hmd-domain` | 001 | Ciclo de vida do chamado, fases da sessão, gatilhos de relatório, housekeeping |
