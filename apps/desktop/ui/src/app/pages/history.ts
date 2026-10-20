// SPDX-License-Identifier: GPL-3.0-or-later
// History of tickets kept on this computer.
import { Component, OnInit, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

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
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <div class="page-head">
        <p class="rule">Registro local · fica só neste computador</p>
        <h1 id="t">Histórico</h1>
        <p class="lead">
          Os atendimentos ficam só neste computador e somem sozinhos 180 dias depois de encerrados.
        </p>
      </div>
      @if (tickets().length > 0) {
        <p class="rule" aria-hidden="true">Mais recente primeiro</p>
        <ol class="timeline">
          @for (t of tickets(); track t.id; let i = $index) {
            <li [class.latest]="i === 0">
              <div class="row entry-row">
                <strong>{{ t.familyTitle }}</strong>
                <span class="small muted mono-note">{{ date(t.lastChangedAt) }}</span>
                <span class="pill {{ state(t).tone }}">{{ state(t).label }}</span>
              </div>
              @if (t.lastTrigger) {
                <span class="small muted">Último gatilho: {{ t.lastTrigger }}</span>
              }
            </li>
          }
        </ol>
      } @else {
        <div class="empty" role="status">
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <circle
              cx="8"
              cy="8"
              r="5.6"
              fill="none"
              stroke="currentColor"
              stroke-width="1.3"
              stroke-dasharray="1.6 2.2"
            />
            <path
              d="M8 5.4V8l2 1.4"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
          <p class="muted">Nenhum atendimento ainda.</p>
          <a class="btn primary" routerLink="/">Começar um atendimento</a>
        </div>
      }
    </section>
  `,
  styles: `
    .entry-row {
      gap: 0.4rem 1rem;
      margin-bottom: 0.15rem;
    }
    .mono-note {
      font-family: var(--font-mono);
      font-size: 0.8rem;
    }
    .entry-row .pill {
      margin-left: auto;
    }
    .empty {
      display: grid;
      justify-items: center;
      gap: 0.6rem;
      border: 1px dashed var(--line-strong);
      border-radius: var(--radius);
      padding: 2.2rem 1rem;
      background: var(--surface);
    }
    .empty svg {
      width: 2.2rem;
      height: 2.2rem;
      color: var(--technical-ink);
    }
    .empty p {
      margin: 0;
    }
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
