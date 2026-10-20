// SPDX-License-Identifier: GPL-3.0-or-later
// Stage B: opening a ticket. Basic mode: menu. With the AI on (spec 018): free text too.
import { Component, OnInit, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { IpcError } from '../api/types';
import { FAMILIES, FAMILY_HINTS } from '../data/catalog';
import { Session } from '../state/session';
import { PrototypeState } from '../state/prototype';

/** Questions the AI may ask before the app falls back to the menu. */
const MAX_ROUNDS = 2;

@Component({
  selector: 'app-home',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <div class="hero">
        <div class="hero-text">
          <p class="rule">Bancada · Atendimento novo</p>
          <h1 id="t" class="display">O que está acontecendo?</h1>
          <p class="hero-copy">
            Conte o que sente — eu <strong>verifico primeiro</strong>, explico o que encontrei e só
            toco no seu computador depois do seu Sim.
          </p>
          <p class="tagline hero-tag">Seu computador, sem mistério.</p>
        </div>
        <img
          class="assistant"
          src="/branding/assistente-oficina.png"
          alt=""
          width="190"
          height="190"
        />
      </div>

      <form class="sheet free" (submit)="$event.preventDefault(); ask()">
        <div class="menu-head">
          <h2 class="free-title">Caminho principal</h2>
          @if (!connected()) {
            <span class="pill basic-pill">Modo básico</span>
          }
        </div>
        <p class="small muted free-note">
          @if (!connected()) {
            Para escrever com suas palavras, ligue a ajuda inteligente em
            <a routerLink="/configuracoes">Configurações</a> (opcional). Enquanto isso, o modo
            básico abaixo resolve os casos comuns.
          } @else {
            Escreva com suas palavras — ou escolha a família do problema logo abaixo.
          }
        </p>
        <label class="small muted" for="free">Escreva com suas palavras</label>
        <input
          id="free"
          type="text"
          maxlength="1000"
          [disabled]="!connected() || busy()"
          [value]="text()"
          (input)="typed($any($event.target).value)"
          [placeholder]="
            connected()
              ? 'Ex.: a internet caiu desde ontem'
              : 'Disponível com a ajuda inteligente ligada'
          "
        />
        @if (connected()) {
          <div class="row free-row">
            <button type="submit" class="btn primary" [disabled]="busy() || !text().trim()">
              Continuar
            </button>
            <span class="small muted">Sem seu nome nem o nome do seu computador.</span>
          </div>
        }
      </form>
      @if (connected() && preview()) {
        <p class="small muted preview">
          Vai para a IA, sem seu nome nem o do computador: «{{ preview() }}»
        </p>
      }
      @if (busy()) {
        <p aria-live="polite">Entendendo o que você escreveu…</p>
      }
      @if (question()) {
        <p class="notice" aria-live="polite">{{ question() }}</p>
      }
      @if (error()) {
        <p class="notice" role="alert">{{ error() }}</p>
      }

      <div class="menu-head">
        <h2>
          {{ connected() ? 'Ou escolha a família do problema' : 'Escolha a família do problema' }}
        </h2>
        <p class="rule">Menu de escolha · 01 a 06</p>
      </div>
      <div class="tiles">
        @for (f of families; track f.code; let i = $index) {
          <button type="button" class="tile" (click)="start(f.code)">
            <span class="num" aria-hidden="true">{{ zero(i + 1) }}</span>
            @switch (f.code) {
              @case ('connectivity') {
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path
                    d="M2.2 6.4a8.6 8.6 0 0 1 11.6 0M4.4 8.8a5.6 5.6 0 0 1 7.2 0M6.7 11a2.6 2.6 0 0 1 2.6 0"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                  />
                  <circle cx="8" cy="13.2" r="1.15" fill="currentColor" />
                </svg>
              }
              @case ('performance') {
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <path
                    d="M2.4 12.6a5.9 5.9 0 0 1 11.2 0"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                  />
                  <path
                    d="M8 11.6 11 7.4"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                  />
                  <circle cx="8" cy="12.2" r="1.2" fill="currentColor" />
                </svg>
              }
              @case ('storage') {
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <rect
                    x="2"
                    y="3.4"
                    width="12"
                    height="9.2"
                    rx="1.4"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                  />
                  <circle cx="11.2" cy="10" r="1" fill="currentColor" />
                  <path
                    d="M4.6 6h5"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                  />
                </svg>
              }
              @case ('audio_video') {
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <rect
                    x="2.2"
                    y="6"
                    width="7.6"
                    height="6.4"
                    rx="1.2"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                  />
                  <path d="m9.8 8.4 3.4-2.2v5.6L9.8 9.6Z" fill="currentColor" />
                </svg>
              }
              @default {
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <rect
                    x="2"
                    y="2.8"
                    width="12"
                    height="10.4"
                    rx="1.3"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                  />
                  <path
                    d="m6 6.6 4 3.4M10 6.6l-4 3.4"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.6"
                    stroke-linecap="round"
                  />
                </svg>
              }
            }
            <strong>{{ f.title }}</strong>
            <span class="hint">{{ hints[f.code] }}</span>
          </button>
        }
        <a class="tile alt" routerLink="/relatorio">
          <span class="num" aria-hidden="true">{{ zero(6) }}</span>
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <rect
              x="2.6"
              y="2.6"
              width="10.8"
              height="10.8"
              rx="1.2"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
            />
            <path
              d="M5 5.8h6M5 8h6M5 10.2h4"
              fill="none"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
          <strong>Não sei explicar</strong>
          <span class="hint">Preparo um relatório para você levar a alguém</span>
        </a>
      </div>

      <div class="safety-foot">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 1.8 13.6 4v4.2c0 3.1-2.3 5.2-5.6 6-3.3-.8-5.6-2.9-5.6-6V4Z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linejoin="round"
          />
          <path
            d="m5.6 7.8 1.6 1.7 3.2-3.4"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <p class="small">
          Primeiro eu verifico. <strong>Nada muda sem a sua autorização</strong> — e tudo fica
          registrado em <a routerLink="/historico">Histórico</a>.
        </p>
      </div>
      <p class="small muted">
        Primeira vez aqui? Veja <a routerLink="/boas-vindas">o que eu faço e o que eu não faço</a>.
      </p>
    </section>
  `,
  styles: `
    .hero {
      display: grid;
      grid-template-columns: minmax(0, 1.7fr) auto;
      gap: 0.75rem 1.5rem;
      align-items: end;
      /* Carvão nos dois temas (tokens da sidebar): mesmo bloco de marca,
         rubrica coral sempre legível (AA nos dois temas). */
      background: var(--rail-bg);
      color: var(--rail-fg);
      border: 1px solid var(--rail-line);
      border-radius: var(--radius);
      padding: 1.4rem 1.5rem 0;
      overflow: hidden;
      position: relative;
    }
    .hero .rule {
      color: var(--rail-accent);
    }
    .display {
      font-size: var(--fs-display, clamp(2rem, 3.2vw, 3.25rem));
      letter-spacing: -0.02em;
      margin: 0.15rem 0 0.4rem;
    }
    .hero-copy {
      margin: 0;
      color: var(--rail-muted);
      max-width: 46ch;
    }
    .hero-tag {
      color: var(--rail-muted);
      margin: 0.6rem 0 1rem;
    }
    .assistant {
      justify-self: end;
      align-self: end;
      max-width: 190px;
      height: auto;
      display: block;
    }
    @media (max-width: 560px) {
      .hero {
        grid-template-columns: 1fr;
        padding-bottom: 1rem;
      }
      .assistant {
        display: none;
      }
    }
    .free-title {
      margin: 0;
      font-size: 1rem;
    }
    .basic-pill {
      color: var(--technical-ink);
      background: var(--technical-soft);
      border-color: var(--technical-ink);
    }
    .free {
      display: grid;
      gap: 0.5rem;
    }
    .free-row {
      margin-top: 0.2rem;
    }
    .preview {
      margin: 0;
    }
    .tile,
    .tile:visited {
      font: inherit;
      text-align: left;
      text-decoration: none;
      color: var(--fg);
      padding: 1rem;
      border-radius: var(--radius);
      border: 1px solid var(--control-border);
      background: var(--surface);
      cursor: pointer;
      display: grid;
      grid-template-columns: auto 1fr;
      align-items: start;
      gap: 0.2rem 0.6rem;
      box-shadow: 3px 3px 0 var(--line);
      transition:
        border-color var(--motion-quick) ease,
        background var(--motion-quick) ease,
        transform var(--motion-quick) ease;
      width: 100%;
      box-sizing: border-box;
    }
    .tile:hover {
      border-color: var(--accent);
      background: var(--accent-soft);
    }
    .tile:active {
      transform: translate(1px, 1px);
      box-shadow: none;
    }
    .tile .num {
      grid-column: 1 / -1;
      font-family: var(--font-mono);
      font-size: 0.8rem;
      color: var(--technical-ink);
      letter-spacing: 0.1em;
    }
    .tile svg {
      grid-column: 1;
      grid-row: 2;
      width: 1.6rem;
      height: 1.6rem;
      flex: none;
      color: var(--accent);
      margin-top: 0.15rem;
    }
    .tile strong {
      grid-column: 2;
      grid-row: 2;
      font-size: 1.1rem;
    }
    .tile .hint {
      grid-column: 1 / -1;
      color: var(--muted);
      font-size: 0.88rem;
    }
    .menu-head {
      display: flex;
      flex-wrap: wrap;
      align-items: baseline;
      justify-content: space-between;
      gap: 0.25rem 1rem;
      margin-top: 0.5rem;
    }
    .menu-head h2 {
      margin: 0;
    }
    .safety-foot {
      display: flex;
      align-items: flex-start;
      gap: 0.55rem;
      border-top: 1px dashed var(--line-strong);
      padding-top: 0.8rem;
      margin-top: 0.4rem;
      max-width: 52rem;
    }
    .safety-foot svg {
      width: 1.2rem;
      height: 1.2rem;
      flex: none;
      color: var(--ok);
      margin-top: 0.1rem;
    }
    .safety-foot p {
      margin: 0;
    }
  `,
})
export class HomePage implements OnInit {
  protected readonly state = inject(PrototypeState);
  protected readonly session = inject(Session);
  private readonly api = inject(CORE_API);
  private readonly router = inject(Router);
  protected readonly families = FAMILIES;
  protected readonly hints = FAMILY_HINTS;
  protected readonly connected = signal(false);
  protected readonly busy = signal(false);
  protected readonly text = signal('');
  /** F3-TRANSP: exactly what would be sent (computed locally by the core). */
  protected readonly preview = signal('');
  /** What the person said before answering the AI's question, and how many tries so far. */
  private story = '';
  private rounds = 0;
  protected readonly question = signal('');
  protected readonly error = signal('');

  protected zero(n: number): string {
    return String(n).padStart(2, '0');
  }

  async ngOnInit(): Promise<void> {
    try {
      this.connected.set((await this.api.aiStatus()).connected);
    } catch {
      this.connected.set(false);
    }
  }

  protected async typed(value: string): Promise<void> {
    this.text.set(value);
    try {
      const said = [this.story, value.trim()].filter((t) => t).join('. ');
      this.preview.set(value.trim() ? await this.api.aiPreview(said) : '');
    } catch {
      this.preview.set('');
    }
  }

  protected start(code: string, said = ''): void {
    this.session.beginAttendance(said);
    this.state.scenario.set(0);
    void this.router.navigate(['/diagnostico', code]);
  }

  /** The AI only picks the family; everything after is the same deterministic flow. */
  protected async ask(): Promise<void> {
    this.error.set('');
    this.question.set('');
    this.busy.set(true);
    // SPEC-010: an answer to the AI's question is added to what was said before.
    const said = [this.story, this.text().trim()].filter((t) => t).join('. ');
    try {
      const c = await this.api.aiClassify(said);
      if (c.family && c.confidence !== 'low') {
        // Spec 021: the story goes with the attendance; the diagnosis page may use it for
        // one re-classification if the checks contradict the AI's pick.
        this.start(c.family, said);
        return;
      }
      this.rounds += 1;
      if (this.rounds >= MAX_ROUNDS) {
        // No insisting: after two tries the menu decides.
        this.story = '';
        this.rounds = 0;
        this.question.set('Ainda não tenho certeza. Escolha uma opção abaixo, por favor.');
        return;
      }
      this.story = said;
      this.text.set('');
      this.preview.set('');
      this.question.set(
        c.question ?? 'Não tenho certeza do que é. Conte um pouco mais ou escolha abaixo.',
      );
    } catch (e) {
      const code = (e as IpcError).code;
      this.error.set(
        code === 'AI_NETWORK'
          ? 'Não consegui falar com a ajuda inteligente agora. Escolha uma opção abaixo.'
          : `A ajuda inteligente não respondeu direito (${code}). Escolha uma opção abaixo.`,
      );
    } finally {
      this.busy.set(false);
    }
  }
}
