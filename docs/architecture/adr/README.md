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
