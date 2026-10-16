# Roteiro de teste manual (pré-alfa)

Para o mantenedor e para quem for testar. Leva uns 20 minutos. Anote o que estranhar, mesmo que
pareça pequeno: texto confuso, botão que não reage, demora, palavra técnica.

## Antes

- Linux: `sudo apt install libwebkit2gtk-4.1-dev` (uma vez).
- Abrir o app: siga "App desktop no Linux" no README.
- O histórico fica em `~/.local/share/io.github.jamesonhenrique.helpmydesk/`. Para começar do zero,
  use **Configurações → Apagar tudo** (não apague a pasta na mão com o app aberto).

## 1. Primeira vez (boas-vindas)

1. Na primeira abertura aparecem 6 passos. Leia cada um: algo ficou confuso?
2. Passo 3: as caixas de marcar estão fáceis de ver e de clicar?
3. Passo 4: o nível "Perguntar sempre" vem marcado.
4. Passo 5, **Ligar a ajuda inteligente** (opcional):
   - "Abrir o OpenCode" abre o navegador em opencode.ai.
   - Entre (ou crie a conta), copie o código de acesso e volte.
   - "Colar o código e ligar": deve aparecer "A ajuda inteligente está ligada".
   - Teste também colar algo errado (por exemplo, uma palavra qualquer): deve dizer que o código
     não funcionou, sem guardar nada.

## 2. Problema pelo menu (sem IA)

Para cada opção da tela inicial (Internet, Computador lento, Sem espaço, Som/microfone/câmera,
Erro ou programa): as verificações rodam, o resultado faz sentido para o seu computador agora?
A explicação está em palavras simples?

## 3. Problema escrito (com a IA ligada)

Na tela inicial, escreva por exemplo:
- "a internet caiu desde ontem" → deve ir para Internet.
- "meu disco está cheio" → Sem espaço.
- "ninguém me ouve na chamada" → Som, microfone ou câmera.
- "não sei o que é, está estranho" → deve perguntar ou mostrar o menu, sem chutar.

## 4. Relatório

Em qualquer resultado, "Preparar relatório para levar a alguém":
- O título faz sentido para o caso?
- Escreva algo no campo e gere. O texto tem seu nome de usuário ou o nome do computador? (Não
  deveria.)
- "Salvar arquivo": aparece "Salvei o relatório em Downloads/…"? O arquivo está lá?

## 5. Configurações e privacidade

- A lista do que cada verificação olha (e do que nunca guarda): está clara?
- Desligar a ajuda inteligente: a tela inicial volta ao modo básico?
- "Ver tudo o que está guardado" mostra seus dados? "Apagar tudo" pede confirmação antes?

## 6. Aprovação (só se aparecer)

Correções reais ainda não estão liberadas (dependem da rodada 2 do S-02, ver a seção 7). Se em algum
momento aparecer a janela "Aprovar correção": nada vem marcado, o botão "Sim" só funciona depois de
um instante, e "Não, obrigado" fecha sem fazer nada.

## 7. Instalação, UAC e desinstalação (só Windows 10/11, spec 020 T012)

Use o instalador de teste do CI (`helpmydesk-nsis`). Correções elevadas ainda não aparecem no app
(a lista de ações permitidas está vazia), então o UAC é testado chamando o helper direto, num
**PowerShell comum** (não "Executar como administrador"):

1. Instale. O Windows pede senha ou confirmação de administrador? A pasta
   `C:\Program Files\HelpMyDesk` tem `helpmydesk.exe` **e** `helpmydesk-elevated.exe`?
2. Rode:
   `$p = Start-Process "C:\Program Files\HelpMyDesk\helpmydesk-elevated.exe" -ArgumentList teste -Verb RunAs -Wait -PassThru; $p.ExitCode`
   - Aparece o UAC? Anote o "Editor" (esperado: "Desconhecido", o programa não é assinado).
   - Clique **Sim**. O número mostrado deve ser **11** (pedido recusado: `teste` não é um pedido
     válido). Nenhuma janela preta deve aparecer.
3. Rode de novo e clique **Não**. Deve aparecer um erro de "operação cancelada" (é o 1223 que o app
   lê como "a pessoa recusou").
4. Copie `helpmydesk-elevated.exe` para a pasta Downloads e rode o mesmo comando apontando para a
   cópia. Depois do **Sim**, o número deve ser **16** (helper fora da pasta protegida).
5. Desinstale pelo Painel de Controle. A tela de confirmação tem **uma** caixa, "Remover dados do
   programa", em português? Marque, conclua e confira que `C:\Program Files\HelpMyDesk` sumiu e que
   `%APPDATA%\io.github.jamesonhenrique.helpmydesk` e `%LOCALAPPDATA%\io.github.jamesonhenrique.helpmydesk`
   também sumiram.
6. Com uma conta **padrão**, rode também a rodada 2 do S-02 (`ipconfig /flushdns` num Prompt de
   Comando comum, ver `docs/spikes/S-02-privilegios.md`) e anote a mensagem.

## O que mandar de volta

Para cada problema: em que tela estava, o que fez, o que esperava e o que aconteceu (uma captura
de tela ajuda). Não mande o código de acesso do OpenCode nem o relatório com dados pessoais.
