# Spike S-02: matriz de privilégio das correções

**Data:** 2026-10-16 · **Rodada:** 1 de 2 (CI). A rodada 2 é manual, em Windows 10/11 e Ubuntu/Mint
com sessão gráfica, e é ela que decide.

## Como foi medido

Workflow `.github/workflows/spike-s02.yml`, executado em
<https://github.com/JamesonHenrique/helpmydesk/actions/runs/38936781071>:

- **Windows:** conta local nova (`hmdstd`), só no grupo `Users`, criada no próprio job. Cada comando
  roda como ela (`Start-Process -Credential`), e o `whoami /groups` confirma o token: **Medium
  Mandatory Level**, sem `BUILTIN\Administrators`. Os mesmos comandos rodam depois como o
  administrador do runner, para comparar.
- **Linux:** usuário novo sem `sudo` e sem sessão (`sudo -u hmdstd`), além do usuário do runner
  sem `sudo`. As políticas do polkit foram lidas com `pkaction --verbose`.

## Resultado: `net.dns.flush_cache` (a única correção do catálogo)

| Sistema | Comando | Conta padrão | Administrador/root |
|---|---|---|---|
| Windows Server 2025 Datacenter | `ipconfig /flushdns` | **funciona** (saída 0, "Successfully flushed the DNS Resolver Cache.") | funciona |
| Windows Server 2025 Datacenter | `Clear-DnsClientCache` | **funciona** (saída 0, sem erro) | funciona |
| Ubuntu 24.04.5 LTS (systemd-resolved) | `resolvectl flush-caches` | **funciona** (saída 0), mesmo sem sessão | funciona |
| Ubuntu 24.04.5 LTS | D-Bus `org.freedesktop.resolve1.Manager.FlushCaches` | **funciona** (saída 0) | funciona |

No Ubuntu 24.04 **não existe** a ação de polkit `org.freedesktop.resolve1.flush-caches`. As ações de
`resolve1` que existem são `register-service`, `revert`, `set-default-route`, `set-dns-over-tls`,
`set-dns-servers`, `set-dnssec`, `set-dnssec-negative-trust-anchors`, `set-domains`, `set-llmnr`,
`set-mdns` e `unregister-service`. O `FlushCaches` não passa por polkit nessa versão.

## O que isso indica (hipótese, a confirmar na rodada 2)

- A limpeza do cache de DNS **não precisa de elevação** nas duas plataformas medidas. Se a rodada 2
  confirmar, `net.dns.flush_cache` passa a `requires_elevation = false`, roda no processo de usuário
  (sem UAC e sem helper) e pode descer de N3 para N2, como o plano v1 já previa (`N2*`).
- O Windows Server não é o Windows 10/11 de quem usa o app; o comportamento do serviço DNS Client
  costuma ser o mesmo, mas isso é justamente o que a rodada 2 mede.
- No Linux, Mint e Debian 13 podem trazer outra versão do systemd e outra política. O Debian 13
  medido em 14/10 também não tinha ação `resolve1.*` para cache (plano v1, Apêndice A).

## Rodada 2 (manual, o que falta)

Em cada máquina, com uma conta **padrão** (sem administrador) e sessão gráfica normal:

- Windows 10 22H2 e Windows 11: `ipconfig /flushdns` num Prompt de Comando comum (não "Executar
  como administrador"). Anote a mensagem.
- Ubuntu e Mint (versões vigentes): `resolvectl flush-caches; echo $?` e
  `pkaction | grep resolve1`. Se não houver `systemd-resolved` (`systemctl is-active
  systemd-resolved`), anote: a correção não se aplica a essa máquina.

Mande o texto da saída. Com isso o catálogo muda (revisão de segurança, como manda o ADR-031).

## O que não foi medido

`net.dhcp.renew` e `net.adapter.restart` (candidatas do plano v1) não foram rodadas no CI: elas
derrubariam a rede do próprio runner. Ficam para a rodada 2, em máquina de teste.
