# ADR-026 — Kill switch assinado com Ed25519

- **Status:** Aceita (2026-10-15)
- **Contexto:** o plano v1 (VII.3, IX.2) prevê `disabled-capabilities.json` assinado, aceito só
  se for mais novo que o último, para desligar capacidades e modelos sem uma versão nova.

## Decisão

- **`ed25519-dalek` =3.0.0** (BSD-3-Clause, 2026-07-06; dalek-cryptography, ativo), sem as
  features padrão, só com `zeroize`. Traz `curve25519-dalek` 5 (BSD-3), `ed25519`, `signature`,
  `subtle` e `zeroize`, e reaproveita o mesmo `sha2` 0.11 da ADR-024.
- **`serde_json` =1.0.151** (MIT OR Apache-2.0) passa a ser dependência normal no
  `hmd-killswitch`, porque o contrato do plano é JSON.
- Assinatura destacada, em hex, sobre os bytes exatos do arquivo, sem canonicalização: o que foi
  assinado é o que é lido. O formato é estrito (`deny_unknown_fields`, `format: 1`, até 64 KiB),
  e nada é interpretado antes de a assinatura ser válida.
- O crate é puro: quem chama traz o arquivo e o `now`. O download do GitHub entra depois.
- **A chave privada é do mantenedor** e é gerada com OpenSSL (`docs/ops/kill-switch.md`). O
  projeto não gera nem guarda chave. Enquanto `TRUSTED_KEYS` estiver vazio, todo arquivo é
  recusado.
- Uma vez aceito, o arquivo continua valendo depois de `expires_at`, porque desligar é o lado
  seguro. Só um arquivo mais novo o substitui.

## Consequências

Perder a chave privada impede publicar kill switches até sair uma versão com a chave nova. A
rotação de chave fica para a F5 ("rotação de chave ensaiada").

## Revisar se

O `ed25519-dalek` tiver advisory, ou se o updater do Tauri passar a oferecer uma assinatura que
sirva aos dois usos.
