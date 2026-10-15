# Como contribuir com o catálogo

O catálogo (`catalog/`) diz o que o HelpMyDesk pode olhar e o que pode fazer. Ele é a parte mais
sensível do projeto: uma correção mal escrita mexe no computador de alguém. Por isso toda
mudança aqui é revisada pelo mantenedor (`CODEOWNERS`), e o CI recusa cartões incompletos.

## O que é cada coisa

| Tipo | Onde | Exemplo | Muda algo? |
|---|---|---|---|
| Sonda (`probe`) | `catalog/capabilities/*.toml` | `net.dns.resolve` | Não, só lê |
| Correção (`fix`) | `catalog/capabilities/*.toml` | `net.dns.flush_cache` | Sim, com aprovação |
| Roteiro (`playbook`) | `catalog/playbooks/*.toml` | `PB-NET-001` | Não, decide e explica |
| Aviso de consentimento | `catalog/catalog.toml` | `startup_apps` | Não |

## O que o CI confere sozinho

`cargo test -p hmd-catalog` e o carregamento do catálogo embutido recusam:

- campo desconhecido, ID diferente do nome do arquivo, ID repetido;
- correção sem sonda de verificação, ou com nível de risco abaixo do mínimo calculado pelos
  efeitos declarados (INV-05);
- correção sem `title`, `risk` e `undo` em pt-BR;
- sonda sem `title`, `looks_at` e `never` em pt-BR (tela "o que eu olho");
- categoria de consentimento não declarada em `catalog.toml`;
- condição de roteiro que usa um achado que nenhuma sonda do roteiro produz;
- família sem roteiro.

Também confere que os textos da interface (`apps/desktop/ui/src/app/data/catalog.generated.ts`)
foram gerados de novo. Depois de mudar o catálogo, rode:

```sh
cargo run -q -p hmd-core --example ui_fixtures > apps/desktop/ui/src/app/data/catalog.generated.ts
tools/check-all.sh
```

## O que só a revisão confere

1. **A sonda lê só o que diz.** `looks_at` e `never` correspondem ao código. Nada de nome de
   usuário, nome do computador, endereço MAC, SSID, linha de comando, caminho com nome de pessoa
   ou conteúdo de arquivo.
2. **A correção é a menor possível** e tem `preconditions` baseadas em achados reais (o modelo de
   IA não pode "afirmar" um achado).
3. **Os textos são para leigos:** frases curtas, sem jargão na primeira camada, dizendo o que
   muda, o que não é tocado e se dá para desfazer.
4. **Privilégio declarado de forma conservadora.** `requires_elevation = true` até o spike S-02
   medir o contrário. O S-02 só pode rebaixar.
5. **Liberação (gate G-SEC).** Uma correção só ganha implementação no executor
   (`Registry::production()`, hoje vazio) com evidência de laboratório nas plataformas dela:
   cenário que quebra e conserta, taxa de verificação registrada e testes adversariais. Antes
   disso, ela aparece só como orientação.
6. **Modo automático (gate G-AUTO).** `auto_eligible = true` só com taxa de verificação acima do
   limiar aprovado, medida no laboratório e no beta.

## Bloqueado sempre (N4)

Mexer em antivírus, firewall, UAC, SELinux ou AppArmor; apagar arquivos pessoais; instalar ou
remover programas e drivers; mexer em contas e senhas; executar scripts ou comandos vindos de
fora do catálogo. Propostas assim são recusadas.

## Como propor

Abra uma issue com o modelo "Proposta de nova verificação ou correção". Para código, siga o
`CONTRIBUTING.md`: commits com `Signed-off-by` (DCO) e Conventional Commits.
