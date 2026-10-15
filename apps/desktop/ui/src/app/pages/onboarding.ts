// SPDX-License-Identifier: GPL-3.0-or-later
// Stage A: first run. Welcome → what I do / don't → permissions → autonomy → AI (optional) → done.
import { Component, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { CONSENTS } from '../data/catalog';
import { Autonomy, PrototypeState } from '../state/prototype';

const STEPS = [
  'Boas-vindas',
  'O que eu faço',
  'Permissões',
  'Autonomia',
  'Inteligência artificial',
  'Pronto',
];

@Component({
  selector: 'app-onboarding',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <p class="small muted" aria-live="polite">
        Passo {{ step() + 1 }} de {{ steps.length }}: {{ steps[step()] }}
      </p>
      @switch (step()) {
        @case (0) {
          <h1 id="t" tabindex="-1">Olá! Eu ajudo com problemas do computador.</h1>
          <p>
            Você me conta o que está acontecendo, eu verifico e explico. Só mudo algo com a sua
            permissão.
          </p>
        }
        @case (1) {
          <h1 id="t" tabindex="-1">O que eu faço e o que eu não faço</h1>
          <div class="card">
            <h2>Eu faço</h2>
            <ul class="steps">
              <li>Verificações de leitura: internet, desempenho, espaço, som e câmera.</li>
              <li>Explico o que encontrei, com palavras simples.</li>
              <li>Proponho correções conhecidas e só faço com a sua aprovação.</li>
              <li>Confiro se resolveu. Se não resolver, preparo um relatório.</li>
            </ul>
            <h2>Eu nunca</h2>
            <ul class="steps">
              <li>Olho seus arquivos, fotos, senhas, e-mails ou histórico do navegador.</li>
              <li>Instalo ou removo programas, nem mexo no antivírus.</li>
              <li>Envio dados para o projeto. Tudo fica neste computador.</li>
            </ul>
          </div>
        }
        @case (2) {
          <h1 id="t" tabindex="-1">Permissões opcionais</h1>
          <p class="lead">
            As verificações básicas não precisam de permissão. Estas são opcionais:
          </p>
          @for (c of optional; track c.id) {
            <label class="option card">
              <input
                type="checkbox"
                [checked]="state.consents().has(c.id)"
                (change)="state.setConsent(c.id, $any($event.target).checked)"
              />
              <span
                ><strong>{{ c.title }}</strong
                ><br /><span class="small">{{ c.text }}</span></span
              >
            </label>
          }
          <p class="small">
            <a routerLink="/configuracoes">Ver exatamente o que cada verificação olha</a>
          </p>
        }
        @case (3) {
          <h1 id="t" tabindex="-1">Quanto eu posso fazer sozinho?</h1>
          <fieldset>
            <legend>Escolha um nível (pode mudar depois)</legend>
            @for (o of autonomyOptions; track o.value) {
              <label class="option">
                <input
                  type="radio"
                  name="autonomy"
                  [value]="o.value"
                  [checked]="state.autonomy() === o.value"
                  (change)="setAutonomy(o.value)"
                />
                <span
                  ><strong>{{ o.title }}</strong
                  ><br /><span class="small muted">{{ o.text }}</span></span
                >
              </label>
            }
          </fieldset>
        }
        @case (4) {
          <h1 id="t" tabindex="-1">Inteligência artificial (opcional)</h1>
          <p>{{ ai.text }}</p>
          <p class="notice small">
            No protótipo a IA ainda não pode ser conectada. O modo básico funciona sem ela.
          </p>
        }
        @default {
          <h1 id="t" tabindex="-1">Tudo pronto</h1>
          <p>Você pode mudar tudo isso depois em Configurações.</p>
          <a class="btn primary" routerLink="/">Começar</a>
        }
      }
      <div class="actions">
        @if (step() > 0) {
          <button type="button" class="btn" (click)="go(-1)">Voltar</button>
        }
        @if (!last()) {
          <button type="button" class="btn primary" (click)="go(1)">Continuar</button>
        }
      </div>
    </section>
  `,
})
export class OnboardingPage {
  protected readonly state = inject(PrototypeState);
  protected readonly steps = STEPS;
  protected readonly step = signal(0);
  protected readonly last = computed(() => this.step() === STEPS.length - 1);
  protected readonly optional = CONSENTS.filter((c) => c.id !== 'ai_opencode');
  protected readonly ai = CONSENTS.find((c) => c.id === 'ai_opencode') ?? { text: '' };
  protected readonly autonomyOptions: { value: Autonomy; title: string; text: string }[] = [
    { value: 'observe_only', title: 'Só olhar', text: 'Eu verifico e explico, mas não mudo nada.' },
    {
      value: 'ask_always',
      title: 'Perguntar sempre (recomendado)',
      text: 'Peço sua aprovação antes de cada correção.',
    },
    {
      value: 'auto_simple',
      title: 'Correções simples automáticas',
      text: 'Faço sozinho só correções muito simples e que podem ser desfeitas.',
    },
  ];

  protected go(delta: number): void {
    this.step.update((s) => Math.min(STEPS.length - 1, Math.max(0, s + delta)));
    queueMicrotask(() => document.getElementById('t')?.focus());
  }

  protected setAutonomy(value: Autonomy): void {
    this.state.autonomy.set(value);
  }
}
