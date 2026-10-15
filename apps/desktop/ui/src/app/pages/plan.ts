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
      <h1 id="t">O que eu proponho fazer</h1>
      @if (plan(); as p) {
        <ol class="steps">
          @for (s of p.steps; track s.stepId) {
            <li class="card">
              @if (choosing()) {
                <label class="option">
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
              <dl class="facts small">
                <dt>Risco</dt>
                <dd>{{ s.risk }}</dd>
                <dt>Desfazer</dt>
                <dd>{{ s.undo }}</dd>
              </dl>
              @if (s.requiresElevation) {
                <p class="small warn">
                  O sistema vai pedir sua permissão (no Windows, a janela de controle de conta).
                </p>
              }
            </li>
          }
        </ol>
        <p class="small">Não vou mexer em: seus arquivos, programas instalados, senhas do Wi-Fi.</p>
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
