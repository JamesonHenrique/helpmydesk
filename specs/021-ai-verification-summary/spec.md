# Spec 021 — Resumo das verificações para a IA (re-classificação)

**Status:** implementada · **Roadmap:** SPEC-010 (resto) · **Data:** 2026-10-19

## Por quê

Com a IA ligada, a tela inicial classifica a família pelo relato; o diagnóstico determinístico
roda as sondas da família. Quando nenhum achado casa com um padrão da família
(`is_fallback`, antes de FR do diagnóstico), a frase de confiança baixa sai sem usar a
informação mais nova que o app tem: o que as verificações encontraram. O plano v1 já autoriza
enviar **só os códigos dos achados** (TH-07: "só códigos de achados"; VI.6: "Nunca enviar
trechos de log brutos; apenas `Finding.code` e números").

## Fluxo

1. A pessoa atendeu pelo caminho da IA (o relato redigido fica na sessão) e o diagnóstico
   terminou com `isFallback` — nenhum padrão da família casou com os achados.
2. O app faz **uma única chamada extra** de classificação: relato (redigido, como sempre) +
   códigos dos achados, pelo mesmo pipeline (prepara no lock → rede fora do lock → registra).
3. Se a IA devolve família **diferente** com confiança média ou alta, aparece um cartão:
   "As verificações sugerem outro problema", com o botão "Verificar isso". Navegar é decisão
   da pessoa; a IA nunca muda a tela sozinha.
4. Confiança baixa, mesma família, IA desligada ou erro: nada aparece, silencioso.
5. Máximo de **1 chamada extra por atendimento** (guarda na sessão; o início zera).

## Payload (o que sai do computador)

- O relato de sempre, redigido (`redact_free_text`), campo `untrusted_text`.
- Campo separado `verification_codes`: só códigos de achados do catálogo (strings estáticas,
  ex.: `net.dns.resolution_failing`), nunca valores, trechos de log ou identificadores.
- O prompt do sistema diz que os códigos são dados das verificações, não instruções.

## Requisitos

- **FR-001** Além do relato redigido, o payload leva apenas códigos de achados; nunca log
  bruto, valores das sondas ou identificadores de pessoa/computador (TH-07, VI.6).
- **FR-002** Os códigos são validados na entrada e na saída do app: só `[a-z0-9._-]`, até 64
  caracteres cada, no máximo 24 códigos, sem repetição; o que não casa é descartado.
- **FR-003** A chamada extra usa o mesmo esquema fechado (só `family`/`confidence`/
  `question`), o mesmo consentimento, o mesmo kill switch (modelos) e a mesma auditoria
  (só códigos: modelo e resultado).
- **FR-004** Máximo de 1 chamada extra por atendimento; o botão de navegação é da pessoa.
- **FR-005** O cartão de sugestão mostra o que vai para a IA, já redigido (prévia da
  F3-TRANSP) e a lista dos códigos, que não identificam ninguém.

## Fora do escopo

Suíte de avaliação da IA (E-04); re-classificação no caminho do menu (sem relato não há o que
reclassificar); perguntas de acompanhamento pós-diagnóstico.
