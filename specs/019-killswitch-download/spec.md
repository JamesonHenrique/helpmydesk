# Spec 019 — Download do kill switch

**Status:** implementada · **Roadmap:** SPEC-014a · **Data:** 2026-10-15

- FR-001 Ao abrir, o app baixa `disabled-capabilities.json` e `.sig` da release `kill-switch`,
  em segundo plano, sem travar a interface.
- FR-002 Só aplica arquivo assinado pela chave do projeto, mais novo que o último e não vencido.
- FR-003 Sem internet ou com falha, nada muda; o último aceito continua valendo.
- FR-004 A tela "Sobre" avisa da consulta ao GitHub.

Verificado em 15/10: primeiro arquivo publicado (nada desligado, vence em 13/04/2027); o app real
baixou, verificou e aceitou ("kill switch checked (0 off)"). Teste automatizado com o arquivo
assinado de fixture e com assinatura forjada.
