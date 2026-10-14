# ADR-007 — IA: modelos gratuitos do OpenCode Zen com a conta da própria pessoa

- **Status:** Aceita (2026-10-14). Decisão do dono do projeto: "vai ser a IA do OpenCode
  gratuita".
- **Contexto:** o plano v1, Parte VI, e a análise dos termos no [ADR-020](ADR-020-p1-analysis.md).

## Decisão

1. O núcleo Rust chama diretamente `https://opencode.ai/inference/v1/` (formato
   `chat/completions` para modelos compatíveis com a API da OpenAI). Não há servidor do projeto
   nem CLI `opencode` embutido.
2. A **chave é da própria pessoa**, informada num onboarding opcional e guardada no cofre do
   sistema (Credential Manager no Windows; Secret Service no Linux).
3. **Allowlist assinada** (`catalog/ai/models.yaml`): só modelos cujo provedor declara retenção
   zero e não treina com os dados, e que passaram na suíte de avaliação. Candidatos iniciais,
   a reconfirmar: Space Bunny Free, LongCat 2.5 Preview Free, Step 5 Preview Free.
4. A IA só é chamada em sessão iniciada pela pessoa (Princípio V) e só escolhe entre opções do
   catálogo (VI.3 do plano).
5. **Modo básico** sem IA é caminho completo, não um fallback de segunda classe.

## Consequências

- Custo zero para o projeto. Fricção de criar conta para a pessoa (H-09).
- Os modelos gratuitos são "preview" e vão mudar. A lista é atualizável e desativável pelo kill
  switch.
- Em contas não pagas, o OpenCode pode usar o conteúdo para melhorar os serviços dele. O app
  avisa e envia o mínimo (redação, só códigos de achados).

## Revisar se

Termos ou políticas mudarem, nenhum modelo de retenção zero passar na suíte, ou os modelos
gratuitos exigirem pagamento (plano B em Q-11 do plano).
