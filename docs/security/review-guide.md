# Guia para a revisão de segurança independente

Estado em 15/10/2026. Este guia serve para quem vai revisar o HelpMyDesk sem ter escrito o código.
A suíte adversarial (spec 016) foi escrita pelo mesmo autor das defesas, então **não substitui**
esta revisão (roadmap F5-SEC).

## O que proteger

1. Nenhuma correção roda sem a aprovação da pessoa para **aquele** plano, **daquele** ticket,
   dentro da validade e uma única vez (INV-04).
2. Só correções do catálogo rodam, e o catálogo decide o nível (INV-03, INV-05, INV-06).
3. Nada que identifique a pessoa (nome de usuário, nome do computador, evidência bruta das
   sondas) chega à tela, ao relatório ou à exportação.
4. Nenhuma entrada da interface ou do sistema derruba o núcleo.

INV-01 e INV-02 (IA) ainda não têm código: não existe cliente de IA (SPEC-009).

## Onde cada coisa é garantida e testada

| Garantia | Código | Testes |
|---|---|---|
| Plano preso ao hash, aprovação com nonce, validade de 10 min, uso único | `crates/hmd-executor/src/lib.rs` (`check_approval`, `execute_step`) | `crates/hmd-executor/tests/executor.rs` |
| Plano preso ao ticket; "só observar"; kill switch; passos oferecidos | `crates/hmd-app/src/lib.rs` (`approve`, `execute`, `take_pending`, `apply_kill_switch`) | `crates/hmd-ipc/tests/adversarial.rs`, `crates/hmd-app/tests/killswitch.rs` |
| Write-ahead (`started` antes de rodar), `unknown` no startup, `started`/`unknown` bloqueiam | `crates/hmd-store/src/journal.rs` | `crates/hmd-store/tests/journal.rs` |
| Política determinística (níveis, consentimento, limites) | `crates/hmd-policy` | `crates/hmd-policy/tests/policy.rs` |
| Catálogo fechado e validado (campos desconhecidos, verificação que não vê o problema) | `crates/hmd-catalog/src/lib.rs` | `crates/hmd-catalog/tests/catalog.rs` |
| Só a janela `approval` aprova; a principal não aprova nem emite eventos | `apps/desktop/src-tauri/capabilities/*.json` | testes em `apps/desktop/src-tauri/src/lib.rs` |
| "Sim" só depois de 1 s (clique sequestrado) | `apps/desktop/ui/src/app/pages/approval.ts` | `apps/desktop/ui/src/app/app.spec.ts` |
| Limpeza e redação de texto não confiável | `crates/hmd-probes/src/lib.rs` (`sanitize_untrusted`, `redact_names`) | testes do módulo, `crates/hmd-core/tests/report.rs` |
| Auditoria encadeada | `crates/hmd-store/src/audit.rs` | `crates/hmd-store/tests/privacy.rs` |
| Kill switch assinado (Ed25519, mais novo, não vencido) | `crates/hmd-killswitch` | testes do crate |

Rodar tudo: `tools/check-all.sh` (o mesmo que o CI).

## Pontos abertos que pedem atenção

- `confirm` e `report_exported` podem ser chamados pela janela principal: uma interface
  comprometida marca "resolvido" (só depois de uma correção verificada) ou registra uma exportação
  falsa na auditoria.
- A limpeza da auditoria usa o relógio do sistema: um salto de mais de 180 dias para a frente apaga
  os eventos, e a cadeia continua válida como prefixo cortado.
- A validade da aprovação também usa o relógio de parede.
- `clarify_unknown` não tem chamador: uma capacidade em `unknown` fica bloqueada para sempre.
- `erase_all`: se apagar ou reabrir falhar, o backend fica `NOT_OPEN` até o app reiniciar.
- O registro de correções de produção está vazio (`Registry::production()`). Revise de novo quando
  a primeira correção real entrar (depende do spike S-02 e da elevação, SPEC-007).
- Ainda não foram revisados: o helper de elevação (não existe), o updater, a assinatura dos
  binários e o cliente de IA.

## Histórico

Os achados da spec 016 e as correções estão em
[`specs/016-adversarial-suite/spec.md`](../../specs/016-adversarial-suite/spec.md).
