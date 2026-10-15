# Kill switch: gerar a chave e publicar um arquivo

O kill switch permite desligar uma capacidade do catálogo (ou um modelo de IA) sem lançar uma
versão nova (spec 014). O app só aceita arquivos assinados por uma chave compilada nele
(`TRUSTED_KEYS` em `crates/hmd-killswitch/src/lib.rs`). **Hoje essa lista está vazia**, então o
app recusa qualquer arquivo até o mantenedor gerar a chave.

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

Os dois arquivos vão para os GitHub Releases (plano v1, VII.3). O download pelo app ainda não
foi implementado: hoje o app só aceita o arquivo por `App::apply_kill_switch`.
