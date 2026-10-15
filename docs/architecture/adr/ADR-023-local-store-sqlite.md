# ADR-023 — Histórico local em SQLite com `rusqlite`

- **Status:** Aceita (2026-10-14)
- **Contexto:** a spec 007 guarda chamados, eventos, sessões, achados, mensagens, consentimentos e
  relatórios só no computador da pessoa (plano v1, VII.4: SQLite em modo WAL, migrações com
  backup, banco corrompido recriado com aviso). O workspace proíbe `unsafe` no nosso código.

## Decisão

- **`rusqlite` 0.40** (MIT; repositório ativo, último push em 2026-10-12) com a feature
  **`bundled`**: compila o SQLite 3.53 (domínio público) junto com o app, sem depender da versão
  instalada no sistema. Isso garante tabelas `STRICT`, `VACUUM INTO` e o mesmo comportamento em
  Windows e Linux.
- Dependências novas que vêm junto: `libsqlite3-sys` (MIT), `hashlink`, `hashbrown`,
  `fallible-iterator`, `fallible-streaming-iterator`, `smallvec`, `bitflags` (MIT/Apache-2.0) e
  `foldhash` (Zlib). Todas compatíveis com a GPL-3.0-or-later.
- O código fica no crate folha **`hmd-store`**: nenhum outro crate depende dele.
- IDs são o `rowid` do SQLite, e não ULID como dizia a VII.4. Os dados não saem do computador
  nem são mesclados com outros bancos, então unicidade global não traz ganho agora.
- `sha2` fica para a spec do executor (hash do plano, cadeia da auditoria).

## Consequências

- Compilar exige um compilador C. Ele existe no CI (Linux: gcc; Windows: MSVC), mas não há
  MinGW na máquina de desenvolvimento. Por isso o check local de Windows usa
  `--exclude hmd-store`, e o Windows desse crate é coberto só pelo CI.
- O `unsafe` interno do `rusqlite`/SQLite é risco de cadeia de suprimentos, a acompanhar com
  `cargo-deny` e auditoria, como na ADR-022.

## Revisar se

O `rusqlite` for arquivado ou tiver advisory, ou se um banco puro em Rust maduro passar a cobrir
WAL e migrações com a mesma confiabilidade.
