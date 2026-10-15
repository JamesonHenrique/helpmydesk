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

Próximas, na ordem do plano: janela de aprovação (SPEC-005, no app),
redação/privacidade (SPEC-011), governança de contribuição ao catálogo (SPEC-016).
