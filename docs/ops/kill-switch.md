# Kill switch: gerar a chave e publicar um arquivo

O kill switch permite desligar uma capacidade do catálogo (ou um modelo de IA) sem lançar uma
versão nova (spec 014). O app só aceita arquivos assinados por uma chave compilada nele
(`TRUSTED_KEYS` em `crates/hmd-killswitch/src/lib.rs`). A chave do projeto foi gerada em
15/10/2026 (pública: `0e495f7a74cee62c7bdf1a72bb4b962ee45757238da6d0edb9161c4f3f097b47`). A
privada está só no computador do mantenedor, em `~/.config/helpmydesk-maintainer/` (permissão
600): **faça uma cópia de segurança offline**; perder a chave obriga a lançar versão com chave nova.

Comandos conferidos com OpenSSL 3.5.7 em 2026-10-15.

## 1. Gerar a chave (uma vez, só o mantenedor)

Num computador confiável e **fora do repositório**:

```sh
openssl genpkey -algorithm ed25519 -out helpmydesk-killswitch.pem
openssl pkey -in helpmydesk-killswitch.pem -pubout -outform DER | tail -c 32 | od -An -tx1 | tr -d ' \n'
```

- O arquivo `.pem` é a chave privada. Guarde em local seguro e com cópia. Ela **nunca** vai para o
  Git, para o CI ou para uma conversa.
- A segunda linha imprime a chave pública (64 caracteres hex). Ela entra em `TRUSTED_KEYS`, como
  bytes, num PR revisado.

## 2. Escrever o arquivo

`disabled-capabilities.json`, exatamente com estes campos:

```json
{"format":1,"issued_at":1760400000,"expires_at":1763000000,"disabled_capabilities":["net.dns.flush_cache"],"disabled_models":[]}
```

- `issued_at` precisa ser maior que o do último arquivo publicado. O app recusa arquivos mais
  antigos, para impedir que alguém republique um arquivo velho e mais permissivo.
- `expires_at` é a data a partir da qual o arquivo deixa de ser aceito. Depois de aceito, as
  desativações continuam valendo, inclusive sem internet, até um arquivo mais novo substituí-lo.
- Campos a mais fazem o app recusar o arquivo.

## 3. Assinar e conferir

```sh
openssl pkeyutl -sign -inkey helpmydesk-killswitch.pem -rawin \
  -in disabled-capabilities.json -out sig.bin
od -An -tx1 sig.bin | tr -d ' \n' > disabled-capabilities.json.sig
cargo run -q -p hmd-killswitch --example verify -- <chave-publica-hex> \
  disabled-capabilities.json disabled-capabilities.json.sig
```

O `verify` precisa imprimir `OK: ...`. Qualquer byte alterado depois da assinatura dá
`REFUSED: KILLSWITCH_BAD_SIGNATURE`.

## 4. Publicar

Os dois arquivos ficam na release `kill-switch` (não é uma versão do app; ADR-030). O app baixa
de `https://github.com/JamesonHenrique/helpmydesk/releases/download/kill-switch/disabled-capabilities.json`
(e `.sig`) toda vez que abre, em segundo plano. Para trocar o arquivo:

```sh
gh release upload kill-switch disabled-capabilities.json disabled-capabilities.json.sig --clobber
```

Guarde a cópia assinada em `ops/killswitch/` por PR, para ficar o histórico. O primeiro arquivo
(15/10/2026, nada desligado) vence em 13/04/2027: **assine um novo antes disso** (um arquivo
vencido não é aceito por quem ainda não tinha o anterior).
