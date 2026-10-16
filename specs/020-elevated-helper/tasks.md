# Tarefas — spec 020

- [x] T001 Spec, plano e tarefas (spec 020, decisão de instalação por máquina)
- [ ] T002 ADR-031: helper elevado (binário separado, instalação por máquina, pedido em argv,
      sem `pkexec` nesta versão, `windows-sys` só se preciso)
- [x] T003 `hmd-canonical`: `canonical.rs` movido do executor, reexportado (hash único para
      executor e helper); `cargo deny` e lints limpos
- [x] T004 `hmd-elevated` lib: pedido estrito (`deny_unknown_fields`, base64url, 4 KiB),
      revalidação (hash, cabeçalho `hmd-plan@1`, `verdict=accepted` + elevação 1), allowlist
      compilada vazia, `plan_text` parser, códigos de saída (SEC-020…SEC-027)
- [x] T005 `hmd-elevated` bin: confere o próprio caminho, executa uma ação, JSON no stdout,
      sem shell/rede/stdin/ambiente
- [x] T006 `hmd-executor`: `Launcher` + `FakeLauncher` + `UnavailableLauncher` (Linux);
      `ELEVATION_NOT_AVAILABLE` antes do `started`; timeout → `unknown`
- [ ] T007 **bloqueado, decisão de constituição**: o `UacLauncher` precisa de `ShellExecuteExW`, e
      o workspace tem `[workspace.lints.rust] unsafe_code = "forbid"` (o repositório inteiro é
      zero-`unsafe`). São três caminhos para o mantenedor decidir: (a) amendamento da constituição
      permitindo `unsafe` só em `hmd-elevated`, com ADR e revisão de cada chamada; (b) crate
     Community que embrulha a chamada com API segura (revisão de cadeia de suprimento); (c) manter
      sem lançador até existir máquina Windows de teste (o estado de hoje: recusa segura).
      Desenho pronto e anotado no plano; o pedido do `Launcher` passou a levar o `ElevatedRequest`
      inteiro para o transporte poder entrar no token
- [x] T008 Testes citando IDs (FR-/SEC-): pedido forjado, allowlist furada, hash adulterado,
      caminho não confiável, timeout, recusa sem admin
- [x] T009 App: aviso pré-elevação na janela de aprovação (UAC e o que acontece se a pessoa
      recusar) e tela de resultado para `elevation_not_available` (`Nada foi alterado` + pedir a
      instalação por máquina)
- [x] T010 Instalador: NSIS **por máquina** (`installMode: perMachine`) com o helper em pasta
      só-admin; desinstalação limpa leva a pasta (e o helper) junto; `INSTALL_DIR` como fonte
      única, com teste que lê o `tauri.conf.json` real (`crates/hmd-elevated/tests/install.rs`).
      O helper entra no pacote como sidecar quando o lançador existir (T007, bloqueado)
- [ ] T011 `tools/check-all.sh` verde, CI com o bin do helper, `specs/README.md` e
      `docs/status/roadmap.json` atualizados
- [ ] T012 Teste manual no Windows (janela de aprovação + UAC) e registro no relatório do spike
- [ ] T013 Allowlist de produção depois do S-02 (fora desta entrega)