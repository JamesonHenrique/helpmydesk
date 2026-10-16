# ADR-031: helper elevado em processo separado

**Status:** aceito · **Data:** 2026-10-16 (lançador decidido no mesmo dia, ver "Lançador no Windows")

## Contexto

O catálogo já marcava correções com `requires_elevation` (a única hoje é `net.dns.flush_cache`), mas
o workspace não tinha como executar nada privilegiado: `Registry::production()` era vazio de
propósito. Sem elevação, essas correções só podiam aparecer como "ainda não disponível" para a
pessoa, o que joga fora a parte do produto que o plano v1 chama de vantagem (resolver sozinha o que
a pessoa pediria para um técnico).

Privilégio no desktop é o caso mais perigoso do produto: é o único lugar onde uma decisão errada
vira estrago no sistema inteiro, e é o único onde o "aprovado pela pessoa" pode ser forjado por
outro programa rodando como ela.

## Decisão

- **`crates/hmd-elevated`, binário separado** (`helpmydesk-elevated`), fora do app: o app continua
  como usuário, o helper sobe, faz **uma** ação e sai (SEC-027, nada residente, sem serviço, sem
  daemon).
- **Pedido em `argv`**: um token base64url com esquema estrito (`deny_unknown_fields`), limite de
  4 KiB, versão de protocolo (`hmd-elevated@1`) e o prazo da aprovação que o originou. Nada vem de
  arquivo, variável de ambiente ou stdin (SEC-022), e nada é corrigido depois de recebido.
- **O helper revalida tudo antes de agir** (SEC-023): recalcula o hash do plano, relê o texto
  canônico com o mesmo parser do executor (`hmd-canonical`), confere `accepted` e `requires_elevation`
  no veredito, procura a correção no catálogo embutido (versão, variante da plataforma, risco N4
  nunca) e compara os params com o esquema da variante.
- **Allowlist compilada, vazia nesta versão** (SEC-021). A allowlist de produção cresce só depois do
  S-02, com revisão de segurança; até lá a única resposta possível é
  `ELEVATION_NOT_AVAILABLE`, recusa limpa.
- **Instalação por máquina** (ADR-002): app e helper em `C:\Program Files\HelpMyDesk`, o Windows
  pede conta de administrador para instalar. O helper confere o próprio caminho antes de tudo e
  recusa rodar fora dessa pasta (`HELPER_PATH_NOT_TRUSTED`, SEC-020). Conta sem direito
  administrativo não instala e recebe orientação na tela (Q-10 do plano v1).
- **Resultado em JSON no stdout + código de saída estável** (SEC-025). Quem escreve no diário é o
  processo de usuário; o helper não conhece banco, plano, pessoa nem máquina.
- **Prazo duro de 120 s por invocação**, incluindo a espera pelo UAC; vencido, o passo vira
  `unknown` e nunca é re-executado (FR-010, SEC-026). O token carrega o prazo da aprovação, e o
  helper **não aceita um prazo maior que uma aprovação** (SEC-029): sem isso, um token capturado
  da linha de comando valeria para sempre.
- **A recusa do helper vira `failed` com o código dela** no diário (`NOT_IN_ALLOWLIST`,
  `HELPER_PATH_NOT_TRUSTED`, `PLAN_HASH_MISMATCH`, …), não `ELEVATION_NOT_AVAILABLE`: um evento de
  segurança que some do diário é pior que um estado conservative. `ELEVATION_NOT_AVAILABLE` fica
  reservado para "esta build não tem caminho de elevação" e acontece antes de qualquer escrita.
- **Linux sem `pkexec` nesta versão** (FR-015): a via preferida do plano v1 é L1 (API de sistema via
  D-Bus, sem elevate); o S-02 precisa medir se o catálogo tem o que oferecer antes de criar
  privilégio no Linux.
- **`hmd-canonical`** virou crate próprio para que executor e helper compartilhem *a mesma*
  codificação e o mesmo hash, sem cópia divergente.

## Lançador no Windows (decidido pelo mantenedor em 2026-10-16)
`ShellExecuteExW` (o "runas") só existe em FFI, e o workspace tem
`[workspace.lints.rust] unsafe_code = "forbid"`. A constituição não fala de `unsafe`: a proibição é
o lint, então a escolha era deste ADR. Das opções (liberar `unsafe` só no lançador, crate de
terceiros, PowerShell, esperar), o mantenedor escolheu **PowerShell**:
- `UacLauncher` (`crates/hmd-elevated/src/uac.rs`) roda o Windows PowerShell 5.1 pelo caminho
  absoluto da pasta do Windows, sem perfil, com `-EncodedCommand`. O script é texto fixo com dois
  buracos: o caminho do helper (constante da build) e o token (base64url). Qualquer outro byte num
  buraco recusa antes de o PowerShell abrir.
- `Start-Process -Verb RunAs -Wait -PassThru` é o mesmo pedido de UAC por uma API gerenciada.
  Zero `unsafe`, zero dependência nova.
- **O resultado volta só pelo código de saída**: um processo elevado pelo UAC não devolve o stdout
  ao processo de usuário. O código já era a parte estável do contrato (SEC-025): `0` deu certo,
  `20` falhou, `10`–`19` recusas com código, `21` incerto. O script acrescenta `1223` (a pessoa
  recusou o UAC → `ELEVATION_DENIED`) e `201` (o helper não abriu → `ELEVATION_NOT_AVAILABLE`; como o `started` já foi escrito, o
  diário registra `unknown` com esse código, a leitura conservadora do executor).
  Qualquer outro código, ou nenhum, vira `unknown` e nunca é repetido (FR-010); `exit $null` é
  tratado à parte para não virar `0`.
- Prazo duro: o lançador espera o PowerShell até o prazo e o encerra depois; o passo vira
  `unknown`. Um helper que já subiu como administrador não pode ser parado daqui.
- `available()` só é verdade no Windows **e** com o helper em `INSTALL_DIR`. Até o helper entrar
  no instalador (sidecar, spec 020 T010), toda elevação continua `ELEVATION_NOT_AVAILABLE` antes
  de qualquer escrita.
- Teste ponta a ponta no CI Windows (`crates/hmd-elevated/tests/uac.rs`, `HMD_UAC_E2E=1`): copia o
  helper para `Program Files`, manda o maior token aceito e recebe `PLAN_HASH_MISMATCH`, o que
  prova que o token cruzou a linha de comando inteiro.

Custos aceitos: o detalhe em JSON se perde (o código basta para o diário); uma máquina com o
PowerShell bloqueado por política fica sem elevação (recusa limpa, `ELEVATION_NOT_AVAILABLE`); o
pipe nomeado do plano fica para quando houver motivo para mais que um código.

## Consequências
- **O que o hash garante e o que não garante.** O `plan_hash` é SHA-256 sem chave e viaja dentro do
  próprio token: dá integridade (o helper lê exatamente o plano que o executor escreveu, no formato
  canônico byte a byte) e, com o prazo, limita o reuso de um token capturado. **Não dá
  autenticidade**: um programa rodando como a pessoa consegue montar um pedido coerente, com hash
  certo e prazo curto, e o helper aceita. As barreiras reais contra isso são duas: o clique no UAC
  e a allowlist compilada, pequena e inofensiva por desenho. Sem assinatura (F5-SIGN), o UAC do
  helper aparece como "editor desconhecido", igual ao de qualquer programa.
- Revisão independente de 16/10 (T016/T017): o helper só roda da pasta exata, com componentes
  simples; o pedido tem `operation` (só `run` existe), para um desfazer futuro nunca ser lido como
  "rodar de novo"; o substituto de resultado grande diz `unknown`.

- Um usuário padrão (sem conta de administrador) não consegue instalar; é o custo aceito do
  ADR-002, e a tela diz o que fazer em vez de falhar em silêncio.
- Instalação por máquina deixa as duas pastas em `Program Files`; o `.deb` do Linux segue como está.
- O **executor** sabe desfazer um passo elevado pela mesma via de elevação (`verify_step` desvia
  para o `Elevation` quando a variante pede rollback), com prazo próprio para não ser recusado só
  porque a janela da aprovação fechou. O que ainda **não** existe é a ação de desfazer do lado do
  helper: `ElevatedFix` não implementa `rollback`, porque nenhuma correção elevada declara hoje um
  `rollback` que valha uma ação privilegiada. Quando a primeira entrar (S-02), a ação de desfazer
  entra na mesma allowlist, com o mesmo token e a mesma revalidação — não há, e não vai haver, um
  caminho privilegiado que não passe por ela.
- O binário privilegiado liga `hmd-executor` (trait `Elevation`) e, com ele, `hmd-core` e
  `hmd-probes` (`netdev`, `sysinfo`): superfície de terceiros maior do que o helper precisa num
  processo administrativo. Separar os dois lados do protocolo em crates distintos é a correção,
  e fica para quando houver ação real para rodar.
- Sem assinatura (F5-SIGN), o UAC mostra "editor desconhecido": motivo para o teste manual (T012),
  não bloqueio de código.
