// SPDX-License-Identifier: GPL-3.0-or-later
// Stages F–H: runs the approved plan in the core and says honestly what happened.
import { Component, OnInit, inject, input, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { IpcError, OutcomeDto } from '../api/types';
import { Session } from '../state/session';

/** Plain words for the refusals a person can actually meet here (spec 016). */
const WHY: Record<string, string> = {
  OBSERVE_ONLY: 'O modo "só observar" está ligado, então eu não mudo nada. Nada foi alterado.',
  CAPABILITY_DISABLED: 'Esta correção foi desligada pelo projeto por segurança. Nada foi alterado.',
  NO_PENDING_PLAN:
    'O plano mudou ou passou do prazo. Nada foi alterado; comece de novo para eu olhar outra vez.',
};

@Component({
  selector: 'app-result',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      @if (running()) {
        <h1 id="t">Fazendo a correção…</h1>
        <p class="muted" aria-live="polite">
          Só o que você aprovou. Depois eu confiro se resolveu.
        </p>
      } @else if (error(); as e) {
        <h1 id="t">Não consegui fazer a correção</h1>
        <p class="notice">{{ why(e.code) }}</p>
        <div class="actions">
          <a class="btn primary" routerLink="/relatorio">Preparar relatório</a>
        </div>
      } @else if (answer() === 'yes') {
        <h1 id="t">Resolvido</h1>
        <div class="card">
          <dl class="facts">
            <dt>O problema</dt>
            <dd>{{ session.diagnosis()?.title }}</dd>
            <dt>O que fiz</dt>
            <dd>A correção que você aprovou, e conferi o resultado.</dd>
            <dt>Se acontecer de novo</dt>
            <dd>Abra o HelpMyDesk e escolha a mesma opção.</dd>
          </dl>
        </div>
        <div class="actions"><a class="btn primary" routerLink="/">Voltar ao início</a></div>
      } @else if (outcome(); as o) {
        @switch (o.kind) {
          @case ('verified') {
            <h1 id="t">Conferi e parece resolvido</h1>
            <p class="ok">
              <strong>Verificado:</strong> a verificação não encontra mais o problema.
            </p>
            @if (answer() === 'no') {
              <p>
                Entendi. Isso eu não consigo resolver com segurança; posso preparar um relatório.
              </p>
              <div class="actions">
                <a class="btn primary" routerLink="/relatorio">Preparar relatório</a>
              </div>
            } @else {
              <p>Para você também voltou a funcionar?</p>
              <div class="actions">
                <button type="button" class="btn" (click)="confirm(false)">Ainda não</button>
                <button type="button" class="btn primary" (click)="confirm(true)">
                  Sim, voltou
                </button>
              </div>
            }
          }
          @case ('not_available') {
            <h1 id="t">Essa correção ainda não está disponível</h1>
            <p>
              Esta versão do HelpMyDesk ainda não faz essa correção sozinha.
              <strong>Nada foi alterado.</strong> Siga as orientações abaixo ou leve o relatório a
              alguém.
            </p>
            <ol class="steps">
              @for (s of session.diagnosis()?.steps ?? []; track $index) {
                <li>{{ s }}</li>
              }
            </ol>
            <div class="actions">
              <a class="btn primary" routerLink="/relatorio">Preparar relatório</a>
            </div>
          }
          @case ('elevation_not_available') {
            <h1 id="t">Preciso de permissão de administrador</h1>
            <p>
              Esta correção só pode ser feita por um auxiliar instalado com permissão de
              administrador, e esta instalação do HelpMyDesk não tem esse auxiliar.
              <strong>Nada foi alterado.</strong>
            </p>
            <p class="small">
              Peça a alguém que cuide deste computador para instalar o HelpMyDesk "para todos os
              usuários". Se preferir, siga as orientações abaixo ou leve o relatório.
            </p>
            <ol class="steps">
              @for (s of session.diagnosis()?.steps ?? []; track $index) {
                <li>{{ s }}</li>
              }
            </ol>
            <div class="actions">
              <a class="btn primary" routerLink="/relatorio">Preparar relatório</a>
            </div>
          }
          @default {
            <h1 id="t">
              {{
                o.trigger === 'REP-04'
                  ? 'Não consegui confirmar o resultado'
                  : 'A correção não resolveu'
              }}
            </h1>
            <p>
              @if (o.trigger === 'REP-04') {
                Não sei se a correção chegou ao fim. Por segurança, não vou repetir sozinho.
              } @else {
                Conferi de novo e o problema continua.
              }
              Isso eu não consigo resolver com segurança.
            </p>
            <div class="actions">
              <a class="btn primary" routerLink="/relatorio">Preparar relatório</a>
            </div>
          }
        }
      }
    </section>
  `,
})
export class ResultPage implements OnInit {
  readonly code = input<string>('', { alias: 'family' });
  private readonly api = inject(CORE_API);
  protected readonly session = inject(Session);
  protected readonly running = signal(true);
  protected readonly outcome = signal<OutcomeDto | null>(null);
  protected readonly error = signal<IpcError | null>(null);
  protected readonly answer = signal<'yes' | 'no' | null>(null);

  async ngOnInit(): Promise<void> {
    const d = this.session.diagnosis();
    if (!d?.plan) {
      this.error.set({ code: 'NO_PENDING_PLAN', message: '' });
      this.running.set(false);
      return;
    }
    try {
      const o = await this.api.execute(d.ticket.id, d.plan.planId);
      this.outcome.set(o);
      this.session.outcome.set(o);
    } catch (e) {
      this.error.set(e as IpcError);
    } finally {
      this.running.set(false);
    }
  }

  protected why(code: string): string {
    return WHY[code] ?? `Algo deu errado do meu lado (${code}).`;
  }

  protected async confirm(fixed: boolean): Promise<void> {
    const d = this.session.diagnosis();
    if (!d) {
      return;
    }
    try {
      await this.api.confirm(d.ticket.id, fixed);
      this.answer.set(fixed ? 'yes' : 'no');
    } catch (e) {
      this.error.set(e as IpcError);
    }
  }
}
