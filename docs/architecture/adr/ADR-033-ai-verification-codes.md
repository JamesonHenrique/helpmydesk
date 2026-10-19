# ADR-033: códigos de verificação na classificação da IA

**Status:** aceito · **Data:** 2026-10-19

## Contexto

A classificação da IA roda na tela inicial, antes do diagnóstico; só o relato da pessoa sai do
computador (spec 018). O ponto mais fraco é a primeira escolha de família: se nenhum padrão da
família casa com os achados (`is_fallback`), o app devolve a frase de confiança baixa sem
aproveitar o que as verificações acabaram de descobrir. O plano v1 autoriza enviar à IA
apenas os códigos dos achados (TH-07, VI.6), e a entrega desses códigos era a peça que faltava
da SPEC-010. Falta um ADR decidindo **o que** e **quando** vai.

## Decisão

- Os códigos dos achados viajam como **campo separado** no corpo da requisição
  (`verification_codes`), nunca concatenados ao texto livre: o prompt do sistema os descreve
  como dados das verificações, não como instruções, e o texto continua em `untrusted_text`
  (defesa contra injeção de prompt).
- Payload só de códigos é garantido por **dedução em duas camadas**: `hmd_app` sanitiza ao
  preparar a chamada (consentimento, chave, kill switch, redação do texto, validação de
  códigos) e `hmd_ai::sanitize_codes` valida de novo ao montar o corpo — só
  `[a-z0-9._-]`, ≤ 64 caracteres, ≤ 24 códigos, sem repetição; o resto é descartado.
- O gatilho é **determinístico**: diagnóstico terminado com `is_fallback` **e** a pessoa
  atendeu pelo caminho da IA (relato guardado na sessão). No máximo **1 chamada extra por
  atendimento** (guarda na sessão; o início zera).
- A chamada extra usa o mesmo pipeline das outras: preparar no lock do backend, rede em
  thread de bloqueio fora do lock, auditoria só de códigos. Mesmo esquema fechado
  (`family`/`confidence`/`question`), sem ferramentas.
- A IA **sugere**; quem muda de tela é a pessoa: se a resposta for família diferente com
  confiança média ou alta, um cartão oferece "Verificar isso" com clique explícito. Erros,
  confiança baixa ou família igual: nada aparece.
- Sem relato (caminho do menu) não há re-classificação — não há o que reclassificar.

## Consequências

- O payload não-identificante continua valendo: códigos do catálogo são strings estáticas
  (`net.dns.resolution_failing`); nem nome de usuário nem hostname existem neles.
- Modelos passam a ver até 24 códigos; a resposta segue limitada ao esquema fechado, então o
  pior caso é uma família errada que a pessoa pode recusar.
- O teste em máquina real com chave real fica pendente (como nas outras entregas de IA);
  os demais caminhos têm testes unitários e de shell.
