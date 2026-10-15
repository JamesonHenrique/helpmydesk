// SPDX-License-Identifier: GPL-3.0-or-later
// History of tickets. In the real app these rows come from hmd-store; here they are examples.
import { Component } from '@angular/core';

interface Row {
  date: string;
  family: string;
  state: string;
  tone: string;
  what: string;
}

const EXAMPLES: Row[] = [
  {
    date: '14/10/2026',
    family: 'Internet',
    state: 'Resolvido',
    tone: 'ok',
    what: 'Limpei a memória de endereços da internet; conferi que os sites voltaram.',
  },
  {
    date: '10/10/2026',
    family: 'Som, microfone ou câmera',
    state: 'Não consegui',
    tone: 'warn',
    what: 'Não encontrei microfone ativo; relatório salvo.',
  },
  {
    date: '02/10/2026',
    family: 'Sem espaço',
    state: 'Encerrado',
    tone: 'muted',
    what: 'O disco tinha espaço; orientei a ver o pendrive.',
  },
];

@Component({
  selector: 'app-history',
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Histórico</h1>
      <p class="lead">
        Os atendimentos ficam só neste computador e somem sozinhos 180 dias depois de encerrados.
      </p>
      <p class="notice small">Exemplos do protótipo, não são dados seus.</p>
      @for (r of rows; track r.date) {
        <details class="card">
          <summary>
            <strong>{{ r.family }}</strong> · {{ r.date }} ·
            <span class="pill {{ r.tone }}">{{ r.state }}</span>
          </summary>
          <p>{{ r.what }}</p>
        </details>
      }
    </section>
  `,
})
export class HistoryPage {
  protected readonly rows = EXAMPLES;
}
