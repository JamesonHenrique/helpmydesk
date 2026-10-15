// SPDX-License-Identifier: GPL-3.0-or-later
// Stages C (investigation) and D (explanation). The prototype walks through the family's checks
// with a timer and then shows one of the catalog diagnoses as an example.
import { Component, OnDestroy, OnInit, computed, inject, input, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { confidencePhrase, family, probe } from '../data/catalog';
import { PrototypeState } from '../state/prototype';

type CheckState = 'waiting' | 'running' | 'done' | 'skipped';

@Component({
  selector: 'app-diagnosis',
  imports: [RouterLink],
  template: `
    @if (fam(); as f) {
      <section class="page" aria-labelledby="t">
        @if (!finished()) {
          <h1 id="t">Verificando: {{ f.title }}…</h1>
        } @else {
          <h1 id="t">O que eu encontrei</h1>
        }
        <ul class="checks card" aria-label="Verificações">
          @for (c of checks(); track c.id) {
            <li>
              @switch (c.state) {
                @case ('done') {
                  <span class="mark ok" aria-hidden="true">✓</span>
                }
                @case ('running') {
                  <span class="mark muted" aria-hidden="true">⟳</span>
                }
                @case ('skipped') {
                  <span class="mark muted" aria-hidden="true">–</span>
                }
                @default {
                  <span class="mark muted" aria-hidden="true">○</span>
                }
              }
              <span
                >{{ c.title }}
                <span class="small muted">({{ label(c.state) }})</span>
              </span>
            </li>
          }
        </ul>
        @if (!finished()) {
          <div class="row">
            <span class="muted" aria-live="polite">Nada está sendo alterado agora.</span>
            <button type="button" class="btn" (click)="stop()">Parar</button>
          </div>
        } @else if (stopped()) {
          <p class="notice">Parei as verificações. Nada foi alterado.</p>
          <div class="actions"><a class="btn" routerLink="/">Voltar ao início</a></div>
        } @else if (diag(); as d) {
          <div class="card" aria-live="polite">
            <p>
              <strong>{{ phrase(d.confidence) }}:</strong> {{ d.title }}
            </p>
            <p>{{ d.explanation }}</p>
            @if (d.steps.length) {
              <h2>O que você pode fazer</h2>
              <ol class="steps">
                @for (s of d.steps; track $index) {
                  <li>{{ s }}</li>
                }
              </ol>
            }
          </div>
          <div class="actions">
            @if (d.fixes.length) {
              <a class="btn primary" [routerLink]="['/plano', f.code]">Ver o que eu posso fazer</a>
            }
            @if (d.report) {
              <a class="btn" routerLink="/relatorio">Preparar relatório para levar a alguém</a>
            }
            <a class="btn" routerLink="/">Voltar ao início</a>
          </div>
          <details class="small">
            <summary>Protótipo: ver outro resultado de exemplo</summary>
            <label for="sc">Resultado</label>
            <select id="sc" [value]="state.scenario()" (change)="pick($any($event.target).value)">
              @for (x of f.diagnoses; track x.id) {
                <option [value]="$index">{{ x.title }}</option>
              }
            </select>
            <label for="pf">Sistema</label>
            <select
              id="pf"
              [value]="state.platform()"
              (change)="setPlatform($any($event.target).value)"
            >
              <option value="windows">Windows</option>
              <option value="linux">Linux</option>
            </select>
          </details>
        }
      </section>
    } @else {
      <h1>Não conheço esse tipo de problema</h1>
      <a class="btn" routerLink="/">Voltar ao início</a>
    }
  `,
})
export class DiagnosisPage implements OnInit, OnDestroy {
  readonly code = input<string>('', { alias: 'family' });
  protected readonly state = inject(PrototypeState);
  protected readonly fam = computed(() => family(this.code()));
  protected readonly diag = computed(() => this.fam()?.diagnoses[this.state.scenario()]);
  private readonly progress = signal(0);
  protected readonly stopped = signal(false);
  private timer: ReturnType<typeof setInterval> | undefined;

  protected readonly checks = computed(() => {
    const f = this.fam();
    if (!f) {
      return [];
    }
    return f.probes.map((id, i) => {
      const p = probe(id);
      const available = p
        ? (p.platforms as readonly string[]).includes(this.state.platform())
        : false;
      let state: CheckState = 'waiting';
      if (!available) {
        state = 'skipped';
      } else if (i < this.progress()) {
        state = 'done';
      } else if (i === this.progress() && !this.stopped()) {
        state = 'running';
      }
      return { id, title: p?.title ?? id, state };
    });
  });
  protected readonly finished = computed(
    () => this.stopped() || this.progress() >= (this.fam()?.probes.length ?? 0),
  );

  ngOnInit(): void {
    this.timer = setInterval(() => {
      if (this.finished()) {
        this.clear();
      } else {
        this.progress.update((p) => p + 1);
      }
    }, 600);
  }

  ngOnDestroy(): void {
    this.clear();
  }

  protected stop(): void {
    this.stopped.set(true);
    this.clear();
  }

  protected phrase(code: string): string {
    return confidencePhrase(code);
  }

  protected label(state: CheckState): string {
    return {
      waiting: 'aguardando',
      running: 'verificando',
      done: 'feito',
      skipped: 'não existe neste sistema',
    }[state];
  }

  protected pick(index: string): void {
    this.state.scenario.set(Number(index));
  }

  protected setPlatform(value: string): void {
    this.state.platform.set(value === 'linux' ? 'linux' : 'windows');
  }

  private clear(): void {
    if (this.timer !== undefined) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
  }
}
