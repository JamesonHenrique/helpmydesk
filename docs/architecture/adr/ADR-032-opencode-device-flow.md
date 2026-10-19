# ADR-032: conectar a IA pelo CLI do OpenCode (device-flow)

**Status:** aceito · **Data:** 2026-10-19

## Contexto

O fluxo aprovado da IA (spec 018) pedia que a pessoa copiasse e colasse chave. O CLI do
OpenCode oferece login por device code (`opencode auth login --method device`), que salva as
credenciais dele mesmo e dispensa colar chave nenhuma. O transporte subprocesso
(`hmd_ai::OpencodeTransport`) já estava previsto; o shell precisa escolher entre a chave
colada (HTTP) e o CLI (subprocesso).

## Decisão

- O login roda **exclusivamente** via `hmd_ai::opencode_command()` (helper único; só `hmd-ai`
  cria processos — constituição e guard `tools/check-no-process-spawn.sh` atualizados). O
  stdout/stderr do CLI é drenado por threads e a URL de verificação é aberta no navegador da
  pessoa assim que o CLI a imprime.
- Terminado o login com sucesso, uma chamada curta de teste (classify "teste de conexão") roda
  **fora do lock** do backend; só com ela verde o marcador `hmd_ai::DEVICE_KEY`
  (`opencode-cli-device`) entra no cofre e o consentimento `ai_opencode` é ligado.
- Com o marcador no cofre, o backend usa o transporte subprocesso
  (`Backend::ai_use_device_transport`); no próximo start, `Backend::ai_restore_transport`
  recompõe o modo a partir do cofre (CLI ou HTTP). "Desligar e apagar o código" apaga o
  marcador e o consentimento (pipeline existente).
- O transporte HTTP com chave colada (ADR-027, ADR-028) continua sendo o caminho principal;
  o device-flow é alternativa com a mesma consent e auditoria (as chamadas do subprocesso
  também passam por `ai_prepare`/`ai_finish` no lock).

## Consequências

- O marcador no cofre não é segredo (não é uma chave de API) — apenas sinaliza o modo.
- Não há como ler a sessão do CLI por API: se o CLI expirar o device code, o erro volta como
  `OPENCODE_FAILED` e a pessoa tenta de novo.
- O caminho do CLI (`opencode`) é descoberto em runtime (PATH e locais por-OS); nada do CLI
  entra no pacote (instalador) — dependência de runtime documentada no manual.
- Falta verificação em máquina real com conta real; coberto por testes unitários e do shell.
