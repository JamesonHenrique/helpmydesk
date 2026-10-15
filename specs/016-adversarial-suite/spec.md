# Spec 016 — Suíte adversarial na fronteira do IPC

**Status:** implementada · **Roadmap:** F5-SEC (parcial) · **Data:** 2026-10-15

## Por quê

A interface (webview) é a parte mais exposta do app: se ela for comprometida ou tiver um bug, só
consegue agir pelos comandos do `hmd-ipc`. Esta suíte trata a interface como atacante e prova que
nenhuma sequência de chamadas faz uma correção rodar sem a aprovação válida da pessoa para aquele
plano, daquele ticket, naquele momento.

## Requisitos

- **FR-001** Cada ataque roda num backend novo com uma correção falsa que conta as execuções.
  "Barrado" = a última chamada devolveu erro com código **e** a correção rodou 0 vezes.
- **FR-002** Um teste de controle prova que, no caminho legítimo, a mesma correção roda uma vez
  (a suíte não passa por um laboratório que nunca executa nada).
- **FR-003** O número de casos é verificado no teste; a meta é 100% barrados.
- **FR-004** Ataques cobertos: executar sem aprovar; aprovar passo inexistente ou nenhum passo;
  plano inexistente e ids extremos; plano e aprovação de outro ticket; ids de ticket forjados;
  aprovar depois de recusar; aprovação vencida; "só observar" antes e depois de aprovar; plano
  antigo depois de um diagnóstico novo (aprovado ou não); planos de um histórico apagado; relatório
  gigante salvo em Downloads (17 casos). Fora do IPC, em `hmd-app/tests/killswitch.rs`: kill switch
  que chega entre a aprovação e a execução.

## Achados e correções (no `hmd-app`)

| Ataque | Antes | Correção |
|---|---|---|
| Executar o plano aprovado de outro ticket | a correção rodava e o resultado ia para o ticket errado | `execute` e `decline` exigem que o plano seja do ticket (`take_pending`) |
| Mudar para "só observar" depois de aprovar | a correção rodava | `execute` recusa com `OBSERVE_ONLY` |
| Plano antigo depois de um diagnóstico que não achou mais o problema | podia ser aprovado e executado | diagnóstico novo descarta os planos pendentes do ticket |
| Aprovar passo inexistente ou nenhum passo | aceito; `execute` dizia "verified" sem fazer nada | `approve` recusa com `STEP_NOT_IN_PLAN` |
| Kill switch depois da aprovação | a correção desligada rodava | `apply_kill_switch` descarta planos afetados; `execute` recusa com `CAPABILITY_DISABLED` |
| Relatório editado sem limite | qualquer tamanho ia para o banco e para Downloads | `MAX_REPORT_BYTES` = 64 KiB, `REPORT_TOO_LARGE` |

## Janela de aprovação (S-04, parte que dá para testar sem abrir o app)

- O conteúdo vem do núcleo (`plan`), nunca da janela principal; a principal não emite eventos e
  só a janela `approval` pode aprovar (ACL testada desde a spec 013).
- Clique sequestrado: a janela principal pode chamar `open_approval` a qualquer momento, e o
  plano já vem com todos os passos marcados. Por isso o "Sim" só funciona 1 segundo depois que o
  plano aparece (`APPROVAL_ARM_MS`), como nos diálogos de permissão dos navegadores. Teste na UI.

## Fora do escopo

Revisão de segurança independente (quem escreveu os ataques também escreveu as defesas), fuzzing,
ataques ao processo nativo e ao sistema operacional. F5-SEC continua aberto.

## Rodada 3: dados do sistema (entradas não confiáveis)

| Achado | Antes | Correção |
|---|---|---|
| Redação do nome da pessoa/computador | caracteres cuja minúscula muda de tamanho em bytes (`İ`, `K` Kelvin) desalinhavam a busca e o nome **ficava no texto** | busca caractere a caractere no texto original (`find_folded`); teste com 5 000 combinações |
| `/proc/asound/pcm` com contagens absurdas | `sum` de `u64` podia estourar (pânico em debug) | `any(n > 0)` |
