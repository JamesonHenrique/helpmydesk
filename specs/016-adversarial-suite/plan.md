# Plano — spec 016

- Teste em `crates/hmd-ipc/tests/adversarial.rs`: tabela `ATTACKS` (nome, função), laboratório
  com relógio, correção falsa contadora e sonda de DNS que deixa de falhar quando a correção roda
  ou quando o teste diz que o problema sumiu sozinho.
- Correções mínimas em `crates/hmd-app/src/lib.rs`: `take_pending`, checagem de modo no
  `execute`, `pending.retain` no começo do `diagnose`, validação dos passos no `approve`; dois
  códigos novos (`STEP_NOT_IN_PLAN`, `OBSERVE_ONLY`).
- A UI já não deixa aprovar sem passo marcado; nada muda para a pessoa.
