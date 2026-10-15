// SPDX-License-Identifier: GPL-3.0-or-later
// Stages C (investigation) and D (explanation): opens or continues the ticket, runs the
// family's checks in the core and explains the result.
import { Component, OnInit, computed, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { IpcError, ProbeRunDto } from '../api/types';
import { family, probe } from '../data/catalog';
import { Session } from '../state/session';
import { PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-diagnosis',
  imports: [RouterLink],
  template: `
    @if (fam(); as f) {
      <section class="page" aria-labelledby="t">
        @if (running()) {
          <h1 id="t">Verificando: {{ f.title }}…</h1>
          <ul class="checks card" aria-label="Verificações">
            @for (id of f.probes; track id) {
              <li>
                <span class="mark muted" aria-hidden="true">⟳</span><span>{{ title(id) }}</span>
              </li>
            }
          </ul>
          <div class="row">
            <span class="muted" aria-live="polite">Nada está sendo alterado agora.</span>
            <button type="button" class="btn" (click)="stop()">Parar</button>
          </div>
        } @else if (error(); as e) {
          <h1 id="t">Não consegui fazer as verificações</h1>
          <p class="notice">Algo deu errado do meu lado ({{ e.code }}). Nada foi alterado.</p>
          <div class="actions"><a class="btn" routerLink="/">Voltar ao início</a></div>
        } @else if (session.diagnosis(); as d) {
          <h1 id="t">O que eu encontrei</h1>
          <ul class="checks card" aria-label="Verificações">
            @for (p of d.probes; track p.probeId) {
              <li>
                <span class="mark {{ tone(p) }}" aria-hidden="true">{{ mark(p) }}</span>
                <span
                  >{{ p.title }} <span class="small muted">({{ label(p) }})</span></span
                >
              </li>
            }
          </ul>
          <div class="card" aria-live="polite">
            <p>
              <strong>{{ d.confidencePhrase }}:</strong> {{ d.title }}
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
            @if (d.plan) {
              <a class="btn primary" [routerLink]="['/plano', f.code]">Ver o que eu posso fazer</a>
            }
            <a class="btn" routerLink="/relatorio">Preparar relatório para levar a alguém</a>
            <a class="btn" routerLink="/">Voltar ao início</a>
          </div>
          @if (!api.real) {
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
        }
      </section>
    } @else {
      <h1>Não conheço esse tipo de problema</h1>
      <a class="btn" routerLink="/">Voltar ao início</a>
    }
  `,
})
export class DiagnosisPage implements OnInit {
  readonly code = input<string>('', { alias: 'family' });
  protected readonly api = inject(CORE_API);
  protected readonly session = inject(Session);
  protected readonly state = inject(PrototypeState);
  private readonly router = inject(Router);
  protected readonly fam = computed(() => family(this.code()));
  protected readonly running = signal(true);
  protected readonly error = signal<IpcError | null>(null);
  private stopped = false;

  ngOnInit(): void {
    void this.run();
  }

  private async run(): Promise<void> {
    const f = this.fam();
    if (!f) {
      return;
    }
    this.session.reset();
    this.running.set(true);
    this.error.set(null);
    try {
      const started = await this.api.startTicket(f.code);
      const d = await this.api.diagnose(started.ticket.id);
      if (!this.stopped) {
        this.session.diagnosis.set(d);
      }
    } catch (e) {
      this.error.set(e as IpcError);
    } finally {
      this.running.set(false);
    }
  }

  protected stop(): void {
    // The checks only read; the result is ignored and nothing was changed.
    this.stopped = true;
    void this.router.navigate(['/']);
  }

  protected title(id: string): string {
    return probe(id)?.title ?? id;
  }

  protected mark(p: ProbeRunDto): string {
    return { ok: '✓', found: '✗', failed: '?', skipped: '–' }[p.status];
  }

  protected tone(p: ProbeRunDto): string {
    return { ok: 'ok', found: 'bad', failed: 'warn', skipped: 'muted' }[p.status];
  }

  protected label(p: ProbeRunDto): string {
    return {
      ok: 'nada de errado',
      found: 'encontrei algo',
      failed: 'não consegui verificar',
      skipped:
        p.reason === 'PLATFORM_UNSUPPORTED'
          ? 'não existe neste sistema'
          : 'não verifiquei sem sua permissão',
    }[p.status];
  }

  protected pick(index: string): void {
    this.state.scenario.set(Number(index));
    void this.run();
  }

  protected setPlatform(value: string): void {
    this.state.platform.set(value === 'linux' ? 'linux' : 'windows');
    void this.run();
  }
}
