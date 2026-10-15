// SPDX-License-Identifier: GPL-3.0-or-later
// History of tickets kept on this computer.
import { Component, OnInit, inject, signal } from '@angular/core';

import { CORE_API } from '../api/core-api';
import { TicketDto } from '../api/types';

const STATES: Record<string, { label: string; tone: string }> = {
  open: { label: 'Aberto', tone: 'muted' },
  in_progress: { label: 'Em andamento', tone: 'warn' },
  awaiting_user: { label: 'Esperando você', tone: 'warn' },
  resolved_pending_confirmation: { label: 'Esperando sua confirmação', tone: 'warn' },
  resolved: { label: 'Resolvido', tone: 'ok' },
  unresolved: { label: 'Não consegui', tone: 'bad' },
  reopened: { label: 'Reaberto', tone: 'warn' },
  cancelled: { label: 'Cancelado', tone: 'muted' },
  abandoned: { label: 'Sem resposta', tone: 'muted' },
  closed: { label: 'Encerrado', tone: 'muted' },
};

@Component({
  selector: 'app-history',
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Histórico</h1>
      <p class="lead">
        Os atendimentos ficam só neste computador e somem sozinhos 180 dias depois de encerrados.
      </p>
      @for (t of tickets(); track t.id) {
        <div class="card row">
          <strong>{{ t.familyTitle }}</strong>
          <span class="small muted">{{ date(t.lastChangedAt) }}</span>
          <span class="pill {{ state(t).tone }}">{{ state(t).label }}</span>
          @if (t.lastTrigger) {
            <span class="small muted">({{ t.lastTrigger }})</span>
          }
        </div>
      } @empty {
        <p class="muted">Nenhum atendimento ainda.</p>
      }
    </section>
  `,
})
export class HistoryPage implements OnInit {
  private readonly api = inject(CORE_API);
  protected readonly tickets = signal<TicketDto[]>([]);

  async ngOnInit(): Promise<void> {
    const list = await this.api.history();
    this.tickets.set([...list].sort((a, b) => b.lastChangedAt - a.lastChangedAt));
  }

  protected state(t: TicketDto): { label: string; tone: string } {
    return STATES[t.state] ?? { label: t.state, tone: 'muted' };
  }

  protected date(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString('pt-BR');
  }
}
