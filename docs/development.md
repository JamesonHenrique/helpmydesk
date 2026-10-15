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
| `crates/hmd-domain` | 001, 007, 008 | Ciclo de vida do chamado, fases da sessão, gatilhos de relatório, housekeeping, retenção, estados da execução |
| `crates/hmd-policy` | 002 | Policy Engine: validação determinística de planos, nível mínimo e validação do catálogo |
| `crates/hmd-catalog` | 003 | Catálogo TOML (`catalog/`): carga estrita, validação, embutido no binário |
| `crates/hmd-core` | 006, 007 | Playbooks, autorização das sondas, diagnóstico, avaliação das correções (sem executar) e relatório "não consegui" |
| `crates/hmd-executor` | 008 | Hash do plano, checagem da aprovação, execução com diário write-ahead e verificação (sem correção real até o S-02) |
| `crates/hmd-app` | 012 | Casos de uso que a interface chama: chamados sem duplicar, diagnóstico, plano, aprovação, execução com prova, relatório, privacidade |
| `crates/hmd-store` | 007, 008 | Histórico local em SQLite: chamados, eventos, sessões, achados, mensagens, consentimentos, relatórios, retenção, planos, aprovações, diário de execução, auditoria encadeada, exportar e apagar tudo |
| `crates/hmd-probes` | 004, 005, 010 | Sondas somente leitura: sistema, memória, CPU, processos, disco, DNS, adaptadores, gateway, HTTP/portal cativo, inicialização, som, microfone e câmera |

Modo básico (texto, sem IA): `cargo run -p hmd-core --example basic_mode` (menu) e
`cargo run -p hmd-core --example basic_mode -- 1` (diagnóstico de Internet). Para ver o relatório:
`cargo run -p hmd-core --example basic_mode -- 1 --relatorio "o que aconteceu"`. O que cada
verificação lê: `cargo run -p hmd-core --example basic_mode -- --o-que-eu-olho`.

Protótipo das telas (Angular, dados de exemplo): veja `apps/desktop/ui/README.md`.

Rodar as sondas na sua máquina: `cargo run -p hmd-probes --example probe_all`.
Guarda da constituição: `tools/check-no-process-spawn.sh`. Tudo o que o CI roda, localmente:
`tools/check-all.sh`.

Checar o código Windows a partir do Linux: `rustup target add x86_64-pc-windows-gnu` e
`cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings`.
O `hmd-store` compila o SQLite em C e precisa do MinGW para esse alvo
(`sudo apt install gcc-mingw-w64-x86-64`); sem ele, use `--exclude hmd-store` e deixe o Windows
desse crate para o CI (ADR-023).
