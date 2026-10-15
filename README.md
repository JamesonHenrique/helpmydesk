# HelpMyDesk

> Suporte técnico gratuito e aberto que examina, explica, corrige com segurança e prova que
> resolveu — no Windows e no Linux.

**Status: em desenvolvimento inicial (pré-alfa). Ainda não há versão para instalar.**

O HelpMyDesk é um aplicativo de desktop para pessoas sem conhecimento técnico. A pessoa
descreve o problema com as próprias palavras ("minha internet parou", "o computador está
lento", "o som não sai") e o app:

1. examina o computador com verificações **somente leitura** de um catálogo fechado;
2. explica a causa provável em português simples, sem fingir certeza;
3. propõe correções **conhecidas, reversíveis quando possível e aprovadas pela pessoa**;
4. **verifica** se resolveu e pergunta se a pessoa também percebe a melhora;
5. quando não consegue, diz isso e gera um **relatório** para a pessoa levar a quem for ajudar.

## O que já funciona (outubro de 2026)

- **Modo básico completo no núcleo:** 13 verificações somente leitura (internet, lentidão,
  espaço, som, microfone e câmera) e roteiros de diagnóstico para as 5 famílias de problema, nas
  duas plataformas.
- **Correções com segurança:** política determinística, aprovação presa ao plano exato numa janela
  separada, diário que sobrevive a quedas e verificação depois de cada correção. Nenhuma correção
  real está liberada ainda: elas esperam a medição de privilégios em Windows e Linux (spike S-02).
- **Histórico local** (SQLite) com auditoria encadeada, retenção automática, exportar e apagar
  tudo, e o **relatório "não consegui"**.
- **App desktop (Tauri)** ligado ao núcleo: compila e abre no CI em Linux e Windows. Ainda não
  há instalador.
- **Kill switch assinado** para desligar uma capacidade sem lançar versão (falta a chave do
  projeto).

Experimente no terminal, sem instalar nada:

```sh
cargo run -p hmd-core --example basic_mode            # menu
cargo run -p hmd-core --example basic_mode -- 1       # verifica a internet
cargo run -p hmd-core --example basic_mode -- --o-que-eu-olho
```

## Princípios

- **A IA propõe, o código decide.** A IA nunca executa nada: só escolhe entre opções de um
  catálogo revisado. Uma política determinística valida tudo antes de qualquer ação.
- **Nada escondido.** O app mostra o que olha, o que vai mudar, o que *não* vai mexer e o que
  é enviado para a IA.
- **Sem servidor e sem telemetria.** Tudo roda no seu computador.
- **Funciona sem IA.** O modo básico (menu + perguntas guiadas) funciona offline e sem conta.
- **IA opcional:** modelos gratuitos do [OpenCode Zen](https://opencode.ai/docs/zen/) usando a
  **sua própria conta**, e só modelos cujo provedor declara retenção zero e não treina com os
  dados. Veja [ADR-007](docs/architecture/adr/ADR-007-ai-opencode.md).

## Plataformas (alvo do MVP)

- Windows 10 22H2 e Windows 11 (x64)
- Linux x64 com systemd, NetworkManager e PipeWire: Ubuntu LTS, Linux Mint, Debian 13

## Baixe somente dos canais oficiais

Quando houver releases, eles serão publicados **apenas** em
[GitHub Releases](https://github.com/JamesonHenrique/helpmydesk/releases), assinados e com
attestations verificáveis. Versões de outras fontes podem ser maliciosas. Veja
[TRADEMARKS.md](TRADEMARKS.md).

## Documentação

- [Planejamento integral (v1)](docs/planning/PLANEJAMENTO-v1.md)
- [Decisões arquiteturais (ADRs)](docs/architecture/adr/README.md)
- [Constituição do projeto](.specify/memory/constitution.md)
- [Especificações](specs/README.md)
- [Como contribuir](CONTRIBUTING.md) · [Segurança](SECURITY.md) · [Código de conduta](CODE_OF_CONDUCT.md)

## Desenvolvimento

O projeto segue **Spec-Driven Development** com o [GitHub Spec Kit](https://github.com/github/spec-kit)
(v1.2.0). Núcleo em Rust; interface em Angular sobre Tauri v2. Detalhes em
[docs/development.md](docs/development.md).

```sh
tools/check-all.sh      # tudo o que o CI roda
```

## Licença

Código e catálogo: [GPL-3.0-or-later](LICENSE). Documentação em `docs/`:
[CC BY-SA 4.0](docs/LICENSE.md). Veja [ADR-018](docs/architecture/adr/ADR-018-license.md).

---

**English:** HelpMyDesk is a free, open-source (GPL-3.0-or-later) desktop assistant for Windows
and Linux that helps non-technical people diagnose and safely fix common computer problems.
The AI only proposes; deterministic code decides; every fix is verified. Early development —
see the planning documents (Portuguese) and [CONTRIBUTING.md](CONTRIBUTING.md).
