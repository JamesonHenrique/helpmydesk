# Spec 017 — Cliente de IA (OpenCode Zen) com allowlist

**Status:** implementada (núcleo) · **Roadmap:** SPEC-009 · **Data:** 2026-10-15

## Por quê

Com a IA ligada, a pessoa escreve o problema com as próprias palavras em vez de escolher no menu.
A IA usa a **conta e a chave da própria pessoa** no OpenCode (Q-01 respondida e liberada pelo
mantenedor em 15/10). O modo básico continua sendo caminho de primeira classe.

## Requisitos

- **FR-001** Uma única URL: `https://opencode.ai/inference/v1/chat/completions`, só HTTPS, sem
  redirecionamento, tempo e tamanho de resposta limitados.
- **FR-002** Só modelos grátis cujo provedor, segundo a documentação do Zen em 15/10, não guarda
  os dados nem treina com eles (`ALLOWED_MODELS`). Modelo fora da lista é recusado antes de
  qualquer envio.
- **FR-003** INV-01/INV-02: a IA só **classifica** o relato numa família (ou "unknown") com uma
  confiança e, opcionalmente, uma pergunta curta. Nenhuma ferramenta, nenhum passo, nenhum link.
  O diagnóstico, o plano e as correções continuam vindo dos roteiros e do catálogo.
- **FR-004** O texto da pessoa vai como dado (`{"untrusted_text": ...}`), limitado a 1000
  caracteres, depois da redação de nomes feita pelo chamador.
- **FR-005** A resposta é validada num esquema fechado (campos extras, família ou confiança
  desconhecidas, pergunta longa ou com link/HTML/Markdown → `AI_BAD_ANSWER`). Nada é "consertado".
- **FR-006** Erros viram códigos: chave recusada (401/403), limite (429), rede, HTTP.

## Fora do escopo (próximas specs)

Guardar a chave no cofre do sistema e a tela "Ligar a ajuda inteligente" (spec 018 / SPEC-015);
usar a classificação no fluxo da tela inicial (SPEC-010); suíte de avaliação (E-04); chamada real
(S-09, só com a chave da pessoa e consentimento).
