// SPDX-License-Identifier: GPL-3.0-or-later
// The approval window (desktop only). It is the only window whose capability allows `approve`
// (INV-04). Its texts come from the plan the core validated, never from the conversation.
import { Component, OnDestroy, OnInit, inject, input, signal } from '@angular/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

import { CORE_API } from '../api/core-api';
import { IpcError, PlanDto } from '../api/types';

/** "Sim" stays off this long after the plan appears, so a click aimed at another window (the main
 * window may reopen this one at any moment) cannot land on it (spec 016, S-04). */
export const APPROVAL_ARM_MS = 1000;

@Component({
  selector: 'app-approval',
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Aprovar correção</h1>
      @if (error(); as e) {
        <p class="notice">Este plano não está mais disponível ({{ e.code }}). Nada foi feito.</p>
      } @else if (done()) {
        <p aria-live="polite">{{ done() }}</p>
      } @else if (plan(); as p) {
        <p class="lead">Marque o que eu posso fazer. Nada acontece sem o seu "Sim".</p>
        @for (s of p.steps; track s.stepId) {
          <label class="option card">
            <input type="checkbox" [checked]="chosen().has(s.stepId)" (change)="toggle(s.stepId)" />
            <span>
              <strong>{{ s.title }}</strong
              ><br />
              <span class="small">Risco: {{ s.risk }}</span
              ><br />
              <span class="small">Desfazer: {{ s.undo }}</span>
              @if (s.requiresElevation) {
                <br /><span class="small warn">O sistema vai pedir sua permissão.</span>
              }
            </span>
          </label>
        }
        <p class="small muted">Plano {{ p.planHash.slice(0, 19) }}… · vale por 10 minutos.</p>
        <div class="actions">
          <button type="button" class="btn" (click)="decline()">Não, obrigado</button>
          <button
            type="button"
            class="btn primary"
            [disabled]="!armed() || chosen().size === 0"
            (click)="approve()"
          >
            Sim, pode fazer
          </button>
        </div>
      } @else {
        <p aria-live="polite">Carregando o plano…</p>
      }
    </section>
  `,
})
export class ApprovalPage implements OnInit, OnDestroy {
  readonly ticket = input<string>('');
  readonly planId = input<string>('', { alias: 'plan' });
  private readonly api = inject(CORE_API);
  protected readonly plan = signal<PlanDto | null>(null);
  protected readonly chosen = signal<ReadonlySet<string>>(new Set());
  protected readonly error = signal<IpcError | null>(null);
  protected readonly done = signal('');
  protected readonly armed = signal(false);
  private timer: ReturnType<typeof setTimeout> | undefined;
  /** Getting focus or becoming visible starts the wait again (a window opened behind another
   * one must not arrive already armed). */
  private readonly rearm = (): void => {
    this.armed.set(false);
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.armed.set(true), APPROVAL_ARM_MS);
  };

  async ngOnInit(): Promise<void> {
    try {
      const p = await this.api.plan(this.ticket(), Number(this.planId()));
      this.plan.set(p);
      // Nothing pre-selected: the choice is made here, never taken from the main window
      // (independent review, M2).
      this.chosen.set(new Set());
      this.rearm();
      window.addEventListener('focus', this.rearm);
      document.addEventListener('visibilitychange', this.rearm);
    } catch (e) {
      this.error.set(e as IpcError);
    }
  }

  ngOnDestroy(): void {
    clearTimeout(this.timer);
    window.removeEventListener('focus', this.rearm);
    document.removeEventListener('visibilitychange', this.rearm);
  }

  protected toggle(id: string): void {
    const next = new Set(this.chosen());
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    this.chosen.set(next);
  }

  protected async approve(): Promise<void> {
    if (!this.armed()) {
      return;
    }
    try {
      await this.api.approve(this.ticket(), Number(this.planId()), [...this.chosen()]);
      this.finish('Aprovado. Pode voltar para a janela principal.');
    } catch (e) {
      this.error.set(e as IpcError);
    }
  }

  protected async decline(): Promise<void> {
    try {
      await this.api.decline(this.ticket(), Number(this.planId()));
      this.finish('Tudo bem, nada será feito.');
    } catch (e) {
      this.error.set(e as IpcError);
    }
  }

  private finish(message: string): void {
    this.done.set(message);
    if (this.api.real) {
      void getCurrentWindow().close();
    }
  }
}
