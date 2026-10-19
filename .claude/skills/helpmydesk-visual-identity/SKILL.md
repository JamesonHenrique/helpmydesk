---
name: helpmydesk-visual-identity
description: Identidade visual aprovada do HelpMyDesk ("Oficina editorial contemporânea" v1.0) — como aplicar os tokens, logos e regras de marca ao aplicar/alterar a UI em apps/desktop. Use quando estiver criando ou revisando telas, estilos, favicons ou qualquer elemento visual do HelpMyDesk.
---

# Identidade visual HelpMyDesk — Oficina editorial contemporânea

Fontes da verdade (nesta ordem ao codificar):

1. **Operacional:** `apps/desktop/ui/src/styles.css` (sendo a única fonte aplicada na UI).
2. **Referência de tokens:** `branding/tokens.json` e `branding/tokens.css` (devem permanecer sincronizados).
3. **Assets de marca:** `branding/manifest-assets.json` lista o que existe e o que não foi fornecido.
4. **Logos:** `branding/logo-horizontal.svg`, `branding/logo-monochrome.svg`, `branding/brand-mark.svg`.

## Direção

Tema: **oficina de suporte editorial** — papel quente (#F3F0E6), cartões claros com sombra
dura (`3px 3px 0 var(--line)`), textura pontilhada sutil no fundo, marca em terceiro
acento (coral #FF6C4D) e o "ponto de verificação" (lime #D8F16A) reservado a
seleção/foco/verificação. Tagline: "Seu computador, sem mistério."

Princípios que a UI deve transmitir: entender primeiro, corrigir com cuidado, explicar
antes de agir, pedir autorização, verificar o resultado.

## Regras codificadas

- **Não inventar cor fora dos tokens.** Novas cores exigem edição de tokens.json + tokens.css + styles.css na mesma mudança (e nota no PR).
- Nomes operacionais: `--bg/--surface/--surface-raised/--fg/--muted/--line/--line-strong/--accent/--accent-fg/--accent-soft/--coral/--checkpoint/--ok/--warn/--bad/--focus/--texture-dot/--font/--font-mono/--font-size/--control-min-height/--focus-width/--motion-quick`.
- `--font-size` é **runtime**: `app.ts` escreve no `documentElement`; nunca fixar `html { font-size }` sem o `var(--font-size, 1.0625rem)`.
- Detalhes de marca (coral) não entram em texto pequeno sobre papel; checkpoint não substitui `--focus`.
- Tema escuro: manter os valores de `branding/tokens.css` (media query + `:root[data-theme='dark']`).
- Orçamento: `anyComponentStyle` w4kB/e8kB — estilos de shell em `app.css` compactos, identidade global em `styles.css`.
- Regra CSS existente de `prefers-reduced-motion: reduce` (mata animações) permanece.

## Verificações pós-mudança de UI

```sh
cd apps/desktop/ui && npx ng test --watch=false
cd apps/desktop/ui && npx prettier --check "src/**/*.{ts,html,css}"
cd apps/desktop/ui && npx ng build
git diff --check
```

Rode o prettier **de dentro de `apps/desktop/ui/`** — na raiz do repo o npx resolve
outra versão (3.9.10 vs 3.9.9 do projeto) e acusa erros de formato falsos.

A11y: os testes unitários rodam axe-core em todas as telas (color-contrast desativado lá; a
cobertura de contraste vem dos tokens revisados + revisão visual manual no app rodando).
Execução honesta: nunca reportar "verificado" passos que não rodaram.

## Revisores

`.opencode/agents/brand-reviewer.md`, `accessibility-reviewer.md` e
`safety-boundary-reviewer.md` são subagentes read-only. Comandos:`/brand-audit` e `/ui-verify`.
