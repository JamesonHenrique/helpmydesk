# ADR-028: cofre da chave da IA e plugins do shell

**Status:** aceito · **Data:** 2026-10-15

## Decisão

- `keyring =4.2.0` (MIT OR Apache-2.0, publicado em 2026-08-29), modo `v1`: Secret Service (via
  `zbus`) no Linux e Gerenciador de Credenciais no Windows. Só no shell (`apps/desktop/src-tauri`);
  o núcleo vê o trait `KeyVault`.
- `tauri-plugin-opener =2.7.0` (2026-09-29) e `tauri-plugin-clipboard-manager =2.4.0`
  (2026-09-26), Apache-2.0 OR MIT. Usados só por comandos nossos com comportamento fixo
  (`open_ai_console` abre uma URL constante; `ai_connect_from_clipboard` lê o código e o entrega
  ao núcleo). As permissões próprias dos plugins não estão em nenhuma capability, então a
  interface não abre URLs nem lê a área de transferência por conta própria.

## Por quê

No WebKitGTK, `navigator.clipboard.readText()` é bloqueado (verificado no app real em 15/10);
ler no Rust também evita que a chave passe pela interface.

## Licenças

No Windows o plugin de área de transferência usa `clipboard-win` e `error-code` (Boost
Software License 1.0, compatível com a GPL segundo a FSF): exceção no `deny.toml` só para eles.

## Consequências

Sem Secret Service (alguns desktops Linux mínimos), a chave fica só na memória e precisa ser
colada de novo a cada abertura.
