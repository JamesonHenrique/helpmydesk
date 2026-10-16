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

`UacLauncher` (T007, decidido em 16/10): Windows PowerShell pelo caminho absoluto, `-NoProfile`,
script fixo em `-EncodedCommand` com `Start-Process -Verb RunAs -WindowStyle Hidden -Wait
-PassThru`; resultado pelo código de saída, prazo duro do chamador, e `unknown` para qualquer código
fora do contrato. Detalhes e custos no ADR-031. O desenho anterior (pipe nomeado com
`BCryptGenRandom`, `ShellExecuteExW` e `GetNamedPipeClientProcessId`) fica registrado para quando
um código de saída não bastar; ele exige `unsafe` ou dependência.

## Protocolo `hmd-elevated@1`

Um único token de argv: `base64url` (alfabeto URL-safe, com padding) do JSON UTF-8 do pedido
(`protocol`, `plan`, `plan_hash`, `step_id`, `capability`, `capability_version`, `platform`,
`params`, `expires_at`, `result_pipe` opcional). Sem `variant_id`: a variante é a do sistema em que
o helper roda, e o helper exige que o `platform` do pedido seja esse sistema — o par
`(capability, versão)` fica inequívoco e o plano (que é o que o hash cobre) não ganha um campo que
ele não tem. Base64url evita as regras de citação do `CommandLineToArgvW` do Windows
(só `[A-Za-z0-9_-]`, sem espaço, aspas ou barra) e o limite de 4 KiB vale sobre o JSON decodificado,
conferido no token antes de decodificar. `deny_unknown_fields`, esquema fechado, nenhum segredo. O
decodificador só aceita a grafia que ele mesmo escreve: um token que decodifica para os mesmos bytes
com outro padding é recusado, para que não existam duas formas do mesmo pedido.

O helper (SEC-023) não confia em nenhum campo recebido:

1. confere o próprio caminho (`HELPER_PATH_NOT_TRUSTED`), antes de olhar a linha de comando;
2. protocolo (`PROTOCOL_MISMATCH`), tamanho do token (`REQUEST_TOO_LARGE`), prazo vencido
   (`REQUEST_EXPIRED`), prazo maior que uma aprovação (`REQUEST_DEADLINE_TOO_LONG`) e a forma do
   `result_pipe` (`BAD_REQUEST`);
3. `plan` precisa ter o cabeçalho `hmd-plan@1` e contém a linha `step=` e as linhas `param=` e
   `verdict=` do passo pedido, com a aridade exata que o executor escreve; divergência →
   `PLAN_HASH_MISMATCH` (recalcula `sha256` do texto);
4. a linha `verdict=` do passo precisa dizer `accepted` **e** elevação `1` — um veredito sem
   elevação é um plano que não pede privilégio nenhum (`PLAN_HASH_MISMATCH`);
5. a versão do catálogo do plano tem de ser a deste build (`CATALOG_MISMATCH`): a pessoa aprovou os
   riscos e a verificação que o catálogo dizia naquele momento;
6. revalida `(capability, versão)` contra o catálogo embutido: existe, tem `requires_elevation:
   true`, a plataforma do pedido é a do processo, a variante existe e os params cabem no
   `params_schema` → senão `NOT_IN_ALLOWLIST` / `PLATFORM_NOT_SUPPORTED` / `INVALID_PARAMS`;
7. executa **uma** ação da allowlist compilada, sem shell, sem rede, sem stdin, sem ambiente
   herdado; resultado em JSON no stdout (teto de 512 bytes, com código substituto) + código de
   saída, que o processo de usuário traduz de volta para o diário (FR-016).

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
| 18 | `REQUEST_DEADLINE_TOO_LONG` (o token pediu mais prazo que uma aprovação) |
| 19 | `CATALOG_MISMATCH` (o plano é de outra versão do catálogo) |
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

Desinstalação: o template do NSIS apaga cada arquivo que instalou (app, sidecars, recursos) e
depois a pasta vazia, então o helper vai junto (FR-012). O helper não grava chave de registro
nenhuma, e a caixa "Remover dados do programa" do template cobre só o histórico por usuário
(F2-UNINST da spec 008).

Onde o helper entra no pacote (T015): sidecar (`bundle.externalBin`), que o Tauri coloca ao lado
do executável principal e apaga na desinstalação (lista `binaries` do template NSIS). O sidecar
fica em `windows/helper.conf.json`, mesclado com `--config` só no `tauri build` do Windows: o
`tauri-build` exige o arquivo do sidecar já em `binaries/` (com o sufixo do alvo), e assim o
`cargo build`/`cargo test` do shell continuam sem depender do helper. O job de instaladores compila
o helper em release, copia com o nome do alvo e confere com `7z l` que ele está no instalador.

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
- **Hash não é autenticação**: o helper recalcula o hash do plano que recebeu e o compara com o
  hash do mesmo token, o que prova que o texto não mudou, não que ele é o plano aprovado. O que
  liga o pedido a uma aprovação é o caminho do executor (plano preso ao ticket, nonce único,
  chave de idempotência única no diário) mais o prazo, e esse limite está declarado no plano v1.
  O que dá para fechar aqui já foi fechado: o token não escolhe o próprio prazo (SEC-029).
- **Valor de parâmetro não validado**: o esquema do catálogo valida nome e tamanho, não domínio. A
  primeira entrada da allowlist só entra sem parâmetros até existir validação de valor (FR-005).
- **Superfície do binário privilegiado**: o helper liga `hmd-executor` (para o trait `Elevation`) e
  com ele chega a `hmd-core` e `hmd-probes` (`netdev`, `sysinfo`) num processo que roda como
  administrador. Dá para separar quando o `Elevation` sair do executor; até lá, nenhuma ação real
  existe para explore.
- **`trusted_path` falha fechado**: no Windows a comparação de caminho é por componente e sensível
  a caixa; um caminho resolvido de outro jeito (nome curto `PROGRA~1`) faz o helper recusar a si
  mesmo. Falha para o lado seguro, mas precisa do teste no Windows real (T012).

## Decisões registradas

- Instalação **por máquina** (app + helper), conforme ADR-002 do plano v1.
- Pedido em `argv` como token base64url (plano v1 SEC-022), não em arquivo nem variável de ambiente.
- `hmd-canonical` novo crate para o hash compartilhado; `hmd-executor` reexporta (sem quebrar API).
- Linux sem `pkexec` nesta spec (FR-015): L1 é a preferida e o S-02 precisa medir antes.
- Prazo duro de 120 s e limite de 4 KiB são propostas; mudam-se em uma linha se o teste manual
  em Windows mostrar atrito.