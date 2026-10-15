# ADR-022 — Dependências para sondas de rede e do registro do Windows

- **Status:** Aceita (2026-10-14)
- **Contexto:** as sondas do lote 2 (spec 005) precisam do gateway padrão e do estado das
  interfaces de rede, nas duas plataformas, e da lista de programas de inicialização no registro
  do Windows. A constituição proíbe processos (`ipconfig`, `ip route`) e o workspace proíbe
  `unsafe`.

## Decisão

- **`netdev` 0.46** (MIT; último push em 2026-09-14), só com a feature `gateway`. Oferece API
  segura para interfaces, endereços e gateway em Windows e Linux. Usamos apenas tipo,
  estado, presença de IPv4 válido e IP do gateway; MAC, nomes e descrições são ignorados.
- **`winreg` 0.56** (MIT; último push em 2026-03-14), só no Windows, para ler
  `…\CurrentVersion\Run` e `StartupApproved\Run` em modo leitura.
- O HTTP de conectividade usa `std::net::TcpStream` com requisição fixa: nenhum cliente HTTP.
- O gateway é testado por TCP nas portas 53, 80 e 443, porque ICMP exige privilégio. Uma conexão
  recusada também conta como resposta.

## Consequências

Duas dependências a mais, cada uma com o próprio `unsafe` interno: risco de cadeia de
suprimentos, a acompanhar com `cargo-deny` e auditoria.

## Revisar se

Um dos crates for arquivado ou tiver advisory, ou se o spike S-01 indicar APIs melhores.
