# Plano — spec 020

**Branch**: `feat/elevated-helper` | **Spec**: [spec.md](spec.md)

## Technical Context

Crate novo `crates/hmd-elevated` (lib + bin `helpmydesk-elevated`), dependente de `hmd-catalog`
(catálogo embutido, puro) e de um novo crate puro `hmd-canonical`, que recebe o `canonical.rs`
de `hmd-executor` para que executor e helper calculem o hash **com o mesmo código** (divergir
seria um furo). `hmd-executor` reexporta, então `hmd-store` e `hmd-app` não mudam.

Lançador em `hmd-elevated` (único outro lugar aceito por `tools/check-no-process-spawn.sh`):
`trait Launcher { fn launch(&self, req: &ElevatedRequest, timeout) -> Result<ElevatedResult,
LaunchError> }`. O pedido inteiro vai para o lançador, não o token: o lançador precisa poder
acrescentar o transporte dele (`result_pipe`, nome de pipe nomeado do Windows) **antes** de
serializar. Com `FakeLauncher` nos testes e `UacLauncher` só em `cfg(windows)`. No Linux,
`UnavailableLauncher` responde `ELEVATION_NOT_AVAILABLE` até o S-02 decidir entre a API D-Bus
(L1) e `pkexec` (L2).

`UacLauncher` está **bloqueado por uma decisão de constituição**: `ShellExecuteExW` (`runas`) é
`unsafe` e o workspace é `unsafe_code = "forbid"`. Desenho fechado e revisado aqui para não se
perder: pipe nomeado com 128 bits de entropia de `BCryptGenRandom` e `PIPE_REJECT_REMOTE_CLIENTS`;
`ShellExecuteExW` com `SEE_MASK_NOCLOSEPROCESS` e `nShow = SW_HIDE` (sem console na tela da
pessoa, FR-014); espera em thread de trabalho com o prazo duro (o diálogo do UAC responde a
nenhuma espera nossa, então o prazo é do chamador e o helper já carrega `expires_at`); a resposta
só é aceita se `GetNamedPipeClientProcessId` for o PID do processo lançado, senão
`ELEVATION_RESULT_NOT_TRUSTED`. Como a allowlist de produção está vazia, mesmo que o lançador
funcionasse hoje nenhuma ação rodaria.

## Protocolo `hmd-elevated@1`

Um único token de argv: `base64url` (alfabeto URL-safe, com padding) do JSON UTF-8 do pedido
(`protocol`, `plan`, `plan_hash`, `step_id`, `capability`, `capability_version`, `variant_id`,
`platform`, `params`, `expires_at`, `result_pipe` opcional). Base64url evita as regras de citação do `CommandLineToArgvW` do Windows
(só `[A-Za-z0-9_-]`, sem espaço, aspas ou barra) e o limite de 4 KiB vale sobre o JSON decodificado.
`deny_unknown_fields`, esquema fechado, nenhum segredo.

O helper (SEC-023) não confia em nenhum campo recebido:

1. confere o próprio caminho (`HELPER_PATH_NOT_TRUSTED`);
2. parse estrito e limite de tamanho (`REQUEST_TOO_LARGE`) e prazo (`REQUEST_EXPIRED`);
3. `plan` precisa ter o cabeçalho `hmd-plan@1` e contém a linha `step=` e as linhas `param=` e
   `verdict=` do passo pedido; divergência → `PLAN_HASH_MISMATCH` (recalcula `sha256` do texto);
4. a linha `verdict=` do passo precisa dizer `accepted` **e** elevação `1`;
5. revalida `(capability, versão, variant_id)` contra o catálogo embutido: existe, tem
   `requires_elevation: true`, existe para a plataforma atual e os params cabem no
   `params_schema` → senão `NOT_IN_ALLOWLIST` / `PLATFORM_NOT_SUPPORTED` / `INVALID_PARAMS`;
6. executa **uma** ação da allowlist compilada, sem shell, sem rede, sem stdin, sem ambiente
   herdado; resultado em JSON no stdout + código de saída.

| código | significado |
|---|---|
| 0 | `succeeded` |
| 10 | `REQUEST_TOO_LARGE` |
| 11 | `BAD_REQUEST` (JSON/campo desconhecido/protocolo diferente) |
| 12 | `NOT_IN_ALLOWLIST` |
| 13 | `PLAN_HASH_MISMATCH` |
| 14 | `PLATFORM_NOT_SUPPORTED` |
| 15 | `INVALID_PARAMS` |
| 16 | `HELPER_PATH_NOT_TRUSTED` |
| 17 | `REQUEST_EXPIRED` |
| 20 | `failed` (a ação correu e não deu certo) |
| 21 | `unknown` (a ação não sabe se aconteceu) |

## Ordem no executor

`execute_step` (spec 008, FR-003) ganha um passo só: se `step.requires_elevation`, o passo só
passa pelo lançador; sem implementação elevada → `ELEVATION_NOT_AVAILABLE` **antes** de
qualquer escrita no diário. Mantém o write-ahead: `started` primeiro, UAC depois. Prazo duro de
120 s (inclui a espera pelo UAC) → `unknown` com `ELEVATION_TIMEOUT`; nunca re-executado
(continua valendo `UNKNOWN_PENDING` da spec 008). A verificação continua no processo de usuário,
com a sonda somente leitura do catálogo.

## Instalador (FR-011, FR-012, SEC-020)

O instalador NSIS é **por máquina**: `bundle.windows.nsis.installMode = "perMachine"` em
`apps/desktop/src-tauri/tauri.conf.json` grava em `Program Files` e em `HKLM`, e o Windows pede
conta de administrador antes de começar. O helper (e o app) ficam em `$INSTDIR`, a pasta que
`trusted_path` aceita; qualquer outro caminho devolve `HELPER_PATH_NOT_TRUSTED`. `hmd-elevated::INSTALL_DIR`
é a única fonte dessa pasta e um teste (`crates/hmd-elevated/tests/install.rs`) lê o
`tauri.conf.json` de verdade, para o instalador e o helper não divergirem em silêncio.

Desinstalação: o template do NSIS remove `$INSTDIR` inteiro, então o helper vai com a pasta
(FR-012). O helper não grava chave de registro nenhuma, e o hook existente continua cobrindo só o
histórico por usuário (F2-UNINST da spec 008).

Onde o helper entra no pacote: sidecar (`bundle.externalBin`), que o Tauri coloca ao lado do
executável principal. Isso exige o binário do helper compilado para a plataforma alvo no momento do
`tauri build`, então entra junto com o lançador (T007), que está bloqueado por decisão de
constituição: até lá o instalador não traz helper nenhum e a elevação responde
`ELEVATION_NOT_AVAILABLE`, que é a recusa segura.

## Constitution Check

| Princípio | Verificação | Resultado |
|---|---|---|
| I | O caminho de elevação vem de `requires_elevation` do catálogo assinado; IA e plano não escolhem | PASS |
| II | Só `hmd-elevated` roda como admin, efêmero, com allowlist própria e revalidação | PASS |
| III | Verificação é sonda do catálogo no processo de usuário; `unknown` não vira sucesso | PASS |
| VI | Todo FR-/SEC- com teste citando o ID | PASS |
| VII | Sem serviço residente, sem daemon; `windows-sys` só se a API do sistema exigir | PASS |

## Riscos aceitos

- **Limite declarado** (plano v1): malware como o usuário pode chamar o helper com qualquer ação
  da allowlist e depender de um clique no UAC, que mostra o editor e não a ação. O dano é limitado
  pela allowlist pequena e inofensiva por desenho; por isso a allowlist de produção fica **vazia**
  até o S-02 e só cresce com revisão de segurança.
- Instalação por máquina (ADR-002): conta padrão sem senha de admin nem instala (Q-10 do plano).
- Sem assinatura ainda (F5-SIGN), o UAC mostra "editor desconhecido"; motivo para teste manual,
  não bloqueio de código.

## Decisões registradas

- Instalação **por máquina** (app + helper), conforme ADR-002 do plano v1.
- Pedido em `argv` como token base64url (plano v1 SEC-022), não em arquivo nem variável de ambiente.
- `hmd-canonical` novo crate para o hash compartilhado; `hmd-executor` reexporta (sem quebrar API).
- Linux sem `pkexec` nesta spec (FR-015): L1 é a preferida e o S-02 precisa medir antes.
- Prazo duro de 120 s e limite de 4 KiB são propostas; mudam-se em uma linha se o teste manual
  em Windows mostrar atrito.