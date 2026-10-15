# Spike S-06: atualização (Windows e Linux)

**Data:** 2026-10-15 · **Fonte:** <https://v2.tauri.app/plugin/updater/> (lida nesta data)

## Fatos da documentação oficial

- O plugin de atualização do Tauri 2 atualiza **NSIS e MSI no Windows** e **AppImage no Linux**.
  Não cita `.deb` nem `.rpm`.
- A assinatura das atualizações é obrigatória ("This cannot be disabled"): par de chaves gerado
  por `tauri signer generate`; a pública vai no `tauri.conf.json`; a privada só em variável de
  ambiente no build (`TAURI_SIGNING_PRIVATE_KEY`, senha opcional).
- Manifesto estático JSON: `version`, `notes`, `pub_date` e `platforms` (`OS-ARCH` → `url` e
  `signature`). TLS obrigatório em produção.

## O que temos hoje

Instaladores de teste `.deb` e NSIS (ADR-029), sem assinatura de código nem atualização.

## Recomendação

| Plataforma | Caminho | Por quê |
|---|---|---|
| Windows | Plugin de atualização + NSIS; manifesto na release | É o caminho oficial e já usamos NSIS |
| Ubuntu/Mint/Debian | Manter o `.deb` e o app **avisar** quando houver versão nova (lê o mesmo manifesto e mostra "Há uma versão nova: baixe aqui"), sem instalar sozinho | O plugin não atualiza `.deb`; um repositório apt assinado exige chave GPG e hospedagem, melhor deixar para depois do primeiro release |
| AppImage (opcional) | Plugin de atualização | Só se houver demanda; AppImage não integra com o sistema como o `.deb` |

## Decisões que dependem do mantenedor

1. Gerar a chave de assinatura do atualizador (como a do kill switch: privada fora do repositório;
   no CI, como segredo do repositório).
2. Hospedar o manifesto: na release de cada versão (`latest.json`).
3. Repositório apt assinado agora ou depois do primeiro release (recomendação: depois).

## Riscos

- Perder a chave do atualizador impede atualizar quem já instalou: guardar com cópia offline.
- Sem assinatura de código (S-07), o Windows mostra o aviso do SmartScreen a cada versão.
