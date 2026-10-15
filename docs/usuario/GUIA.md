# HelpMyDesk: guia para quem usa

O HelpMyDesk ajuda com problemas comuns do computador: internet, lentidão, falta de espaço, som,
microfone, câmera e programas que dão erro. Ele **olha**, **explica** em palavras simples e só
**muda algo com a sua permissão**. Quando não dá para resolver com segurança, prepara um
relatório para você levar a alguém.

> Versão de teste (pré-alfa). Ainda não faz correções no computador: só verifica e explica.

## Instalar

- **Ubuntu, Linux Mint, Debian:** baixe o arquivo `.deb` e instale com
  `sudo apt install ./HelpMyDesk_*.deb` (ou dê dois cliques nele).
- **Windows 10 e 11:** baixe o instalador `.exe` e abra. Como a versão de teste não é assinada, o
  Windows pode mostrar um aviso azul: clique em "Mais informações" e "Executar assim mesmo".

## Primeira vez

Na primeira abertura aparecem 6 passos curtos: o que o app faz e o que nunca faz, permissões
opcionais, quanto ele pode fazer sozinho (o padrão é **perguntar sempre**) e a ajuda inteligente
(opcional).

## Usar

1. Na tela inicial, escolha o que está acontecendo (por exemplo, **Internet**).
2. O app faz as verificações e mostra o que encontrou, com uma explicação.
3. Se houver uma correção conhecida, ela só acontece se você aprovar numa janela separada.
4. Se não der para resolver, use **"Preparar relatório para levar a alguém"**. O relatório fica no
   seu computador; você decide se e para quem entregar.

## Ajuda inteligente (opcional)

Com ela, você escreve o problema com as suas palavras ("a internet caiu desde ontem").

- Para ligar: **Configurações → Abrir o OpenCode**, entre com sua conta (gratuita), clique em
  **Copiar** e volte em **Colar o código e ligar**.
- O que vai para a IA aparece embaixo do campo antes de enviar, sem seu nome nem o nome do
  computador. A IA só escolhe o tipo de problema; ela não executa nada.
- Para desligar e apagar o código: **Configurações → Desligar e apagar o código**.

## Privacidade

- Tudo fica no seu computador. O app não envia dados para o projeto.
- Ao abrir, o app consulta o GitHub para saber se alguma correção foi desligada por segurança;
  nada seu vai junto.
- **Configurações → Ver tudo o que está guardado** mostra seus dados; **Apagar tudo** apaga o
  histórico, os relatórios, as permissões e o código da ajuda inteligente.

## Desinstalar

- **Windows:** desinstale pelo Painel de Controle; o desinstalador pergunta se apaga também o
  histórico.
- **Linux:** use **Apagar tudo** no app e depois `sudo apt remove help-my-desk`.
