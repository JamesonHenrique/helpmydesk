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

Correções reais ainda não estão liberadas (dependem do S-02). Se em algum momento aparecer a janela
"Aprovar correção": nada vem marcado, o botão "Sim" só funciona depois de um instante, e "Não,
obrigado" fecha sem fazer nada.

## O que mandar de volta

Para cada problema: em que tela estava, o que fez, o que esperava e o que aconteceu (uma captura
de tela ajuda). Não mande o código de acesso do OpenCode nem o relatório com dados pessoais.
