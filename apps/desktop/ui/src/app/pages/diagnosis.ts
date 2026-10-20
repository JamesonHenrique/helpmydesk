// SPDX-License-Identifier: GPL-3.0-or-later
// Stages C (investigation) and D (explanation): opens or continues the ticket, runs the
// family's checks in the core and explains the result.
import { Component, OnInit, computed, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { DiagnosisDto, IpcError, ProbeRunDto } from '../api/types';
import { family, probe } from '../data/catalog';
import { Session } from '../state/session';
import { PrototypeState } from '../state/prototype';

/** Spec 021: one suggestion of another family, from an AI call the person already knows
 * about (the card shows exactly what went out). */
interface AiSuggestion {
  family: string;
  title: string;
  preview: string;
  codes: string;
}

@Component({
  selector: 'app-diagnosis',
  imports: [RouterLink],
  template: `
    @if (fam(); as f) {
      <section class="page" aria-labelledby="t">
        @if (running()) {
          <div class="page-head">
            <p class="rule">Inspeção · família “{{ f.title }}”</p>
            <h1 id="t">Verificando: {{ f.title }}…</h1>
            <p class="lead">
              Passo a passo da inspeção, na ordem que o app executa. Indicador indeterminado: o app
              não inventa porcentagem — o tempo termina quando as verificações terminam.
            </p>
          </div>
          <ol class="trail" aria-label="Verificações">
            @for (id of f.probes; track id) {
              <li>
                <span class="mark muted" aria-hidden="true">⟳</span><span>{{ title(id) }}</span>
              </li>
            }
          </ol>
          <div class="row">
            <span class="muted" aria-live="polite">Nada está sendo alterado agora.</span>
            <button type="button" class="btn" (click)="stop()">Parar</button>
          </div>
        } @else if (error(); as e) {
          <div class="page-head">
            <p class="rule">Inspeção interrompida</p>
            <h1 id="t">Não consegui fazer as verificações</h1>
            <p class="lead">Nada foi alterado no seu computador.</p>
          </div>
          <p class="notice error" role="alert">
            Algo deu errado do meu lado ({{ e.code }}). Nada foi alterado.
          </p>
          <div class="actions"><a class="btn" routerLink="/">Voltar ao início</a></div>
        } @else if (session.diagnosis(); as d) {
          <div class="page-head">
            <p class="rule">Relatório da inspeção · {{ f.title }}</p>
            <h1 id="t">O que eu encontrei</h1>
            <p class="lead">
              Cada verificação, o que ela viu e a leitura que faço disso — separados, como num
              prontuário.
            </p>
          </div>
          <ol class="trail sheet" aria-label="Verificações">
            @for (p of d.probes; track p.probeId) {
              <li>
                <span class="mark {{ tone(p) }}" aria-hidden="true">{{ mark(p) }}</span>
                <span
                  >{{ p.title }} <span class="small muted">({{ label(p) }})</span></span
                >
              </li>
            }
          </ol>
          @if (suggestion(); as s) {
            <div class="card" aria-live="polite">
              <p class="rule">Sugestão alternativa · IA</p>
              <p>
                As verificações não confirmaram isso. Elas sugerem outro tipo de problema:
                <strong>{{ s.title }}</strong
                >.
              </p>
              <div class="actions">
                <button type="button" class="btn primary" (click)="go(s.family)">
                  Verificar isso
                </button>
              </div>
              <p class="small muted">
                Para isso, a ajuda inteligente recebeu o seu relato redigido («{{ s.preview }}») e
                os códigos das verificações ({{ s.codes }}) — nada mais.
              </p>
            </div>
          }
          <div class="card" aria-live="polite">
            <p class="rule">Leitura do técnico · confiança declarada</p>
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
  protected readonly suggestion = signal<AiSuggestion | null>(null);
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
        void this.suggest(f.code, d);
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

  /** Spec 021: the checks contradicted the AI's first pick — one extra classification with
   * the finding codes (never values or log lines). Silent on any error. */
  private async suggest(current: string, d: DiagnosisDto): Promise<void> {
    if (!d.isFallback) {
      return;
    }
    const codes = [...new Set(d.probes.flatMap((p: ProbeRunDto) => p.findings))];
    const story = this.session.story();
    if (codes.length === 0 || !story || this.session.extraClassify()) {
      return;
    }
    try {
      if (!(await this.api.aiStatus()).connected) {
        return;
      }
      const c = await this.api.aiClassify(story, codes);
      if (!c.family || c.family === current || c.confidence === 'low') {
        this.session.extraClassify.set(true);
        return;
      }
      this.session.extraClassify.set(true);
      this.suggestion.set({
        family: c.family,
        title: family(c.family)?.title ?? c.family,
        preview: await this.api.aiPreview(story),
        codes: codes.join(', '),
      });
    } catch {
      // Best effort only: the deterministic diagnosis of this family stands.
    }
  }

  /** Navigating to the suggestion is the person's decision; the AI never changes the page. */
  protected go(code: string): void {
    this.suggestion.set(null);
    void this.router.navigate(['/diagnostico', code]);
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
