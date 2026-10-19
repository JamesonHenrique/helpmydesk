# Instruções para agentes de código

Vale para qualquer agente (OpenCode, Codex, Claude Code etc.). Em conflito, a
[constituição](.specify/memory/constitution.md) vence este arquivo.

## O projeto

HelpMyDesk: assistente de suporte técnico para pessoas leigas, desktop Windows e Linux.
Tauri 2 + Rust (workspace em `crates/`) + Angular (`apps/desktop/ui`). Licença GPL-3.0-or-later.
A IA só classifica o problema; quem decide e executa é código determinístico, com aprovação da
pessoa.

## Idioma

- Código, nomes, comentários e specs: inglês.
- Documentação de produto, textos para o usuário e mensagens de commit: português simples.

## Comandos

Ambiente (Linux): `export PATH="$HOME/.cargo/bin:$PATH"` e, se usar fnm, `eval "$(fnm env)"`.

- Tudo o que o CI roda: `tools/check-all.sh` (rode antes de cada commit).
- Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`.
- Um crate: `cargo test -p hmd-ai`.
- Interface: `cd apps/desktop/ui && npx ng test --watch=false && npx prettier --check "src/**/*.{ts,html,css}"`.
- Catálogo mudou: regenere o arquivo da interface com
  `cargo run -q --locked -p hmd-core --example ui_fixtures > apps/desktop/ui/src/app/data/catalog.generated.ts`.

## Fluxo de trabalho

- Mudança de comportamento começa por uma spec em `specs/NNN-nome/` (Spec Kit:
  specify → clarify → plan → checklist → tasks → analyze → implement → converge). Se a
  implementação mostrar que a spec está errada, corrija o `spec.md` primeiro.
- O PR que muda comportamento atualiza no mesmo PR a spec, o ADR (`docs/architecture/adr/`) e a
  documentação afetados.
- IDs de requisito (FR-/NFR-/SEC-) aparecem no nome dos testes, por exemplo
  `fr002_sc001_exhaustive_table`.
- Entrega de item do roteiro: atualize `docs/status/roadmap.json` no mesmo commit (ver
  `docs/status/README.md`).
- Commits: Conventional Commits, sempre com `git commit -s` (DCO).
- `main` é protegida: só entra por PR com os checks verdes.

## Regras que não se negociam

- Nada de `unwrap()`, `expect()` ou `panic!` em código de produção. Erros carregam um
  `reason_code` estável.
- Só `hmd-executor`, `hmd-elevated` e `hmd-ai` (transporte OpenCode, consentimento
  `ai_opencode`, helper único `hmd-ai::opencode_command`) podem criar processos.
  `tools/check-no-process-spawn.sh` verifica isso.
- A IA não recebe ferramentas e só responde no esquema fechado (`crates/hmd-ai`). Resposta fora
  do esquema é recusada, nunca "consertada".
- Nenhuma correção roda sem aprovação da pessoa para aquele plano, daquele chamado, dentro da
  validade e uma única vez.
- Nada que identifique a pessoa (nome de usuário, nome do computador, saída bruta das sondas)
  chega à tela, ao relatório ou à exportação.
- Ações proibidas em qualquer caso: desligar antivírus, firewall, UAC, SELinux ou AppArmor;
  apagar arquivos pessoais; instalar ou remover drivers e pacotes; rodar comando arbitrário;
  controle remoto.
- Caminhos protegidos (`catalog/`, `crates/hmd-policy`, `crates/hmd-executor`,
  `crates/hmd-elevated`, capabilities do Tauri, `.github/workflows/`): mudanças mínimas,
  explicadas no PR.
- Dependência nova: versão exata, justificativa (licença, manutenção, alternativa) no PR e ADR
  quando for estrutural. `cargo deny` precisa passar.
- Segredos nunca entram no repositório (o repositório é público). Não leia nem imprima `.env`,
  chaves ou tokens.
- Testes não acessam a rede. A avaliação real da IA
  (`cargo run -p hmd-ai --example eval`) usa a chave de quem roda e não vai para o CI.

Onde cada garantia é testada: [docs/security/review-guide.md](docs/security/review-guide.md).

## Identidade visual e interface

- A identidade visual aprovada é a "Oficina editorial contemporânea" (v1.0), em
  `branding/` (`tokens.json`, `tokens.css`, `manifest-assets.json` e os 3 SVGs de logo).
- A fonte operacional da UI é `apps/desktop/ui/src/styles.css`: `branding/tokens.css`
  e `tokens.json` são referência e devem ser mantidos sincronizados — novas cores
  passam pelos três arquivos na mesma mudança.
- `--font-size` é runtime (A−/A+ em `app.ts`); nunca quebre a escalada de texto.
- Estilos de shell ficam compactos em `app.css` (orçamento `anyComponentStyle` w4kB/e8kB).
- Revisores read-only: `.opencode/agents/{brand-reviewer,accessibility-reviewer,safety-boundary-reviewer}.md`;
  comandos `/brand-audit` e `/ui-verify`. Zero de cor inventada fora dos tokens.
- Prettier sempre dentro de `apps/desktop/ui/`: o npx na raiz resolve outra versão
  (3.9.10 vs 3.9.9 do projeto) e acusa falsos erros de formato.
