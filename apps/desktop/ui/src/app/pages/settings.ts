// SPDX-License-Identifier: GPL-3.0-or-later
// Settings: privacy ("o que eu olho", consents, export, erase), autonomy, AI, about.
import { Component, computed, inject, signal } from '@angular/core';

import { CATALOG_VERSION } from '../data/catalog.generated';
import { CONSENTS, probesFor } from '../data/catalog';
import { Autonomy, Platform, PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-settings',
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Configurações</h1>

      <h2 id="priv">Privacidade</h2>
      <div class="card">
        <h3>O que eu olho no seu computador</h3>
        <label for="pf" class="small">Mostrar para</label>
        <select
          id="pf"
          [value]="state.platform()"
          (change)="setPlatform($any($event.target).value)"
        >
          <option value="windows">Windows</option>
          <option value="linux">Linux</option>
        </select>
        <ul class="checks">
          @for (p of disclosures(); track p.id) {
            <li>
              <span class="mark ok" aria-hidden="true">•</span>
              <span>
                <strong>{{ p.title }}</strong>
                @if (p.consent) {
                  <span class="pill warn small">só com permissão</span>
                }
                <br /><span class="small">Olho: {{ p.looksAt }}</span> <br /><span
                  class="small muted"
                  >Não guardo nem envio: {{ p.never }}</span
                >
              </span>
            </li>
          }
        </ul>
      </div>
      <div class="card">
        <h3>Permissões opcionais</h3>
        @for (c of optional; track c.id) {
          <label class="option">
            <input
              type="checkbox"
              [checked]="state.consents().has(c.id)"
              (change)="state.setConsent(c.id, $any($event.target).checked)"
            />
            <span
              ><strong>{{ c.title }}</strong>
              <span class="small muted">(aviso versão {{ c.version }})</span><br />
              <span class="small">{{ c.text }}</span></span
            >
          </label>
        }
      </div>
      <div class="card">
        <h3>Meus dados</h3>
        <p class="small">
          Tudo fica neste computador. Você pode ver tudo o que guardo ou apagar tudo.
        </p>
        <div class="row">
          <button
            type="button"
            class="btn"
            (click)="message.set('No app, isto salva um arquivo com tudo o que está guardado.')"
          >
            Exportar meus dados
          </button>
          @if (!confirming()) {
            <button type="button" class="btn danger" (click)="confirming.set(true)">
              Apagar tudo
            </button>
          } @else {
            <span role="alert"
              >Apagar histórico, relatórios e permissões? Não dá para desfazer.</span
            >
            <button type="button" class="btn" (click)="confirming.set(false)">Cancelar</button>
            <button type="button" class="btn danger" (click)="erase()">Sim, apagar tudo</button>
          }
        </div>
        <p class="small muted" aria-live="polite">{{ message() }}</p>
      </div>

      <h2 id="auto">Autonomia</h2>
      <fieldset class="card">
        <legend>Quanto eu posso fazer sozinho</legend>
        @for (o of autonomyOptions; track o.value) {
          <label class="option">
            <input
              type="radio"
              name="autonomy"
              [checked]="state.autonomy() === o.value"
              (change)="state.autonomy.set(o.value)"
            />
            <span>{{ o.title }}</span>
          </label>
        }
      </fieldset>

      <h2 id="ia">Inteligência artificial</h2>
      <div class="card">
        <p><strong>Desligada.</strong> O modo básico funciona sem IA.</p>
        <p class="small">{{ ai.text }}</p>
        <button type="button" class="btn" disabled>
          Conectar minha conta do OpenCode (em breve)
        </button>
      </div>

      <h2 id="sobre">Sobre</h2>
      <dl class="facts card">
        <dt>Versão</dt>
        <dd>protótipo 0.1.0</dd>
        <dt>Catálogo</dt>
        <dd>{{ catalogVersion }}</dd>
        <dt>Licença</dt>
        <dd>GPL-3.0-or-later</dd>
        <dt>Código</dt>
        <dd>github.com/JamesonHenrique/helpmydesk</dd>
      </dl>
    </section>
  `,
})
export class SettingsPage {
  protected readonly state = inject(PrototypeState);
  protected readonly disclosures = computed(() => probesFor(this.state.platform()));
  protected readonly optional = CONSENTS.filter((c) => c.id !== 'ai_opencode');
  protected readonly ai = CONSENTS.find((c) => c.id === 'ai_opencode') ?? { text: '' };
  protected readonly catalogVersion = CATALOG_VERSION;
  protected readonly confirming = signal(false);
  protected readonly message = signal('');
  protected readonly autonomyOptions: { value: Autonomy; title: string }[] = [
    { value: 'observe_only', title: 'Só olhar' },
    { value: 'ask_always', title: 'Perguntar sempre (recomendado)' },
    { value: 'auto_simple', title: 'Correções simples automáticas' },
  ];

  protected setPlatform(value: string): void {
    this.state.platform.set((value === 'linux' ? 'linux' : 'windows') as Platform);
  }

  protected erase(): void {
    this.confirming.set(false);
    this.state.consents.set(new Set());
    this.message.set(
      'Pronto: no app, isto apaga o histórico, os relatórios e as permissões deste computador.',
    );
  }
}
