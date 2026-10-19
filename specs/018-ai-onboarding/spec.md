# Spec 018 — "Ligar a ajuda inteligente"

**Status:** implementada · **Roadmap:** SPEC-015, SPEC-010 (parte), S-08, S-09 (parte) · **Data:** 2026-10-15

## Por quê

O mantenedor quer que a pessoa ligue a IA sem precisar entender o que é uma "chave de API".
A documentação do OpenCode Zen (15/10) descreve o console com copiar/colar; a partir de
19/10/2026 o CLI do OpenCode também oferece login por device code (`opencode auth login
--method device`), que salva a sessão no próprio CLI e dispensa colar chave nenhuma.

## Fluxo

1. "Abrir o OpenCode" abre `https://opencode.ai/auth` no navegador (URL fixa no núcleo).
2. A pessoa entra (ou cria a conta) e clica em **Copiar**.
3. "Colar o código e ligar": o núcleo lê a área de transferência **no Rust** (o código nunca passa
   pela interface), testa a chave com uma chamada curta e só então guarda a chave no cofre do
   sistema e liga o consentimento `ai_opencode`. A área de transferência é limpa depois.
4. Se ler a área de transferência não der certo, aparece um campo de senha para colar com Ctrl+V.
5. Com a IA ligada, a tela inicial aceita texto livre: a IA só escolhe a família; com confiança
   baixa ou família desconhecida, o app pergunta ou mostra o menu. O resto do fluxo é o mesmo.

## Fluxo alternativo: device-flow (ADR-032, 19/10/2026)

1. "Entrar com sua conta" roda `opencode auth login --method device` pelo único helper
   `hmd_ai::opencode_command()` (só `hmd-ai` cria processos). O app drena o stdout/stderr do CLI
   e **abre a URL de verificação no navegador** assim que um linha dela aparece.
2. O CLI espera a pessoa concluir; terminado com sucesso, uma chamada curta de teste de conexão
   roda **fora do lock** (thread de bloqueio, como nas outras chamadas de rede).
3. Só com o teste verde o marcador `hmd_ai::DEVICE_KEY` entra no cofre e o consentimento
   `ai_opencode` é ligado; as chamadas seguintes passam pelo transporte subprocesso.
4. No próximo start, `Backend::ai_restore_transport` recompõe o modo a partir do cofre; "Desligar
   e apagar o código" apaga o marcador e o consentimento (sem logout do CLI).
5. Erros em palavras simples: o CLI não abriu ou falhou (`OPENCODE_FAILED`), URL não abriu
   (`OPEN_URL_FAILED`), teste recusado (`AI_KEY_REFUSED`/`AI_RATE_LIMITED`).

## Requisitos

- **FR-001** A chave só existe no cofre do sistema (Secret Service no Linux, Gerenciador de
  Credenciais no Windows). Sem cofre disponível, fica só na memória enquanto o app roda; nunca no
  arquivo de histórico nem em texto puro no disco.
- **FR-002** Chave recusada pelo serviço não é guardada; erros em palavras simples.
- **FR-003** Cada chamada à IA gera uma linha de auditoria só com códigos (modelo, resultado);
  nunca o texto.
- **FR-004** O texto da pessoa é redigido (nome de usuário e do computador) antes de sair.
- **FR-005** O kill switch também desliga modelos (`disabled_models`).
- **FR-006** "Desligar e apagar o código" remove a chave e o consentimento.
- **FR-007** O caminho alternativo de device-flow (ADR-032): login só por `hmd_ai::opencode_command`
  (spawn único de processo), teste de conexão real fora do lock, marcador `DEVICE_KEY` no cofre e o
  mesmo consentimento e auditoria do fluxo por chave.

## Verificado no app real (Xvfb, 15/10)

Com um código falso: a chamada HTTPS real ao `opencode.ai` foi feita e recusada, e a tela disse
"Esse código não funcionou". Falta o caso de sucesso, que exige a chave real de uma pessoa.

Device-flow (19/10/2026): testes unitários e de shell verdes (login bloqueante, guardas de erro,
restauração do transporte no start). Falta a verificação em máquina real com conta real (o CLI
espera a pessoa no navegador); registrada em `waiting_on` do SPEC-015.

## Fora do escopo

Suíte de avaliação (E-04). O resumo das verificações saiu deste escopo: feito na spec 021
(códigos dos achados e re-classificação pós-diagnóstico, ADR-033).
A chamada à IA roda fora do lock do backend (preparar → rede em thread de bloqueio → registrar),
então uma resposta lenta não trava as outras telas.

## Transparência (F3-TRANSP)

- Embaixo do campo de texto livre: "Vai para a IA, sem seu nome nem o do computador: «…»",
  calculado no núcleo (a mesma redação usada no envio).
- Em Configurações, com a IA ligada: a lista das últimas chamadas (data, modelo, resultado em
  palavras simples). O texto enviado nunca é guardado, por isso não aparece nessa lista.

## Pergunta de acompanhamento (SPEC-010)

Se a IA não tiver certeza, a pergunta dela aparece; a resposta da pessoa é somada ao relato e
classificada de novo (a prévia mostra o texto inteiro que vai sair). No máximo 2 rodadas; depois,
o app pede para escolher no menu, sem insistir.
