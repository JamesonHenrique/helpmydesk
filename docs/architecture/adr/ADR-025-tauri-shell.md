# ADR-025 — App desktop com Tauri 2, fora do workspace principal

- **Status:** Aceita (2026-10-15)
- **Contexto:** a ADR-004 escolheu Tauri v2 + Rust + Angular. A spec 013 cria a casca desktop.
  No Linux o Tauri precisa do WebKitGTK (`libwebkit2gtk-4.1-dev`), que exige `sudo` e ainda não
  está na máquina de desenvolvimento. O workspace principal precisa continuar compilando e sendo
  testado sem ele.

## Decisão

- **`tauri` =2.12.1 e `tauri-build` =2.7.1** (Apache-2.0 OR MIT), publicados em 2026-09-30. A
  linha 3 está em alfa e fica de fora.
- A casca fica em `apps/desktop/src-tauri`, com **workspace e `Cargo.lock` próprios**, excluída do
  workspace principal. Ela só registra comandos: toda a lógica e os testes ficam em `hmd-ipc` e
  `hmd-app`, sem Tauri.
- **Permissões por janela** (ACL do Tauri 2): `build.rs` declara os comandos; a janela `main` não
  recebe `allow-approve`; só a janela `approval` pode aprovar (INV-04).
- **CSP no `tauri.conf.json`**: só recursos do próprio app; IPC por `ipc:`; sem scripts remotos,
  sem frames e sem formulários. `freezePrototype` ligado.
- Os comandos são `async`, para as sondas de rede não travarem a janela.
- O CI compila e passa o clippy no Ubuntu (instala o WebKitGTK) e no Windows (WebView2).
  Localmente, o check cruzado `--target x86_64-pc-windows-gnu` funciona com o MinGW.
- Licenças de toda a árvore (443 pacotes): MIT, Apache-2.0, BSD, ISC, Zlib, Unicode-3.0 e
  MPL-2.0 (`cssparser`, `selectors`, `dtoa-short`, `option-ext`). Todas compatíveis com a
  GPL-3.0-or-later.

## Consequências

- Rodar o app localmente no Linux exige `sudo apt install libwebkit2gtk-4.1-dev`.
- Mais de 400 dependências: risco de cadeia de suprimentos. Mitigação: `cargo-deny` (licenças,
  RustSec, origem só crates.io; `deny.toml`) nos dois workspaces, no CI e em
  `tools/check-all.sh`; `npm audit` na interface.
- Os pacotes de instalação (NSIS, AppImage, .deb), a assinatura e o updater ficam para a F5
  (`bundle.active = false`).

## Revisar se

O Tauri 3 sair do alfa, ou se o spike S-01 mostrar problema do WebKitGTK nas distros-alvo.

## Adendo (15/10/2026): primeiro uso real

- O binário só embute a interface com a feature `custom-protocol` do Tauri (sem ela roda em modo
  dev e carrega `devUrl`). Ela é padrão no `Cargo.toml` do shell; para recarga ao vivo use
  `ng serve` + `cargo run --no-default-features`.
- O Tauri acrescenta nonces ao CSP; com nonce o navegador ignora `'unsafe-inline'`, e os estilos
  que o Angular injeta por componente eram bloqueados. `dangerousDisableAssetCspModification`
  desliga a modificação **só de `style-src`**; `script-src` continua controlado pelo Tauri
  (`'self'` + nonces). Risco aceito: CSS injetado não executa código.
- O teste de fumaça do CI agora exige a linha `helpmydesk: interface ready`, escrita quando a
  interface chama `startup` (antes só conferia que o processo ficava de pé).
