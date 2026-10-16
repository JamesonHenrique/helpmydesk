# Laboratório de falhas (F2-LAB)

Cada cenário quebra **uma** coisa conhecida numa máquina virtual descartável, para conferir se o
HelpMyDesk acha o problema certo, e depois desfaz. Os scripts derrubam a rede, enchem o disco e
param o som: **nunca rode num computador de verdade**. Eles se recusam a rodar sem
`HMD_LAB_VM=1` e fora de uma máquina virtual detectada.

## Preparar

1. VM limpa: Windows 10 22H2 ou 11, e Ubuntu, Mint ou Debian 13 com sessão gráfica.
2. Instale o HelpMyDesk (instalador de teste do CI) e tire um **snapshot**.
3. Copie a pasta `lab/` para a VM.

## Rodar um cenário

- Linux: `HMD_LAB_VM=1 lab/linux/lab.sh <cenário> break`, use o app, depois
  `HMD_LAB_VM=1 lab/linux/lab.sh <cenário> restore`.
- Windows, num PowerShell **de administrador**: `$env:HMD_LAB_VM = '1'`, depois
  `.\lab\windows\lab.ps1 -Scenario <cenário> -Action break` e, no fim, `-Action restore`.

Se algo der errado, volte o snapshot.

| Cenário | O que quebra | Família no app | Achado esperado |
|---|---|---|---|
| `dns` | Servidor DNS que não responde (192.0.2.53, RFC 5737) | 1. Internet | `net.dns.resolution_failing` |
| `adapter` | Rede desligada | 1. Internet | `net.no_active_adapter` (ou `net.no_valid_ip`) |
| `disk` | Deixa só 4 GiB livres no disco | 3. Sem espaço | `storage.low_space` |
| `audio` | Para o PipeWire (Linux) ou o serviço Áudio do Windows | 4. Som, microfone ou câmera | Linux: `av.no_sound_server`; Windows: `av.no_output_device` (a confirmar) |
| `startup` | Oito programas de inicialização inofensivos | 2. Computador lento (com consentimento) | `startup.many_apps` |

## Registrar o resultado

Para cada cenário: sistema e versão, o que o app disse, se o achado esperado apareceu, se a
orientação faz sentido para uma pessoa leiga e se o `restore` deixou tudo como antes. É essa
evidência que os gates G-SEC e G-AUTO do plano v1 pedem antes de liberar uma correção.
