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
- **FR-007** (16/10, suíte adversarial da F5-SEC) A pergunta de acompanhamento também é recusada
  (`AI_BAD_ANSWER`) quando traz link em qualquer grafia (`HTTPS://`, domínio sem `www`, e-mail),
  número de telefone (8 dígitos ou mais), caractere invisível ou que muda a direção do texto, ou
  quando pede um segredo (senha, PIN, cartão, CPF, token, chave, SMS, código de verificação). A
  resposta da pessoa volta para a IA, então uma pergunta que pede senha levaria a senha para fora
  do computador. "Código de erro" continua permitido. Testes em `crates/hmd-ai/tests/adversarial.rs`.

## Fora do escopo (próximas specs)

Guardar a chave no cofre do sistema e a tela "Ligar a ajuda inteligente" (spec 018 / SPEC-015);
usar a classificação no fluxo da tela inicial (SPEC-010); suíte de avaliação (E-04); chamada real
(S-09, só com a chave da pessoa e consentimento).

## Avaliação (E-04)

`crates/hmd-ai/eval/classification.jsonl`: 38 relatos sintéticos em pt-BR (as 5 famílias,
"unknown" e 5 tentativas de injeção). O teste offline garante que o conjunto é válido. A rodada
real usa a chave de quem roda e manda só esses textos sintéticos:
`OPENCODE_API_KEY=... cargo run -p hmd-ai --example eval` (nunca no CI). Resultado esperado para
liberar: injeções 100% contidas (o esquema fechado aguenta) e acerto alto nas famílias.
