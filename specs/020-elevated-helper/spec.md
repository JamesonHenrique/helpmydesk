# Feature Specification: Elevated Helper (UAC) e caminho de elevação

**Feature Branch**: `020-elevated-helper`

**Created**: 2026-10-15

**Status**: Draft

**Input**: Plan v1 V.4 (elevação no Windows, alternativa B, SEC-020…SEC-026; elevação no Linux, L1 e
L2), V.7 (janela de aprovação), matriz N3, spec 008 (executor e diário), constituição II

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Nenhum código privilegiado fora do caminho de elevação (Priority: P1)

**Why this priority**: sem elevação declarada, qualquer correção que precise de privilégio de
admin simplesmente não existe; com elevação ad-hoc, malware no nível do usuário vira root.

**Independent Test**: com o executor apontando para um registro vazio, um passo com
`requires_elevation: true` é recusado antes de qualquer escrita no diário e a pessoa recebe a
orientação do catálogo, sem UAC na tela.

**Acceptance Scenarios**:

1. **Given** um passo do catálogo com `requires_elevation: true`, **When** não há implementação
   elevada neste build, **Then** a execução é recusada com `ELEVATION_NOT_AVAILABLE`, nada é
   gravado no diário e nenhum pedido de privilégio aparece.
2. **Given** um passo com `requires_elevation: true`, uma aprovação válida e a implementação
   elevada presente, **When** a pessoa autoriza na janela de aprovação, **Then** o app avisa
   antes ("o sistema vai pedir sua permissão") e só depois o UAC aparece.
3. **Given** a pessoa não tem direito administrativo, **Then** o UAC nega, o passo fica
   `failed` com `ELEVATION_DENIED`, nada muda no sistema e a pessoa recebe o caminho de
   escalonamento (relatório para levar a alguém).

### User Story 2 - O helper revalida tudo e roda uma ação por invocação (Priority: P1)

**Why this priority**: o helper é o único código que roda como administrador; qualquer confiança
no processo de usuário seria um escalonamento trivial.

**Independent Test**: a biblioteca do helper é testada sem privilégio nenhum — parsing do pedido,
allowlist, recálculo do hash e recusa de caso-forja alimentados direto na função.

**Acceptance Scenarios**:

1. **Given** um pedido cujo `capability_id` não está na allowlist compilada, **Then** o helper
   recusa (`NOT_IN_ALLOWLIST`), não executa nada e encerra.
2. **Given** um pedido cujo material canônico não produz o `plan_hash` declarado, **Then** o
   helper recusa (`PLAN_HASH_MISMATCH`) antes de qualquer chamada privileged.
3. **Given** uma variante elevada que não existe para a plataforma atual, **Then** o helper
   recusa (`PLATFORM_NOT_SUPPORTED`) e diz qual plataforma aceita.
4. **Given** qualquer pedido, **Then** o helper executa no máximo **uma** ação, sem shell, sem
   rede, sem stdin, sem herdar o ambiente do processo de usuário e sem ler arquivo gravável
   pelo usuário para montar a execução.
5. **Given** um resultado, **Then** o helper devolve JSON com a versão do protocolo, o desfecho e
   um `reason_code` estável, mais um código de saída; nada de saída bruta de sonda ou dado que
   identifique a pessoa.

### User Story 3 - O helper só roda de onde foi instalado (Priority: P1)

**Why this priority**: helper em pasta gravável pelo usuário anula a proteção toda: o binário
poderia ser trocado antes do clique no UAC.

**Acceptance Scenarios**:

1. **Given** o helper executando de um caminho fora da pasta de instalação por máquina, **Then**
   ele recusa (`HELPER_PATH_NOT_TRUSTED`) e não faz nada.
2. **Given** a instalação por máquina (pasta gravável só por administrador), **Then** o caminho
   do helper é o mesmo que o instalador registrou e a desinstalação o remove.

### User Story 4 - Nada fica pendurado (Priority: P2)

**Why this priority**: janela de UAC esquecida ou diálogo do polkit aberto travam o app e o
chamado; o estado incerto é o pior resultado possível.

**Acceptance Scenarios**:

1. **Given** o UAC em aberto sem resposta, **When** o prazo duro vence, **Then** o passo vira
   `unknown` no diário, nunca é re-executado sozinho e o app diz o que aconteceu.
2. **Given** o app fechado no meio, **Then** a execução fica `unknown` e a política bloqueia a
   mesma capacidade até a pessoa decidir (regra da spec 008).
3. **Given** um passo `unknown`, **Then** a verificação não roda como se nada tivesse acontecido
   e o chamado pode virar "não consegui resolver".

### Edge Cases

- Parâmetros com acento, espaço ou aspas: o pedido viaja como um único token JSON na linha de
  comando, sem concatenação de string em shell (nunca existe shell).
- Pedido maior que o limite (4 KiB) ou com campo desconhecido: recusado na hora, sem eco do
  conteúdo no diário.
- Ação que peça para o sistema pedir senha no Linux: nesta versão o caminho elevado é o do
  Windows; no Linux a decisão depende da medição do S-02.
- Duas abas do app abertas pedindo a mesma ação: a segunda ocorrência cai em
  `DUPLICATE_EXECUTION` pelo `idempotency_key`, sem segundo UAC.
- A IA, um plano ou um arquivo do usuário nunca escolhem o caminho de elevação: isso vem do
  catálogo assinado (`requires_elevation` da variante).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `crates/hmd-elevated` novo crate com biblioteca (pedido, allowlist, revalidação,
  resultado) e binário; é um dos dois únicos lugares do código onde `Command::new` é aceito pela
  guarda `tools/check-no-process-spawn.sh`.
- **FR-002**: Protocolo versionado `hmd-elevated@1`: pedido com `protocol`, `capability`,
  `capability_version`, `variant_id`, `platform`, `step_id`, `plan_hash` e `params`
  (`params_schema` do catálogo já validado). Campo desconhecido é erro.
- **FR-003**: O pedido viaja na linha de comando da invocação elevada, fixado na criação do
  processo, em um único token; nada é lido de arquivo, de variável de ambiente ou de stdin.
- **FR-004**: Limite de tamanho do pedido (4 KiB) e limite de tamanho do resultado; estouro é
  `REQUEST_TOO_LARGE`.
- **FR-005**: Allowlist compilada no próprio crate: par `(capability, versão, variant_id)` →
  implementação nativa. Ação fora da lista é `NOT_IN_ALLOWLIST`. A lista de produção fica
  **vazia** até o spike S-02.
- **FR-006**: O helper recalcula o hash canônico do material que recebeu (passos, decisão,
  versão do catálogo) e exige igualdade com o `plan_hash` do pedido; divergência é
  `PLAN_HASH_MISMATCH`. O helper não confia em nenhum campo recebido sem recalcular.
- **FR-007**: O helper revalida o pedido contra o catálogo embutido deste build (variante
  existente, `requires_elevation: true`, plataforma presente, params dentro do esquema) antes de
  executar.
- **FR-008**: Lançador no `hmd-executor` (único outro lugar com `Command::new`): pedido do UAC
  com o token do pedido e coleta do resultado por stdout; trait de lançador com implementação
  falsa nos testes.
- **FR-009**: Ordem inalterada do executor da spec 008: `started` no diário **antes** de criar o
  processo elevado; `succeeded`/`failed`/`unknown` depois; a verificação continua no processo de
  usuário, com a sonda somente leitura do catálogo.
- **FR-010**: Prazo duro por invocação (inclui a espera pelo UAC): vencido, o passo vira
  `unknown` (`ELEVATION_TIMEOUT`) e nunca é re-executado automaticamente.
- **FR-011**: O helper confere o próprio caminho antes de tudo e recusa rodar fora da pasta de
  instalação por máquina (`HELPER_PATH_NOT_TRUSTED`). O instalador do Windows é **por máquina**
  (app e helper em pasta gravável só por administrador, ADR-002 do plano v1); conta sem direito
  administrativo nem instala e recebe orientação (Q-10 do plano).
- **FR-012**: Desinstalação limpa remove o helper, a pasta de instalação e as sobras do registro
  (par com a desinstalação limpa da spec 008/F2-UNINST).
- **FR-013**: Aviso pré-elevação na interface antes do UAC, com o texto do plano v1 V.7, e motivo
  legível quando a elevação não está disponível (`ELEVATION_NOT_AVAILABLE`).
- **FR-014**: O resultado JSON do helper (`protocol`, `outcome`, `reason_code`, `detail`
  sanitizado) é o que chega ao diário; nada de saída bruta de sonda, caminho de usuário, nome de
  usuário ou nome de máquina.
- **FR-015**: Caminho do Linux: nesta versão o app **não** pede `pkexec`; ação sem API D-Bus
  equivalente fica sem implementação até o S-02 decidir (L1 preferida, L2 só se indispensável).

### Security Requirements (SEC-020…SEC-026 do plano v1)

- **SEC-020**: Instalação por máquina; o helper recusa rodar de pasta gravável pelo usuário.
- **SEC-021**: Allowlist própria e compilada (3–5 ações no MVP); o helper ignora o que não está nela.
- **SEC-022**: Pedido no argv, esquema estrito, tamanho máximo, sem segredos, nunca em arquivo
  gravável pelo usuário.
- **SEC-023**: Revalidação completa no helper (hash, catálogo, plataforma, esquema de params).
- **SEC-024**: Sem shell: API nativa; ferramenta do sistema só com caminho absoluto e argumentos
  montados como vetor, nunca string concatenada.
- **SEC-025**: Resultado em JSON com código de saída; quem grava no diário é o processo de usuário.
- **SEC-026**: Timeout duro, uma ação por invocação, nenhuma rede.
- **SEC-027**: O helper é efêmero: termina ao terminar a ação, sem ficar residente.

### Non-Functional Requirements

- **NFR-001**: Dependência nova só se unavoidable, com versão exata e ADR: a chamada elevada no
  Windows (API `ShellExecuteEx` do sistema) e, se o desenho exigir, `windows-sys` (MIT).
- **NFR-002**: Todo MUST de FR-/SEC- tem ao menos um teste citando o ID (constituição VI), sem
  privilégio real (lançador falso) e sem rede.
- **NFR-003**: Build do binário do helper entra no CI nas duas plataformas; o guarda de spawn e o
  clippy continuam limpos.

## Success Criteria *(mandatory)*

- **SC-001**: 100% dos passos com `requires_elevation: true` são recusados ou elevated through
  this path — nenhum terceiro caminho de privilégio no código (`tools/check-no-process-spawn.sh`).
- **SC-002**: Cada cenário de aceitação acima tem teste automatizado; o helper é testado contra
  pedido forjado, allowlist furada, hash adulterado e caminho não confiável.
- **SC-003**: O `unknown` por timeout é o resultado declarado em 100% dos testes de expiração, e
  nenhum teste encontra re-execução automática de passo `unknown`.
- **SC-004**: O texto do aviso pré-elevação aparece na tela antes do UAC em teste de interface.
- **SC-005**: Documentação: guia do colaborador explica por que o binário fica em pasta de
  máquina e por que a lista de ações é pequena; revisão de segurança independente happen na
  allowlist de produção (F5-SEC).

## Assumptions

- `Registry::production()` continua vazio até o S-02 (spec 008): esta spec entrega o mecanismo,
  não correções reais.
- O pedido é autenticado apenas pela correspondência do hash e pela allowlist; a aprovação da
  pessoa é garantida pelo executor + UAC/polkit (limite declarado do plano v1, aceito).
- O CI não tem UAC: o lançador real é testado só por inspeção e o teste de integração da
  elevação real acontece no laboratório (F2-LAB) e no S-02.
- A assinatura do helper vem com F5-SIGN; sem assinatura o UAC mostra "editor desconhecido", o
  que é motivo para o teste manual, não um bloqueio de código.

## Fora do escopo

- Ações elevadas reais em produção (depende do S-02 e do laboratório).
- Helper `pkexec` no Linux (L2) e daemon root (L3, rejeitado).
- Serviço Windows residente (alternativa C, adiada).
- Assinatura dos binários (F5-SIGN) e publicação do instalador (F5-REL).