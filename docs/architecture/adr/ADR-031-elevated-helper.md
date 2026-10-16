# ADR-031: helper elevado em processo separado

**Status:** aceito (parcial: sem lançador no Windows, ver "Bloqueio") · **Data:** 2026-10-16

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
  `unknown` e nunca é re-executado (FR-010, SEC-026).
- **Linux sem `pkexec` nesta versão** (FR-015): a via preferida do plano v1 é L1 (API de sistema via
  D-Bus, sem elevate); o S-02 precisa medir se o catálogo tem o que oferecer antes de criar
  privilégio no Linux.
- **`hmd-canonical`** virou crate próprio para que executor e helper compartilhem *a mesma*
  codificação e o mesmo hash, sem cópia divergente.

## Bloqueio: o lançador no Windows

`ShellExecuteExW` (o "runas") só existe em FFI, e o workspace tem
`[workspace.lints.rust] unsafe_code = "forbid"` — o repositório inteiro é zero-`unsafe`. Escrever o
lançador é, portanto, decisão de constituição, não de implementação. Três saídas, para o mantenedor
escolher:

1. Emendar a constituição para permitir `unsafe` **só** em `hmd-elevated`, com ADR e revisão de
   cada chamada (a superfície é pequena: uma chamada de UAC, uma de processo, um pipe nomeado).
2. Usar um crate Community que embrulha a chamada com API segura, aceitando o custo de cadeia de
   suprimento (`cargo deny` precisa passar).
3. Ficar sem lançador até existir máquina Windows de teste (o estado de agora: recusa segura).

Enquanto nenhuma estiver decidida, `production_launcher()` devolve `UnavailableLauncher` e a
elevação falha com `ELEVATION_NOT_AVAILABLE` antes de qualquer escrita no diário.

## Consequências

- Um usuário padrão (sem conta de administrador) não consegue instalar; é o custo aceito do
  ADR-002, e a tela diz o que fazer em vez de falhar em silêncio.
- Instalação por máquina deixa as duas pastas em `Program Files`; o `.deb` do Linux segue como está.
- A Reversão automática de um passo elevado é tentada pela **mesma** via de elevação (mesmo token,
  prazo próprio), para não existir um caminho privilegiado que não revalida.
- Sem assinatura (F5-SIGN), o UAC mostra "editor desconhecido": motivo para o teste manual (T012),
  não bloqueio de código.
