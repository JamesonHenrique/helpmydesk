// SPDX-License-Identifier: GPL-3.0-or-later
// Stage B: opening a ticket. Basic mode (no AI): menu only.
import { Component, inject } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { FAMILIES, FAMILY_HINTS } from '../data/catalog';
import { PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-home',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">O que está acontecendo?</h1>
      @if (!state.aiConnected()) {
        <p class="notice small">
          Modo básico: escolha uma opção abaixo. Para escrever com suas palavras, conecte a IA em
          <a routerLink="/configuracoes">Configurações</a> (opcional).
        </p>
      }
      <label class="small muted" for="free">Escreva com suas palavras</label>
      <input
        id="free"
        type="text"
        [disabled]="!state.aiConnected()"
        placeholder="Disponível com a IA conectada"
      />
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
export class HomePage {
  protected readonly state = inject(PrototypeState);
  private readonly router = inject(Router);
  protected readonly families = FAMILIES;
  protected readonly hints = FAMILY_HINTS;

  protected start(code: string): void {
    this.state.scenario.set(0);
    void this.router.navigate(['/diagnostico', code]);
  }
}
