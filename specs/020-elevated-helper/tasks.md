# Tarefas — spec 020

- [x] T001 Spec, plano e tarefas (spec 020, decisão de instalação por máquina)
- [ ] T002 ADR-031: helper elevado (binário separado, instalação por máquina, pedido em argv,
      sem `pkexec` nesta versão, `windows-sys` só se preciso)
- [ ] T003 `hmd-canonical`: `canonical.rs` movido do executor, reexportado (hash único para
      executor e helper); `cargo deny` e lints limpos
- [ ] T004 `hmd-elevated` lib: pedido estrito (`deny_unknown_fields`, base64url, 4 KiB),
      revalidação (hash, cabeçalho `hmd-plan@1`, `verdict=accepted` + elevação 1), allowlist
      compilada vazia, `plan_text` parser, códigos de saída (SEC-020…SEC-027)
- [ ] T005 `hmd-elevated` bin: confere o próprio caminho, executa uma ação, JSON no stdout,
      sem shell/rede/stdin/ambiente
- [ ] T006 `hmd-executor`: `Launcher` + `FakeLauncher` + `UnavailableLauncher` (Linux);
      `ELEVATION_NOT_AVAILABLE` antes do `started`; timeout → `unknown`
- [ ] T007 `UacLauncher` em `cfg(windows)` (`ShellExecuteExW` `runas`), token base64url,
      leitura do stdout com limite
- [ ] T008 Testes citando IDs (FR-/SEC-): pedido forjado, allowlist furada, hash adulterado,
      caminho não confiável, timeout, recusa sem admin
- [ ] T009 App: aviso pré-elevação antes do UAC + motivo de `ELEVATION_NOT_AVAILABLE` na tela
- [ ] T010 Instalador: NSIS por máquina com o helper em pasta só-admin; desinstalação remove
      helper e sobras do registro (FR-011, FR-012)
- [ ] T011 `tools/check-all.sh` verde, CI com o bin do helper, `specs/README.md` e
      `docs/status/roadmap.json` atualizados
- [ ] T012 Teste manual no Windows (janela de aprovação + UAC) e registro no relatório do spike
- [ ] T013 Allowlist de produção depois do S-02 (fora desta entrega)