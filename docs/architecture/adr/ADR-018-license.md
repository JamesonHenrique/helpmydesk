# ADR-018 — Licença

- **Status:** Aceita (2026-10-14)
- **Contexto:** o projeto é gratuito e open source. O software executa ações no computador de
  pessoas leigas, então o código precisa ser auditável. Dependências principais: Tauri
  (MIT/Apache-2.0) e crates/pacotes com licenças permissivas.

## Alternativas

| Licença | A favor | Contra |
|---|---|---|
| **GPL-3.0-or-later** | Forks distribuídos continuam abertos e auditáveis, o que importa num agente que mexe no sistema; compatível com dependências MIT e Apache-2.0 | Reduz o reuso em produtos fechados; algumas empresas evitam contribuir |
| Apache-2.0 | Máxima adoção; concessão de patentes | Permite forks fechados do agente |
| MPL-2.0 | Copyleft por arquivo | Mais complexa de explicar; protege menos o conjunto |

## Decisão

- Código e catálogo (`catalog/`, que é compilado no binário): **GPL-3.0-or-later**.
- Documentação (`docs/`): **CC BY-SA 4.0**.
- Contribuições sob DCO (ADR-019), sem CLA.

**Critério que decidiu:** *aceitamos que alguém feche um fork do agente?* Não.

## Consequências

- Toda nova dependência precisa ter licença compatível com GPL-3.0. O `cargo-deny` fará essa
  checagem a partir da F1.
- Cada arquivo de código leva o cabeçalho SPDX `GPL-3.0-or-later`.

## Revisar se

Surgir necessidade concreta de embutir o núcleo em software de terceiros com licença
incompatível.
