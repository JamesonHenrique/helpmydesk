# Tarefas — spec 020

- [x] T001 Spec, plano e tarefas (spec 020, decisão de instalação por máquina)
- [x] T002 ADR-031: helper elevado (binário separado, instalação por máquina, pedido em argv,
      sem `pkexec` nesta versão, `windows-sys` só se preciso) — inclui o bloqueio do `unsafe`
      (`unsafe_code = "forbid"`) e as três saídas para o mantenedor decidir
- [x] T003 `hmd-canonical`: `canonical.rs` movido do executor, reexportado (hash único para
      executor e helper); `cargo deny` e lints limpos
- [x] T004 `hmd-elevated` lib: pedido estrito (`deny_unknown_fields`, base64url, 4 KiB),
      revalidação (hash, cabeçalho `hmd-plan@1`, `verdict=accepted` + elevação 1), allowlist
      compilada vazia, `plan_text` parser, códigos de saída (SEC-020…SEC-027)
- [x] T005 `hmd-elevated` bin: confere o próprio caminho, executa uma ação, JSON no stdout,
      sem shell/rede/stdin/ambiente
- [x] T006 `hmd-executor`: `Launcher` + `FakeLauncher` + `UnavailableLauncher` (Linux);
      `ELEVATION_NOT_AVAILABLE` antes do `started`; timeout → `unknown`
- [x] T007 `UacLauncher` por PowerShell (`Start-Process -Verb RunAs`), escolha do mantenedor em
      16/10 (ADR-031, "Lançador no Windows"): sem `unsafe` e sem dependência; script fixo em
      `-EncodedCommand`, resultado pelo código de saída, prazo duro, teste ponta a ponta no CI
      Windows com o maior token (`tests/uac.rs`)
- [x] T008 Testes citando IDs (FR-/SEC-): pedido forjado, allowlist furada, hash adulterado,
      caminho não confiável, timeout, recusa sem admin
- [x] T009 App: aviso pré-elevação na janela de aprovação (UAC e o que acontece se a pessoa
      recusar) e tela de resultado para `elevation_not_available` (`Nada foi alterado` + pedir a
      instalação por máquina)
- [x] T010 Instalador: NSIS **por máquina** (`installMode: perMachine`) com o helper em pasta
      só-admin; desinstalação limpa leva a pasta (e o helper) junto; `INSTALL_DIR` como fonte
      única, com teste que lê o `tauri.conf.json` real (`crates/hmd-elevated/tests/install.rs`).
      O helper entra no pacote como sidecar (T015)
- [x] T011 `tools/check-all.sh` verde (o job `rust` do CI já compila e testa o bin do helper nas
      duas plataformas), `specs/README.md`, `docs/status/roadmap.json` e o guia de revisão
      atualizados; `Cargo.lock` do shell regenerado (o `hmd-app` passou a depender do helper, e o
      CI usa `--locked`)
- [x] T014 Revisão adversarial da branch (leitura por outro agente, achados P1/P2/P3) e
      correções: plataforma do pedido tem de ser a do helper, versão do catálogo conferida, prazo
      máximo do token (SEC-029), `result_pipe` com uma só forma, token em grafia única, tamanho do
      resultado com teto real, parser do plano estrito (aridade, duplicidade, catálogo), recusa do
      helper chegando ao diário com o código dela (FR-016), ordem caminho→argv no binário, e texto
      da janela de aprovação sem prometer um UAC que esta build não tem
- [ ] T012 Teste manual no Windows (janela de aprovação + UAC) e registro no relatório do spike
- [ ] T013 Allowlist de produção depois do S-02 (fora desta entrega)
- [ ] T015 Helper no instalador do Windows (sidecar `bundle.externalBin`), com o CI compilando o
      helper antes do `tauri build`; até lá o lançador se declara indisponível
- [x] T016 Revisão independente (Opus, 16/10) da spec 020, correções que vêm antes do helper no
      instalador: o token do passo elevado vive no máximo um `ELEVATION_TIMEOUT`
      (`elevation_deadline`; um clique tardio no UAC encontra pedido vencido); o executor lê a
      allowlist do helper (passo fora dela não é oferecido, sem UAC e sem `started`); pedido que
      não pôde ser montado vira `failed ELEVATION_MALFORMED`; o helper lê o argv com `args_os` e
      recusa argumento extra ou que não seja Unicode (sem pânico)
- [x] T017 Achados baixos da revisão de 16/10: `trusted_path` exige a pasta exata e só
      componentes simples; substituto de resultado grande diz `unknown`; parser do plano no helper
      lê só o texto canônico (`\n`, ordem das seções, sem `catalog` ou `param` repetido); campo
      `operation` (só `run`) no pedido; o catálogo recusa sonda de verificação com id `-` (mesmo
      texto que "sem verificação"); hook de desinstalação troca para `SetShellVarContext current`
      antes de apagar o histórico (o template por máquina usa `all`, conferido no `installer.nsi`
      do Tauri 2.12.1); ADR e guia de revisão dizem que o hash dá integridade, não autenticidade
