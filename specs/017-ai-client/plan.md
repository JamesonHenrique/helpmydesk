# Plano — spec 017

- Crate novo `crates/hmd-ai`: `request_body`, `parse_answer`, `classify`, trait `Transport`
  (falso nos testes) e `UreqTransport` (real).
- Dependência nova: `ureq =3.4.2` com `rustls` (provedor `ring`, raízes `webpki-roots`). ADR-027.
- `deny.toml`: exceção de licença só para `webpki-roots` (CDLA-Permissive-2.0, dados).
