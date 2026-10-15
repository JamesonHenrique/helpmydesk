# ADR-030: download do kill switch

**Status:** aceito · **Data:** 2026-10-15

## Decisão

- O arquivo assinado e a assinatura ficam numa release dedicada `kill-switch` do repositório
  (não é versão do app, marcada como não-"latest"); o endereço é fixo no app.
- O app baixa ao abrir, numa thread em segundo plano: só HTTPS (inclusive depois do
  redirecionamento do GitHub para o host de arquivos), até 5 redirecionamentos, 15 s, arquivo de
  até 64 KiB e assinatura de até 1 KiB. Falha ou falta de internet: segue valendo o último
  aceito.
- Só é aplicado o que passa em `hmd_killswitch::verify` (chave compilada) e `accept` (mais novo
  que o último, não vencido); já ter o mesmo arquivo não é erro.
- Mesmo `ureq =3.4.2` do ADR-027, agora também em `hmd-ipc`.

## Privacidade

Nenhum dado da pessoa vai na requisição; o GitHub vê o IP, como em qualquer download (plano v1,
"o que sai do computador"). A tela "Sobre" diz isso.

## Consequências

O arquivo precisa ser reassinado antes de vencer (`docs/ops/kill-switch.md`).
