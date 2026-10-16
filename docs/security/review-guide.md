# Guia para a revisão de segurança independente

Estado em 16/10/2026. Este guia serve para quem vai revisar o HelpMyDesk sem ter escrito o código.
A suíte adversarial (spec 016) foi escrita pelo mesmo autor das defesas. Uma primeira revisão
independente foi feita por outro modelo (15/10); uma revisão humana externa continua desejável.
O helper de elevação (spec 020) é novo e **ainda não foi revisado por ninguém**.

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
| Nada privilegiado fora do caminho de elevação (`ELEVATION_NOT_AVAILABLE` antes do diário) | `crates/hmd-executor/src/lib.rs` (`Elevation`, `execute_step`) | `crates/hmd-executor/tests/executor.rs` |
| Helper revalida tudo e só roda o que está na allowlist compilada (SEC-020…SEC-024, SEC-028) | `crates/hmd-elevated/src/lib.rs` (`validate`, `run_request`), `crates/hmd-canonical` | `crates/hmd-elevated/tests/protocol.rs` |
| Helper recusa rodar fora da pasta só-admin; instalador é por máquina (FR-011, FR-012) | `crates/hmd-elevated/src/lib.rs` (`trusted_path`, `INSTALL_DIR`), `apps/desktop/src-tauri/tauri.conf.json` | `crates/hmd-elevated/tests/install.rs`, `crates/hmd-elevated/tests/protocol.rs` |
| Helper é efêmero: uma ação por invocação, JSON no stdout, prazo duro (SEC-025…SEC-027, FR-010) | `crates/hmd-elevated/src/lib.rs`, `crates/hmd-elevated/src/main.rs` | `crates/hmd-elevated/tests/protocol.rs`, `crates/hmd-elevated/tests/binary.rs` |

Rodar tudo: `tools/check-all.sh` (o mesmo que o CI).

## Pontos abertos que pedem atenção

- **Risco aceito pelo mantenedor (15/10):** `confirm` pode ser chamado pela janela principal.
  Uma interface comprometida marca "resolvido", mas só depois de uma correção verificada; não
  executa nada.
- Resolvidos em 15/10: exportação forjada (comando `report_exported` removido), purga com salto
  de relógio (espera uma sessão normal), validade da aprovação também por relógio monotônico,
  saída de `unknown` pela sonda de verificação, `erase_all` que falha reabre no comando seguinte.
- A saída de `unknown` só marca "não aplicada" (e permite propor de novo, com aprovação nova)
  correções com `idempotent = true` no catálogo; as demais continuam bloqueadas até a sonda ver o
  problema resolvido. Ao liberar uma correção real, confira se o `idempotent` dela é verdadeiro.
- O registro de correções de produção está vazio (`Registry::production()`). Revise de novo quando
  a primeira correção real entrar (depende do spike S-02 e da elevação, SPEC-007).
- A **allowlist do helper** (`Allowlist::production`) também nasce vazia: enquanto for assim, a
  elevação recusa com `ELEVATION_NOT_AVAILABLE` e nada privilegiado acontece. Ela cresce por
  revisão de segurança, não por conveniência (SEC-021, ADR-031).
- O **lançador no Windows** (`crates/hmd-elevated/src/uac.rs`) chama o PowerShell com um script
  fixo e dois buracos validados (ADR-031). Ao mexer nele, confira: os buracos continuam recusando
  aspas, espaço, `$` e acento; nenhum código fora do contrato vira sucesso; o PowerShell continua
  pelo caminho absoluto e sem perfil.
- Ainda não foram revisados: o updater, a assinatura dos binários e o cliente de IA.

## Histórico

Revisão independente (modelo Opus, contexto limpo, só leitura) feita em 15/10/2026; achados e correções na rodada 7.

Os achados da spec 016 e as correções estão em
[`specs/016-adversarial-suite/spec.md`](../../specs/016-adversarial-suite/spec.md).
