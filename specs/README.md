# Especificações

Diretórios `NNN-nome` criados pelo Spec Kit. A numeração do Spec Kit é o ID canônico; a coluna
"Plano" mapeia para a lista de specs do planejamento v1 (Parte X).

| Spec | Plano | Status |
|---|---|---|
| [001-domain-ticket-lifecycle](001-domain-ticket-lifecycle/spec.md) | SPEC-003 (parte de domínio) | Implementada |
| [002-policy-engine](002-policy-engine/spec.md) | SPEC-004 | Implementada |
| [003-catalog](003-catalog/spec.md) | SPEC-001 (catálogo) | Implementada |
| [004-read-only-probes](004-read-only-probes/spec.md) | SPEC-001 (sondas, 1º lote) | Implementada (4 sondas) |
| [005-probes-batch-2](005-probes-batch-2/spec.md) | SPEC-001 (sondas, 2º lote) | Implementada (6 sondas) |
| [006-playbooks-basic-mode](006-playbooks-basic-mode/spec.md) | SPEC-002 | Implementada (5 playbooks + modo básico em texto) |
| [007-local-history-report](007-local-history-report/spec.md) | SPEC-003 | Implementada (histórico SQLite + relatório `report@1`) |
| [008-executor-approval-journal](008-executor-approval-journal/spec.md) | SPEC-006 + parte da SPEC-005 | Implementada (hash do plano, aprovação, diário, verificação; sem correção real até o S-02) |
| [009-privacy-consent-audit](009-privacy-consent-audit/spec.md) | SPEC-011 | Implementada ("o que eu olho", consentimento versionado, auditoria encadeada, exportar/apagar tudo) |
| [010-av-probes](010-av-probes/spec.md) | SPEC-001 (sondas, 3º lote) | Implementada (som, microfone e câmera; leitura do registro do Windows a confirmar no S-01) |
| [011-ui-prototype](011-ui-prototype/spec.md) | F1 (protótipo de telas) | Implementada (Angular, dados de exemplo, textos gerados do catálogo) |
| [012-app-service](012-app-service/spec.md) | Base do app desktop | Implementada (casos de uso que a interface chama) |
| [013-desktop-shell](013-desktop-shell/spec.md) | ADR-004 + SPEC-005 (janela de aprovação) | Implementada (Tauri ligado ao núcleo; compila no CI; falta teste manual) |
| [014-kill-switch](014-kill-switch/spec.md) | SPEC-014 (kill switch) | Implementada (verificação e aplicação; falta a chave do projeto e o download) |
| [015-catalog-governance](015-catalog-governance/spec.md) | SPEC-016 | Implementada (guia, modelos, CODEOWNERS, trava G-SEC no executor) |

Próximas, na ordem do plano: elevação UAC/polkit (SPEC-007, depende do spike S-02 em máquinas Windows e Ubuntu/Mint), cliente de IA e orquestração (SPEC-009/010, dependem da consulta Q-01 ao OpenCode), download do kill switch e updater (SPEC-014, F5).
