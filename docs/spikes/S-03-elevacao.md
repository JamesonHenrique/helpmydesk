# Spike S-03: helper do Windows (UAC) e D-Bus × `pkexec` no Linux

**Data:** 2026-10-16 · **Estado:** decidido para o catálogo de hoje; o diálogo do UAC em Windows
10/11 ainda precisa do teste manual (spec 020 T012).

## Windows: helper pelo UAC

| Pergunta | Resposta | Evidência |
|---|---|---|
| Como o app pede o UAC sem `unsafe`? | PowerShell 5.1 pelo caminho absoluto, `Start-Process -Verb RunAs -Wait -PassThru`, script fixo em `-EncodedCommand` | ADR-031, `crates/hmd-elevated/src/uac.rs` |
| O pedido cruza a linha de comando inteiro? | Sim, até o maior token que o protocolo aceita | Teste ponta a ponta no runner Windows (`crates/hmd-elevated/tests/uac.rs`): o helper decodifica o pedido e responde `PLAN_HASH_MISMATCH`, de propósito |
| Como o resultado volta? | Pelo código de saída (o UAC não devolve o stdout) | ADR-031 |
| O helper chega à pasta protegida? | Sim, como sidecar do instalador por máquina | Job de instaladores confere com `7z l` (spec 020 T015) |
| O diálogo aparece, e "Não" vira `ELEVATION_DENIED`? | **A confirmar** | O runner do CI é administrador sem diálogo; roteiro manual, seção 7 |

## Linux: L1 (D-Bus) basta para o catálogo de hoje

O plano v1 prefere L1 (APIs D-Bus do sistema, com a política polkit da distro decidindo) e deixa
L2 (helper próprio via `pkexec`) só para ação sem API equivalente.

| Correção (catálogo ou candidata) | Via L1 | O que foi observado |
|---|---|---|
| `net.dns.flush_cache` | `org.freedesktop.resolve1.Manager.FlushCaches` | Ubuntu 24.04: funciona **sem** privilégio e sem sessão; não existe ação polkit para isso (S-02, rodada 1). Debian 13: nenhuma ação `resolve1.*` instalada (plano v1, Apêndice A) |
| Reconectar/reativar a rede (candidata) | NetworkManager, `network-control` / `enable-disable-network` | Debian 13: `yes` em sessão ativa (plano v1, Apêndice A) |
| Reiniciar serviço do sistema (candidata) | `org.freedesktop.systemd1`, `manage-units` | Debian 13: `auth_admin_keep` (pede senha) |
| Reiniciar áudio da sessão (candidata) | `systemctl --user` | Sem polkit |

**Decisão:** nenhuma correção do catálogo, nem das candidatas medidas, precisa de L2. O Linux segue
**sem `pkexec`** (spec 020 FR-015), e o `UnavailableLauncher` continua sendo o lançador do Linux.
Uma ação com `auth_admin*` passa pela política do sistema (o próprio sistema pede a senha), sem
código privilegiado do projeto. Se um dia uma correção não tiver API D-Bus, ela reabre o L2 com o
mesmo protocolo e a mesma allowlist do helper do Windows.

## O que falta

- Rodada 2 do S-02 em Ubuntu e Mint com sessão gráfica: confirma `FlushCaches` e as políticas do
  NetworkManager nessas distros.
- Teste manual do UAC em Windows 10/11 (T012).
