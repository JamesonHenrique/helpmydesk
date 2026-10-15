// SPDX-License-Identifier: GPL-3.0-or-later
// Stage B: opening a ticket. Basic mode: menu. With the AI on (spec 018): free text too.
import { Component, OnInit, inject, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { IpcError } from '../api/types';
import { FAMILIES, FAMILY_HINTS } from '../data/catalog';
import { PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-home',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">O que está acontecendo?</h1>
      @if (!connected()) {
        <p class="notice small">
          Modo básico: escolha uma opção abaixo. Para escrever com suas palavras, ligue a ajuda
          inteligente em <a routerLink="/configuracoes">Configurações</a> (opcional).
        </p>
      }
      <form class="free" (submit)="$event.preventDefault(); ask()">
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
          <button type="submit" class="btn primary" [disabled]="busy() || !text().trim()">
            Continuar
          </button>
        }
      </form>
      @if (connected() && preview()) {
        <p class="small muted">
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
      <h2>Ou escolha</h2>
      <div class="tiles">
        @for (f of families; track f.code) {
          <button type="button" class="tile" (click)="start(f.code)">
            <strong>{{ f.title }}</strong>
            <span>{{ hints[f.code] }}</span>
          </button>
        }
        <a class="tile" routerLink="/relatorio">
          <strong>Não sei explicar</strong>
          <span>Preparo um relatório para você levar a alguém</span>
        </a>
      </div>
      <p class="small muted">
        Primeira vez aqui? Veja <a routerLink="/boas-vindas">o que eu faço e o que eu não faço</a>.
      </p>
    </section>
  `,
})
export class HomePage implements OnInit {
  protected readonly state = inject(PrototypeState);
  private readonly api = inject(CORE_API);
  private readonly router = inject(Router);
  protected readonly families = FAMILIES;
  protected readonly hints = FAMILY_HINTS;
  protected readonly connected = signal(false);
  protected readonly busy = signal(false);
  protected readonly text = signal('');
  /** F3-TRANSP: exactly what would be sent (computed locally by the core). */
  protected readonly preview = signal('');
  protected readonly question = signal('');
  protected readonly error = signal('');

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
      this.preview.set(value.trim() ? await this.api.aiPreview(value) : '');
    } catch {
      this.preview.set('');
    }
  }

  protected start(code: string): void {
    this.state.scenario.set(0);
    void this.router.navigate(['/diagnostico', code]);
  }

  /** The AI only picks the family; everything after is the same deterministic flow. */
  protected async ask(): Promise<void> {
    this.error.set('');
    this.question.set('');
    this.busy.set(true);
    try {
      const c = await this.api.aiClassify(this.text());
      if (c.family && c.confidence !== 'low') {
        this.start(c.family);
        return;
      }
      this.question.set(
        c.question ?? 'Não tenho certeza do que é. Escolha uma opção abaixo ou conte mais.',
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
