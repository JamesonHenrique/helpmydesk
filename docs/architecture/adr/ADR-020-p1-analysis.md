# ADR-020 — Análise das pendências P1 e autorização para iniciar o desenvolvimento

- **Status:** Aceita (2026-10-14)
- **Decisores:** dono do projeto. A análise foi delegada ao assistente ("faça essa análise você
  mesmo e aprovo") e aprovada de antemão.
- **Contexto:** o planejamento v1 (`docs/planning/PLANEJAMENTO-v1.md`, Partes XI–XIII) deixou
  quatro pendências P1 e gates de entrada para a F1. O dono pediu que o assistente decidisse e
  que o desenvolvimento começasse.

## Q-01 — Termos do OpenCode (lidos em 2026-10-14, vigência 15/08/2026)

**Fatos:**
- "You will only use the Services for your own internal use, and not on behalf of or for the
  benefit of any third party."
- Itens proibidos: 4 (contas em massa ou múltiplas contas para contornar limites); 5
  ("automatically or programmatically extracts data or Output"); 10 ("any processes that run
  or are activated while you are not logged into the Services"); 12 ("copies or stores any
  significant portion of the Content").
- A documentação do Console diz que a chave funciona "with OpenCode or another coding agent" e
  descreve "applications calling the endpoints directly".

**Análise:**
1. **Chave do projeto embutida: incompatível.** Seria uso em nome de terceiros, a chave ficaria
   pública num repositório aberto e as chamadas aconteceriam sem o "dono" da conta logado.
   Proibido pela constituição (Princípio V).
2. **Chave da própria pessoa, para o computador dela:** é a leitura mais compatível com "own
   internal use". A pessoa usa o serviço para si; o HelpMyDesk é o cliente, como qualquer
   "coding agent" citado na própria documentação.
3. **Item 5 (extração programática):** a leitura razoável é que mira raspagem e coleta em massa
   de dados/Output do serviço, não o uso normal da API que a própria documentação oferece para
   aplicações. Fica como **risco residual**.
4. **Item 10:** atendido por desenho. A IA só é chamada dentro de uma sessão aberta pela pessoa,
   sem tarefas em segundo plano (Princípio V).
5. **Item 12 (armazenar Content):** o histórico local guarda o texto do catálogo e metadados
   (`AiCallLog` sem conteúdo), não a saída bruta do modelo. Respostas brutas não são
   persistidas por padrão.

**Decisão:** seguir com a integração **desta forma**. A IA fica **desligada por padrão**, com
consentimento e aviso de que é um serviço de terceiros com termos próprios. A checagem dos
termos e das políticas de dados dos modelos entra no checklist de cada release. A consulta
escrita ao OpenCode (rascunho no plano, XII.1) continua **recomendada**, mas não bloqueia o
desenvolvimento. O assistente não envia e-mails em nome do dono.

**Revisar se:** os termos mudarem, o OpenCode se manifestar, ou os modelos gratuitos exigirem
pagamento.

## Q-05 — Licença

Decisão: **GPL-3.0-or-later** para código e catálogo; **CC BY-SA 4.0** para `docs/`. Detalhe em
[ADR-018](ADR-018-license.md).

## Q-06 — Menores de idade (ECA Digital, Lei 15.211/2025; LGPD art. 14)

**Análise:**
- O app não coleta idade nem dados de identificação, não tem conta nem servidor, e não envia
  nada ao projeto.
- O risco jurídico depende de o software ser considerado "de acesso provável" por menores,
  o que está em aberto.
- O provedor de IA tem termos e idade mínima próprios (não verificados).

**Decisão:**
- O app e o README declaram uso destinado a **maiores de 18 anos**.
- A conexão com a IA reforça que a pessoa deve cumprir os termos do provedor.
- Nenhuma verificação de idade: exigiria coletar dados, contra o Princípio IV.

**Risco aceito pelo dono.** Recomenda-se análise jurídica antes do primeiro release público.

## Q-07 — Nome

**Busca no GitHub (2026-10-14):** apenas um repositório homônimo de terceiro (`helpmydesk`, sem
descrição, 0 estrelas). Busca no INPI **não realizada**.

**Decisão:** manter o nome por ora, com política de uso em `TRADEMARKS.md`. Risco aceito.

## Gates de entrada da F1 (Parte XIII.1 do plano)

| Gate | Situação | Decisão |
|---|---|---|
| A-04 (E-01: problemas reais) | Não realizado | **Dispensado pelo dono para iniciar**; obrigatório antes do primeiro release público |
| A-05 (E-03: teste com leigos) | Não realizado | Idem |
| A-06 (S-01: spike Rust nas duas plataformas) | Não realizado | Substituído pela entrega incremental do núcleo puro (`hmd-domain`), que mede a produtividade em Rust sem depender de bibliotecas de sistema |
| A-07 (Q-01) | Analisada acima | Atendido nesta forma |
| A-03 / A-02 (ADRs e constituição) | Constituição v1.0.0 ratificada; ADRs registrados | Atendido |

**Consequência:** o desenvolvimento começa pelo núcleo de domínio puro e testável (`specs/001`).
Sondas e correções reais seguem os gates por fase do plano.
