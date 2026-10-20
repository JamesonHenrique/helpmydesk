// SPDX-License-Identifier: GPL-3.0-or-later
// Stage E: what the core proposes. In the desktop app the decision happens in a separate
// approval window (INV-04); here the person reviews and asks for it.
import { Component, computed, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { IpcError } from '../api/types';
import { Session } from '../state/session';
import { PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-plan',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <div class="page-head">
        <p class="rule">Plano proposto · nada roda sem aprovação</p>
        <h1 id="t">O que eu proponho fazer</h1>
        <p class="lead">
          Compare cada etapa: o que faz, o risco, e como desfazer. Você escolhe tudo ou só algumas —
          a decisão é sua.
        </p>
      </div>
      @if (plan(); as p) {
        <ol class="steps plan-steps">
          @for (s of p.steps; track s.stepId; let i = $index) {
            <li class="card step">
              <div class="step-head">
                <span class="num" aria-hidden="true">{{ pad(i + 1) }}</span>
                @if (choosing()) {
                  <label class="option step-option">
                    <input
                      type="checkbox"
                      [checked]="chosen().has(s.stepId)"
                      (change)="toggle(s.stepId)"
                    />
                    <strong>{{ s.title }}</strong>
                  </label>
                } @else {
                  <strong>{{ s.title }}</strong>
                }
              </div>
              <dl class="facts small">
                <dt>Risco</dt>
                <dd>{{ s.risk }}</dd>
                <dt>Desfazer</dt>
                <dd>{{ s.undo }}</dd>
              </dl>
              @if (s.requiresElevation) {
                <p class="small warn elev-note">
                  <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path
                      d="M8 2 14.4 13.2H1.6Z"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.5"
                      stroke-linejoin="round"
                    />
                    <path d="M8 6.2v3.4M8 11.2v.2" stroke="currentColor" stroke-width="1.5" />
                  </svg>
                  O sistema vai pedir sua permissão (no Windows, a janela de controle de conta).
                </p>
              }
            </li>
          }
        </ol>
        <div class="card untouched" role="note" aria-label="O que não será tocado">
          <p class="rule">Fora de alcance · sempre</p>
          <p class="small">
            Não vou mexer em: seus arquivos, programas instalados, senhas do Wi-Fi.
          </p>
        </div>
        <div class="actions">
          <a class="btn" [routerLink]="['/diagnostico', code()]">Não, obrigado</a>
          <button type="button" class="btn" (click)="choosing.set(!choosing())">
            {{ choosing() ? 'Usar todos' : 'Escolher só algumas' }}
          </button>
          <button
            type="button"
            class="btn primary"
            [disabled]="busy() || (choosing() && chosen().size === 0)"
            (click)="ask()"
          >
            {{ api.real ? 'Revisar e aprovar' : 'Sim, pode fazer' }}
          </button>
        </div>
        <p class="small muted" aria-live="polite">{{ status() }}</p>
        @if (api.real) {
          <p class="small muted">
            A aprovação abre numa janela separada e vale só para este plano exato, por 10 minutos.
          </p>
        } @else {
          <details class="small">
            <summary>Protótipo: escolher o resultado da correção</summary>
            <select
              id="mo"
              [value]="state.mockOutcome()"
              (change)="state.mockOutcome.set($any($event.target).value)"
            >
              <option value="verified">Verificado</option>
              <option value="not_available">Correção ainda não disponível</option>
              <option value="elevation_not_available">Sem permissão de administrador</option>
              <option value="not_resolved">Não resolveu</option>
            </select>
          </details>
        }
      } @else {
        <p>
          Para este resultado eu não tenho correção para propor. Siga as orientações ou prepare um
          relatório.
        </p>
        <div class="actions"><a class="btn" routerLink="/relatorio">Preparar relatório</a></div>
      }
    </section>
  `,
  styles: `
    .plan-steps {
      list-style: none;
      padding: 0;
      margin: 0;
      display: grid;
      gap: 0.8rem;
    }
    .step-head {
      display: flex;
      align-items: baseline;
      gap: 0.7rem;
    }
    .step-head .num {
      font-family: var(--font-mono);
      font-size: 0.85rem;
      color: var(--technical-ink);
      letter-spacing: 0.08em;
      flex: none;
    }
    .elev-note {
      display: flex;
      align-items: flex-start;
      gap: 0.4rem;
      margin: 0.5rem 0 0;
    }
    .elev-note svg {
      width: 1rem;
      height: 1rem;
      flex: none;
      margin-top: 0.15rem;
    }
    .untouched {
      border: 1px dashed var(--line-strong);
      box-shadow: none;
      background: var(--technical-soft);
      max-width: 52rem;
    }
    .untouched .rule {
      color: var(--muted);
      margin-bottom: 0.2rem;
    }
  `,
})
export class PlanPage {
  readonly code = input<string>('', { alias: 'family' });
  protected readonly api = inject(CORE_API);
  protected readonly state = inject(PrototypeState);
  private readonly session = inject(Session);
  private readonly router = inject(Router);
  protected readonly plan = computed(() => this.session.diagnosis()?.plan ?? null);
  protected readonly choosing = signal(false);
  protected readonly chosen = signal<ReadonlySet<string>>(new Set());
  protected readonly busy = signal(false);
  protected readonly status = signal('');

  protected toggle(id: string): void {
    const next = new Set(this.chosen());
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    this.chosen.set(next);
  }

  protected pad(n: number): string {
    return String(n).padStart(2, '0');
  }

  protected async ask(): Promise<void> {
    const d = this.session.diagnosis();
    const p = this.plan();
    if (!d || !p) {
      return;
    }
    const ids = this.choosing() ? [...this.chosen()] : p.steps.map((s) => s.stepId);
    this.busy.set(true);
    this.status.set(this.api.real ? 'Aguardando sua decisão na janela de aprovação…' : '');
    try {
      const decision = await this.api.requestApproval(d.ticket.id, p, ids);
      if (decision === 'approved') {
        void this.router.navigate(['/resultado', this.code()]);
      } else {
        this.status.set('Tudo bem, não vou fazer nada. As orientações continuam valendo.');
      }
    } catch (e) {
      this.status.set(`Não consegui abrir a aprovação (${(e as IpcError).code}).`);
    } finally {
      this.busy.set(false);
    }
  }
}
