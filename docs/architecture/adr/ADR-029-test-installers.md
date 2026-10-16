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

- Windows: a tela de confirmação do desinstalador traz a caixa "Remover dados do programa", do
  próprio template do Tauri (instalador em português, `languages: ["PortugueseBR", "English"]`).
  Marcada, apaga `%APPDATA%` e `%LOCALAPPDATA%` do app no perfil de quem desinstala
  (`SetShellVarContext current`); desinstalação silenciosa guarda os dados. Até 16/10 havia também
  uma pergunta nossa (`windows/hooks.nsh`), removida porque repetia a caixa. Limite: quando uma
  conta padrão digita a senha de um administrador no UAC, "quem desinstala" é o administrador, e o
  histórico da pessoa fica; "Apagar tudo" no app sempre alcança o perfil certo. A chave da IA no Gerenciador de Credenciais sai
  pelo "Apagar tudo" do app (o desinstalador não mexe no cofre).
- Linux (`.deb`): `apt remove` não apaga dados da pasta pessoal (padrão dos pacotes). Para tirar
  tudo, use "Apagar tudo" no app antes de desinstalar.
