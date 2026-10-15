// SPDX-License-Identifier: GPL-3.0-or-later
// Stage E: the plan and its approval. In the real app this is a separate Tauri window with its
// own IPC capability (INV-04); the texts come only from the catalog and the Policy Engine.
import { Component, computed, inject, input, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { Fix, family, fix } from '../data/catalog';
import { PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-plan',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">O que eu proponho fazer</h1>
      @if (fixes().length === 0) {
        <p>
          Para este resultado eu não tenho correção automática. Siga as orientações ou prepare um
          relatório.
        </p>
        <div class="actions"><a class="btn" routerLink="/relatorio">Preparar relatório</a></div>
      } @else {
        <ol class="steps">
          @for (f of fixes(); track f.id) {
            <li class="card">
              @if (choosing()) {
                <label class="option">
                  <input type="checkbox" [checked]="chosen().has(f.id)" (change)="toggle(f.id)" />
                  <strong>{{ f.title }}</strong>
                </label>
              } @else {
                <strong>{{ f.title }}</strong>
              }
              <dl class="facts small">
                <dt>Risco</dt>
                <dd>{{ f.risk }}</dd>
                <dt>Desfazer</dt>
                <dd>{{ f.undo }}</dd>
              </dl>
              @if (f.requiresElevation) {
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
            [disabled]="choosing() && chosen().size === 0"
            (click)="approve()"
          >
            Sim, pode fazer
          </button>
        </div>
        <p class="small muted">
          No app de verdade, esta tela abre numa janela própria e a aprovação vale só para este
          plano exato, por 10 minutos.
        </p>
      }
    </section>
  `,
})
export class PlanPage {
  readonly code = input<string>('', { alias: 'family' });
  private readonly state = inject(PrototypeState);
  private readonly router = inject(Router);
  protected readonly choosing = signal(false);
  protected readonly chosen = signal<ReadonlySet<string>>(new Set());

  protected readonly fixes = computed<Fix[]>(() => {
    const f = family(this.code());
    const d = f?.diagnoses[this.state.scenario()];
    const ids: readonly string[] = d?.fixes.length
      ? d.fixes
      : (f?.diagnoses.flatMap((x) => [...x.fixes]) ?? []);
    return [...new Set(ids)].map((id) => fix(id)).filter((x): x is Fix => x !== undefined);
  });

  protected toggle(id: string): void {
    const next = new Set(this.chosen());
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    this.chosen.set(next);
  }

  protected approve(): void {
    void this.router.navigate(['/resultado', this.code()]);
  }
}
