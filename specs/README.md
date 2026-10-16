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
| [016-adversarial-suite](016-adversarial-suite/spec.md) | F5-SEC (parte) | Implementada (suíte adversarial na fronteira do IPC) |
| [017-ai-client](017-ai-client/spec.md) | SPEC-009 | Implementada (núcleo; falta a chave de quem roda para a avaliação real) |
| [018-ai-onboarding](018-ai-onboarding/spec.md) | SPEC-015, SPEC-010 (parte) | Implementada ("ligar a ajuda inteligente"; falta o teste com a conta real) |
| [019-killswitch-download](019-killswitch-download/spec.md) | SPEC-014a | Implementada (download do arquivo assinado; falta a chave e a release) |
| [020-elevated-helper](020-elevated-helper/spec.md) | SPEC-007 | Parcial (helper, caminho de elevação, lançador UAC por PowerShell e helper no instalador do Windows prontos; falta a allowlist depois do S-02 e o teste manual, T012) |

Próximas, na ordem do plano: allowlist de produção do helper e correção real (dependem do spike S-02 em máquinas Windows e Ubuntu/Mint), cliente de IA avaliado de verdade (SPEC-009/010, dependem da consulta Q-01 ao OpenCode) e assinatura dos instaladores (F5-SIGN).
