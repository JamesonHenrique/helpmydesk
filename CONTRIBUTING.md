# Contribuindo com o HelpMyDesk

Obrigado pelo interesse! Este projeto mexe no computador de pessoas sem conhecimento técnico,
então as regras de contribuição são mais rígidas que o normal em algumas áreas.

## Antes de começar

- Leia a [constituição](.specify/memory/constitution.md). Ela tem precedência sobre tudo.
- Mudança de comportamento começa por uma **spec** (`specs/NNN-nome/`), seguindo o fluxo do
  Spec Kit: `specify → clarify → plan → checklist → tasks → analyze → implement → converge`.
- Idioma: documentação de produto em português; specs, código, nomes e comentários em inglês.

## Autoria (DCO)

Usamos o [Developer Certificate of Origin](https://developercertificate.org/) em vez de CLA.
Todo commit precisa de `Signed-off-by` (use `git commit -s`).

## Commits

[Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`,
`refactor:`, `test:`, `chore:`, `ci:`. Título curto, em português ou inglês.

## Caminhos protegidos

Exigem revisão de CODEOWNER e ficam ao menos 48 h abertos antes do merge enquanto houver um
único mantenedor:

- `catalog/` (capacidades, playbooks, textos, modelos de IA permitidos)
- `crates/hmd-policy`, `crates/hmd-executor`, `crates/hmd-elevated`
- capabilities do Tauri e `.github/workflows/`

### Nova capacidade (correção ou sonda)

O PR precisa ter:

1. cartão completo: nível, efeitos, pré-condições, verificação e rollback **por plataforma**;
2. textos em português simples (o que faz, risco, como desfazer);
3. testes unitários;
4. cenário de laboratório que quebra a máquina de teste, corrige e verifica;
5. justificativa de por que não é uma ação bloqueada (N4).

## Checks que bloqueiam merge

Formatação, lint (`clippy -D warnings`), testes, varredura de segredos, auditoria de
dependências e, quando existirem, rastreabilidade de requisitos e validação do catálogo.

## Rodando localmente

Veja [docs/development.md](docs/development.md).
