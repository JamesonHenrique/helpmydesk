## O que muda

## Spec / requisitos
- Spec: `specs/NNN-...`
- Requisitos atendidos: FR-..., SEC-...

## Checklist
- [ ] Commits com `Signed-off-by` (DCO) e Conventional Commits
- [ ] `tools/check-all.sh` passou localmente
- [ ] Testes citam os IDs dos requisitos
- [ ] Spec, ADR e docs afetados atualizados no mesmo PR
- [ ] Nenhum segredo, dado pessoal ou chave de serviço
- [ ] Dependência nova? Versão fixa, publicada há pelo menos 2 semanas, ADR com licença

## Se mexe no catálogo (`catalog/`)
- [ ] Li `docs/contributing/catalog.md`
- [ ] `looks_at` / `never` batem com o código da sonda
- [ ] Correção: precondições por achados reais, `requires_elevation` conservador, `risk` e `undo` claros
- [ ] Textos da interface gerados de novo (`ui_fixtures`)
- [ ] Implementação no executor só com evidência de laboratório (G-SEC)
