# Architecture Decision Records

Formato: MADR simplificado. Os ADRs ainda sem arquivo próprio estão resumidos no
[plano v1, Parte XI](../../planning/PLANEJAMENTO-v1.md) e ganham arquivo quando forem revisados.

| ADR | Título | Status |
|---|---|---|
| 001 | Plataformas: Windows 10 22H2/11 x64 + Linux x64 (Ubuntu LTS, Mint, Debian 13) | Aceita (plano XI) |
| 002 | Distribuição: GitHub Releases; NSIS por máquina; AppImage + .deb | Aceita (plano XI) |
| 003 | Local-only, sem servidor do projeto | Aceita (plano XI) |
| 004 | Tauri v2 + Rust + Angular | Aceita (plano XI) |
| 005 | Backend: não existe no MVP | Aceita (plano XI) |
| 006 | Persistência: SQLite local | Aceita (plano XI) |
| [007](ADR-007-ai-opencode.md) | IA: OpenCode Zen gratuito com a conta da pessoa | Aceita |
| 008 | Privilégio: helper efêmero via UAC; polkit/D-Bus no Linux | Aceita (plano XI) |
| 009 | Credenciais no cofre do SO; sem contas HelpMyDesk | Aceita (plano XI) |
| 010 | Comunicação: HTTPS só para OpenCode e GitHub | Aceita (plano XI) |
| 011 | Aprovação: N0–N4, padrão "Perguntar sempre" | Aceita (plano XI) |
| 012 | Sem telemetria | Aceita (plano XI) |
| 013 | Retenção local | Aceita (plano VII.4) |
| 014 | Atualizações: updater do Tauri + kill switch assinado | Aceita (plano XI) |
| 015 | SDD: Spec Kit v1.2.0, `--integration claude`, Living Spec; CI Windows + Linux | Aceita |
| 017 | Catálogo embutido e assinado | Aceita (plano XI) |
| [018](ADR-018-license.md) | Licença GPL-3.0-or-later / CC BY-SA 4.0 | Aceita |
| 019 | Governança: CODEOWNERS, DCO, releases só pelo CI | Aceita (CONTRIBUTING.md) |
| [020](ADR-020-p1-analysis.md) | Análise das pendências P1 e autorização para iniciar | Aceita |
| [021](ADR-021-catalog-format-and-probe-deps.md) | Catálogo em TOML; `sysinfo` para sondas | Aceita |
| [022](ADR-022-network-and-registry-deps.md) | `netdev` e `winreg` para sondas de rede e inicialização | Aceita |
| [023](ADR-023-local-store-sqlite.md) | Histórico local em SQLite (`rusqlite` com `bundled`) | Aceita |
| [024](ADR-024-plan-hash-sha2.md) | Hash do plano e chave de idempotência com `sha2` | Aceita |
| [025](ADR-025-tauri-shell.md) | App desktop com Tauri 2, workspace próprio, permissões por janela | Aceita |
| [026](ADR-026-kill-switch-signature.md) | Kill switch assinado com Ed25519 (`ed25519-dalek`) | Aceita |
| [027](ADR-027-ai-client-https.md) | Cliente de IA por HTTPS com `ureq`; allowlist de modelos | Aceita |
| [028](ADR-028-ai-key-vault-and-plugins.md) | Chave da IA no cofre do SO e plugins de IA | Aceita |
| [029](ADR-029-test-installers.md) | Instaladores de teste pelo CI (`.deb` + NSIS) | Aceita |
| [030](ADR-030-kill-switch-download.md) | Download do kill switch assinado | Aceita |
| [031](ADR-031-elevated-helper.md) | Helper elevado em processo separado (UAC, instalação por máquina) | Aceita (sem lançador, ver ADR) |
| [032](ADR-032-opencode-device-flow.md) | Conectar a IA pelo CLI do OpenCode (device-flow, transporte subprocesso) | Aceita |
| [033](ADR-033-ai-verification-codes.md) | Códigos de verificação na classificação da IA (re-classificação após o diagnóstico) | Aceita |
