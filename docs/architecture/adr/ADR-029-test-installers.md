# ADR-029: instaladores de teste no CI

**Status:** aceito · **Data:** 2026-10-15

## Contexto

Quem quer testar precisa hoje compilar o app (Rust, Node, WebKitGTK). O mantenedor e outros
testadores precisam de algo que se instala com dois cliques.

## Decisão

- `@tauri-apps/cli` 2.12.1 (Apache-2.0 OR MIT, publicada em 2026-09-30) como dependência de
  desenvolvimento fixa de `apps/desktop/ui`.
- O CI gera, a cada merge na `main` (e em PRs de branches `ci/*`), um `.deb` (Ubuntu/Mint/Debian)
  e um instalador NSIS (Windows), publicados como **artefatos do GitHub Actions** por 14 dias.
  `actions/upload-artifact` fixada por SHA na v7.0.1 (a v7.0.2 tinha menos de 2 semanas).
- Não é release: sem assinatura (S-07), sem atualização automática (S-06), sem AppImage (precisa
  de ferramentas baixadas na hora do build).

## Consequências

- O Windows mostra o aviso do SmartScreen para instalador não assinado; o `.deb` instala sem
  repositório (`sudo apt install ./HelpMyDesk_*.deb`).
- Baixar artefatos exige estar logado no GitHub.

## Desinstalação (F2-UNINST)

- Windows: o desinstalador pergunta "Apagar também o histórico do HelpMyDesk neste computador?"
  (`windows/hooks.nsh`, `NSIS_HOOK_POSTUNINSTALL`); em desinstalação silenciosa, guarda os dados.
  Apaga `%APPDATA%` e `%LOCALAPPDATA%` do app. A chave da IA no Gerenciador de Credenciais sai
  pelo "Apagar tudo" do app (o desinstalador não mexe no cofre).
- Linux (`.deb`): `apt remove` não apaga dados da pasta pessoal (padrão dos pacotes). Para tirar
  tudo, use "Apagar tudo" no app antes de desinstalar.
