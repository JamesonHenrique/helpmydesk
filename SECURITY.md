# Política de segurança

## Como reportar uma vulnerabilidade

**Não abra issue pública.** Use o reporte privado do GitHub:
[Security → Report a vulnerability](https://github.com/JamesonHenrique/helpmydesk/security/advisories/new).

Inclua: versão/commit, sistema operacional, passos para reproduzir e impacto.

Meta inicial de primeira resposta: 7 dias corridos.

## Escopo de especial interesse

- Qualquer forma de a IA (ou texto controlado por terceiros, como logs e nomes de arquivos)
  causar execução fora do catálogo ou fora da política.
- Contornar a janela de aprovação ou o vínculo da aprovação ao hash do plano.
- Elevação de privilégio via helper (Windows/UAC) ou polkit/`pkexec` (Linux).
- Vazamento de dados pessoais para a IA ou no relatório exportado.
- Atualização, manifesto ou kill switch adulterados aceitos pelo app.

## Resposta

Contenção via kill switch assinado (capacidade ou modelo desativado), correção, release
assinada e GitHub Security Advisory.

## Versões suportadas

Pré-alfa: apenas o branch `main`.
