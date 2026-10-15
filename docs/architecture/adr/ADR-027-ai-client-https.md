# ADR-027: cliente HTTPS da IA e allowlist de modelos

**Status:** aceito · **Data:** 2026-10-15

## Contexto

A IA (opcional) chama o OpenCode Zen com a chave da própria pessoa (ADR-007, ADR-020; Q-01
liberada pelo mantenedor em 15/10/2026). O núcleo não tem cliente HTTPS: as sondas de rede usam
só TCP e HTTP simples.

## Decisão

- `ureq =3.4.2` (MIT OR Apache-2.0, publicado em 2026-09-13), sem recursos padrão, só `rustls`:
  TLS em Rust com o provedor `ring` e as raízes da Mozilla (`webpki-roots`). Sem OpenSSL do
  sistema e sem runtime assíncrono. Configuração: só HTTPS, sem redirecionamento, 30 s, resposta
  de até 64 KiB.
- `webpki-roots` usa CDLA-Permissive-2.0 (licença permissiva para dados): exceção no
  `deny.toml` só para esse crate.
- Allowlist (`hmd_ai::ALLOWED_MODELS`), conferida em <https://opencode.ai/docs/zen/> em
  15/10/2026: `deepseek-v4.1-flash-free`, `space-bunny-free`, `longcat-2.5-preview-free`,
  `step-5-preview-free` (provedor com retenção zero e sem treino).
  Fora: Big Pickle, Exo Free, MiMo (dados podem melhorar o modelo); Cat Jam Free (guarda 30
  dias); Nemotron Free ("não envie dados pessoais"); Muse Spark Contributor (treina com os
  dados); Jev Free (política não informada).

## Consequências

- Os modelos grátis são "por tempo limitado": a allowlist precisa ser revista quando a lista do
  Zen mudar (o kill switch já pode desligar um modelo: `disabled_models`).
- Raízes da Mozilla embutidas: um certificado corporativo instalado só no sistema não é
  aceito. Se isso aparecer em campo, avaliar `rustls-platform-verifier`.
