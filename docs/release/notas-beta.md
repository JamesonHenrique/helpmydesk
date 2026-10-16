**Versão beta, para teste.** O HelpMyDesk ajuda a resolver problemas comuns do computador
(internet, som, câmera, lentidão) com explicações simples. Nesta versão ele **diagnostica e
orienta**; correções que mudam o sistema ainda não estão liberadas.

## Instalar

- **Windows 10/11:** baixe `HelpMyDesk_*_x64-setup.exe`. A instalação é para todos os usuários e
  pede a senha ou a confirmação de um administrador. O programa ainda **não é assinado**: o Windows
  pode mostrar "O Windows protegeu o computador". Clique em **Mais informações** e depois em
  **Executar assim mesmo**, só se o arquivo bater com o `SHA256SUMS` abaixo.
- **Ubuntu, Linux Mint e Debian:** baixe o `.deb` e rode `sudo apt install ./<arquivo>.deb`.

## Conferir o arquivo antes de instalar

Cada arquivo tem o seu código em `SHA256SUMS`:

- Linux: `sha256sum -c SHA256SUMS --ignore-missing`
- Windows (PowerShell): `Get-FileHash .\HelpMyDesk_*_x64-setup.exe` e compare com a linha do
  arquivo em `SHA256SUMS`.

Se houver `SHA256SUMS.asc`, ele é a assinatura do mantenedor:
`gpg --verify SHA256SUMS.asc SHA256SUMS`.

## Ajuda inteligente (opcional)

Vem desligada. Para ligar, use a sua própria conta do OpenCode; o app mostra antes exatamente o que
é enviado. Sem ela, o modo básico (menu de problemas) funciona sem internet.

## Problemas e sugestões

Abra uma issue no GitHub. Não cole códigos de acesso nem relatórios com dados pessoais.
