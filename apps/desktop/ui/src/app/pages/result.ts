// SPDX-License-Identifier: GPL-3.0-or-later
// Stages F (execution), G (verification) and H (closing). The four verification outcomes have
// distinct texts (Plan v1 III.13).
import { Component, OnDestroy, OnInit, computed, input, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { family } from '../data/catalog';

type Outcome = 'verified' | 'partial' | 'unknown' | 'worse';

const OUTCOMES: Record<Outcome, { title: string; text: string; tone: string }> = {
  verified: {
    title: 'Verificado',
    text: 'Conferi de novo e o problema não aparece mais.',
    tone: 'ok',
  },
  partial: {
    title: 'Melhorou em parte',
    text: 'Uma parte do problema sumiu, mas ainda encontro outra.',
    tone: 'warn',
  },
  unknown: {
    title: 'Não consegui confirmar',
    text: 'A correção terminou, mas não consegui conferir o resultado. Não vou repetir sozinho.',
    tone: 'warn',
  },
  worse: {
    title: 'Piorou: desfazendo',
    text: 'O resultado ficou pior que antes. Estou desfazendo o que fiz.',
    tone: 'bad',
  },
};

@Component({
  selector: 'app-result',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      @if (running()) {
        <h1 id="t">Fazendo a correção…</h1>
        <progress [value]="progress()" max="100" aria-label="Andamento"></progress>
        <p class="muted" aria-live="polite">
          Só o que você aprovou. Você pode parar o que ainda não começou.
        </p>
      } @else if (answer() === 'yes') {
        <h1 id="t">Resolvido</h1>
        <div class="card">
          <dl class="facts">
            <dt>O problema</dt>
            <dd>{{ fam()?.title }}</dd>
            <dt>O que fiz</dt>
            <dd>A correção que você aprovou, e conferi o resultado.</dd>
            <dt>Se acontecer de novo</dt>
            <dd>Abra o HelpMyDesk e escolha a mesma opção.</dd>
          </dl>
        </div>
        <div class="actions"><a class="btn primary" routerLink="/">Voltar ao início</a></div>
      } @else {
        <h1 id="t">Conferindo se resolveu…</h1>
        <div class="card" aria-live="polite">
          <p class="{{ current().tone }}">
            <strong>{{ current().title }}</strong>
          </p>
          <p>{{ current().text }}</p>
        </div>
        @if (outcome() === 'verified') {
          <p>Para você também voltou a funcionar?</p>
          <div class="actions">
            <button type="button" class="btn" (click)="answer.set('no')">Ainda não</button>
            <button type="button" class="btn primary" (click)="answer.set('yes')">
              Sim, voltou
            </button>
          </div>
        }
        @if (answer() === 'no' || outcome() !== 'verified') {
          <p>
            Isso eu não consigo resolver com segurança. Posso preparar um relatório para você levar
            a alguém.
          </p>
          <div class="actions">
            <a class="btn primary" routerLink="/relatorio">Preparar relatório</a>
          </div>
        }
        <details class="small">
          <summary>Protótipo: ver outro resultado</summary>
          @for (o of outcomeKeys; track o) {
            <label class="option"
              ><input
                type="radio"
                name="o"
                [checked]="outcome() === o"
                (change)="setOutcome(o)"
              />{{ titles[o] }}</label
            >
          }
        </details>
      }
    </section>
  `,
})
export class ResultPage implements OnInit, OnDestroy {
  readonly code = input<string>('', { alias: 'family' });
  protected readonly fam = computed(() => family(this.code()));
  protected readonly progress = signal(0);
  protected readonly running = computed(() => this.progress() < 100);
  protected readonly outcome = signal<Outcome>('verified');
  protected readonly answer = signal<'yes' | 'no' | null>(null);
  protected readonly current = computed(() => OUTCOMES[this.outcome()]);
  protected readonly outcomeKeys = Object.keys(OUTCOMES) as Outcome[];
  protected readonly titles = Object.fromEntries(
    this.outcomeKeys.map((k) => [k, OUTCOMES[k].title]),
  );
  private timer: ReturnType<typeof setInterval> | undefined;

  ngOnInit(): void {
    this.timer = setInterval(() => {
      this.progress.update((p) => Math.min(100, p + 25));
      if (this.progress() >= 100 && this.timer !== undefined) {
        clearInterval(this.timer);
      }
    }, 400);
  }

  ngOnDestroy(): void {
    if (this.timer !== undefined) {
      clearInterval(this.timer);
    }
  }

  protected setOutcome(o: Outcome): void {
    this.outcome.set(o);
    this.answer.set(null);
  }
}
