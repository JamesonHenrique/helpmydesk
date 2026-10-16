# Spike S-07: assinatura gratuita de código para projeto open source

**Data:** 2026-10-16 · Fontes lidas nesta data (lista no fim). O que não foi confirmado em fonte
oficial está marcado como **não verificado**.

## Por que importa

Sem assinatura, o Windows mostra "editor desconhecido" no UAC do helper elevado (ADR-031) e o
SmartScreen avisa no instalador. Assinar troca o "editor desconhecido" pelo nome de quem assina, mas
o SmartScreen continua avisando até a reputação se formar (semanas e muitas instalações, segundo a
Microsoft), assinado ou não.

## Opções

| Opção | Custo | Serve para um mantenedor no Brasil? | GitHub Actions |
|---|---|---|---|
| **SignPath Foundation** (programa OSS) | Grátis | **Talvez.** Exige licença OSI sem licença comercial dupla, binários só do código do repositório, MFA de todos, aprovação manual por release e projeto "já lançado na forma a ser assinada". A página não fala de equipe de uma pessoa (pede papéis de autor, revisor e aprovador): **não verificado**. | `signpath/github-action-submit-signing-request@v3`, build em runner hospedado pelo GitHub |
| Azure Artifact Signing (antigo Trusted Signing) | Pago (a partir de US$ 9,99/mês segundo a Microsoft Learn; página de preços **não verificada**) | **Não.** Pessoa física só nos EUA e no Canadá; o Brasil não está na lista de organizações | Sim |
| Certum (certificado open source) | **Não verificado** (não é grátis segundo fontes de terceiros) | Não verificado | Não verificado |
| Sem assinatura | Grátis | Sim | — |

Com a SignPath, o certificado é emitido em nome da **SignPath Foundation**, então o UAC mostraria
esse nome, e não "HelpMyDesk" (inferência a partir dos termos, não confirmada).

## Encaixe com o Tauri

- O Tauri 2 aceita `bundle.windows.signCommand` (comando local chamado para cada exe, com `%1` no
  lugar do arquivo). Segundo o changelog do bundler, os sidecars (`externalBin`, que é como o
  helper entra desde a T015) também são assinados.
- A SignPath assina por envio de artefato (sobe, assina, baixa), não por comando local. Assinar app,
  helper **e** o instalador exige duas etapas no CI: assinar os exes soltos, montar o NSIS com eles e
  assinar o NSIS. Não há exemplo pronto para Tauri na documentação lida.
- Prova de que deu certo: `signtool verify /pa` nos três arquivos.

## Linux

Grátis e sem aprovação de terceiros: chave OpenPGP própria. Para `.deb` avulso, publicar
`SHA256SUMS` assinado (`gpg --detach-sign`). Para um repositório apt, assinar o `Release`/`InRelease`
e o cliente usa `Signed-By:` com o caminho do keyring (nunca `apt-key`).

## Recomendação

1. Primeiro release pré-1.0 **sem** assinatura, com `SHA256SUMS` assinado por GPG e o aviso do
   SmartScreen explicado no guia. Isso também cumpre o "já lançado" da SignPath.
2. Depois, pedir a SignPath Foundation, a única via gratuita que pode servir no Brasil.
3. Descartar a Azure (inelegível) e a Certum até haver preço oficial.

## O que só o mantenedor pode fazer

1. Ativar MFA no GitHub (exigência da SignPath para todos do projeto).
2. Publicar no README uma seção "Code signing policy": papéis (mesma pessoa em todos, perguntar se é
   aceito), a atribuição "Free code signing provided by SignPath.io, certificate by SignPath
   Foundation" e o link da política de privacidade.
3. Preencher <https://signpath.org/apply>, dizendo que o projeto tem uma pessoa e que o instalador
   leva um helper elevado (o produto **não** faz acesso remoto; controle remoto é proibido pelas
   regras do projeto). Perguntar se os dois pontos são aceitos.
4. Se aprovado: criar o projeto na SignPath, guardar o token como segredo `SIGNPATH_API_TOKEN` no
   repositório e configurar a política com aprovação manual. A integração no CI é um PR à parte.
5. Gerar a chave GPG de release (privada fora do repositório, como a do kill switch).

## Fontes (lidas em 2026-10-16)

- <https://signpath.org/terms>, <https://signpath.org/>, <https://signpath.org/apply>
- <https://docs.signpath.io/trusted-build-systems/github>
- <https://github.com/SignPath/github-action-submit-signing-request>
- <https://v2.tauri.app/distribute/sign/windows/>, <https://v2.tauri.app/reference/config/>
- <https://v2.tauri.app/release/tauri-bundler/all-versions/>
- <https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart> (atualizada em 2026-10-08)
- <https://learn.microsoft.com/en-us/azure/artifact-signing/faq> (atualizada em 2026-10-06)
- <https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation>
  (atualizada em 2026-08-17)
- <https://wiki.debian.org/DebianRepository/UseThirdParty>
