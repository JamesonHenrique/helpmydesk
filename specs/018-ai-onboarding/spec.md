# Spec 018 — "Ligar a ajuda inteligente"

**Status:** implementada · **Roadmap:** SPEC-015, SPEC-010 (parte), S-08, S-09 (parte) · **Data:** 2026-10-15

## Por quê

O mantenedor quer que a pessoa ligue a IA sem precisar entender o que é uma "chave de API".
Criar a conta e a chave automaticamente em nome da pessoa não é possível pelo caminho oficial: a
documentação do OpenCode Zen (15/10) só descreve entrar no console e copiar a chave; não há OAuth,
código de dispositivo nem API de criação. Este é o caminho mais curto dentro disso.

## Fluxo

1. "Abrir o OpenCode" abre `https://opencode.ai/auth` no navegador (URL fixa no núcleo).
2. A pessoa entra (ou cria a conta) e clica em **Copiar**.
3. "Colar o código e ligar": o núcleo lê a área de transferência **no Rust** (o código nunca passa
   pela interface), testa a chave com uma chamada curta e só então guarda a chave no cofre do
   sistema e liga o consentimento `ai_opencode`. A área de transferência é limpa depois.
4. Se ler a área de transferência não der certo, aparece um campo de senha para colar com Ctrl+V.
5. Com a IA ligada, a tela inicial aceita texto livre: a IA só escolhe a família; com confiança
   baixa ou família desconhecida, o app pergunta ou mostra o menu. O resto do fluxo é o mesmo.

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

## Verificado no app real (Xvfb, 15/10)

Com um código falso: a chamada HTTPS real ao `opencode.ai` foi feita e recusada, e a tela disse
"Esse código não funcionou". Falta o caso de sucesso, que exige a chave real de uma pessoa.

## Fora do escopo

Enviar o resumo das verificações para a IA (só o texto vai hoje); suíte de avaliação (E-04).
A chamada à IA roda fora do lock do backend (preparar → rede em thread de bloqueio → registrar),
então uma resposta lenta não trava as outras telas.
