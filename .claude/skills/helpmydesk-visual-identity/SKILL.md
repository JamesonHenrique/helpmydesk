---
name: helpmydesk-visual-identity
description: Identidade visual aprovada do HelpMyDesk ("Oficina editorial de precisão" v2.0) — como aplicar os tokens, logos, sidebar e regras de marca ao aplicar/alterar a UI em apps/desktop. Use quando estiver criando ou revisando telas, estilos, favicons ou qualquer elemento visual do HelpMyDesk.
---

# Identidade visual HelpMyDesk — Oficina editorial de precisão (v2.0)

Fontes da verdade (nesta ordem ao codificar):

1. **Operacional:** `apps/desktop/ui/src/styles.css` (a única fonte aplicada na UI).
2. **Referência de tokens:** `branding/tokens.json` e `branding/tokens.css` (devem permanecer sincronizados).
3. **Assets de marca:** `branding/manifest-assets.json` lista o que existe e o status de cada um.
4. **Logos:** `branding/logo-horizontal.svg`, `branding/logo-monochrome.svg`, `branding/brand-mark.svg`.
5. **Ilustração:** `apps/desktop/ui/public/branding/assistente-oficina.png` (mascote; Início no hero e Onboarding nos passos de abertura/conclusão apenas).

## Direção v2.0

Tema: **oficina editorial de precisão** — papel quente (#F3F0E6), superfícies com sombra dura
(`3px 3px 0 var(--line)`), textura pontilhada sutil no fundo, vermelho-sinal para ação
(#B73825 claro / #FF8066 escuro), coral como detalhe de marca (nunca texto pequeno sobre
papel) e o "ponto de verificação" (lime #D8F16A) reservado a seleção/foco/verificação.
Tagline: "Seu computador, sem mistério." (visível na sidebar).

Princípios que a UI deve transmitir: entender primeiro, corrigir com cuidado, explicar
antes de agir, pedir autorização, verificar o resultado.

### O que a v2 acrescentou (shell e escrita visual)

- **Sidebar persistente** (236px) com grupos Atendimento/Computador/Preferências, ícones
  `--rail-*`, item ativo em superfície carvão com marcador vermelho-sinal + ponto lime.
  Compacta para 64px ≤1099px (nomes ficam acessíveis a leitores de tela) e vira faixa
  superior com nomes visíveis <760px. Shell vive em `app.css` (budget w4kB/e8kB).
- **Escrita de página:** `.page-head` com rubrica técnica mono (`.rule`, ex. "Inspeção ·
  família …") + título `var(--fs-page)` + `.lead`. Display do Início usa
  `var(--fs-display)`. Prosa limitada com `.prose` (52rem); workspace até 64rem.
- **Superfícies variadas, não cards iguais:** `.card` (agivel), `.sheet` (ficha técnica,
  sem sombra), `.tile` (escolha com índice `01…` + ícone SVG), `.trail` (trilha de
  verificação numerada com linha pontilhada), `.timeline` (histórico), `.stamp` (selo
  rotate −2°, mono). Evitar card dentro de card sem motivo.
- **Selo "CONFERIDO"** (`-stamp`, aria-hidden quando redundante) só aparece em resultado
  tecnicamente sustentado ("Conferi e parece resolvido") e "CONFIRMADO POR VOCÊ" após a
  confirmação do usuário. Nunca decorativo, nunca em aprovação (janela séria, sem mascote).
- **Rubricas e índices em `--font-mono`** com `--technical-ink`.
- **Nota de retorno do shell** (`app.ts` → `.restore-note`, role="note"): histórico
  recriado/correção interrompida, compacta, sem banner gigante.

## Regras codificadas

- **Não inventar cor fora dos tokens.** Novas cores exigem edição de tokens.json +
  tokens.css + styles.css + este SKILL.md na mesma mudança.
- Nomes operacionais: `--bg/--surface/--surface-raised/--fg/--muted/--line/--line-strong/
  --accent/--accent-fg/--accent-soft/--coral/--checkpoint(--fg)/--technical-ink/
  --technical-soft/--rail-bg/--rail-fg/--rail-muted/--rail-line/--rail-active-bg/
  --rail-accent/--ok/--warn/--bad/--focus/--texture-dot/--font/--font-mono/--font-size/
  --fs-display/--fs-page/--control-min-height/--focus-width/--motion-quick`.
- `--fs-display` (hero Início) e `--fs-page` (títulos) são clamp: `clamp(2rem, 3.2vw,
  3.25rem)` e `clamp(1.7rem, 2.6vw, 2.35rem)`. `--font-size` é **runtime**: `app.ts` escreve
  no `documentElement`; nunca fixar `html { font-size }` sem `var(--font-size, 1.0625rem)`.
- `--technical-ink` (#496965 claro / #A2C7BF escuro) é tinta para rubricas, índices e
  material técnico; sem variantes aprovadas, não usar em texto longo. `--technical-soft`
  (#DDE8E3 / #2D413E) é só fundo suave.
- Detalhes de marca (coral) não entram em texto pequeno sobre papel; checkpoint não
  substitui `--focus`; `-accent` não é para texto pequeno no tema escuro sobre papel escuro
  (usa `-rail-accent` sobre sidebar).
- Tema escuro completo (inputs, bordas, hover, seleção, foco): manter os valores de
  `branding/tokens.css` (media query + `:root[data-theme='dark']`).
- Orçamento: `anyComponentStyle` w4kB/e8kB — estilos de shell em `app.css` compactos,
  identidade global em `styles.css`.
- Regras permanentes: `prefers-reduced-motion` mata animações (`.stamp` perde o rotate);
  `prefers-contrast: more` reforça bordas; skip-link lime; foco visível `3px`.
- Execução honesta: sem progressos fake (diagnóstico é indeterminado), sem métricas
  inventadas, protótipo sempre rotulado ("Protótipo: …", faixa `.proto`), textos
  user-facing em português simples, comentários de código em inglês.

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
`safety-boundary-reviewer.md` são subagentes read-only. Comandos: `/brand-audit` e `/ui-verify`.
