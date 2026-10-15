// SPDX-License-Identifier: GPL-3.0-or-later
// Settings: privacy ("o que eu olho", consents, export, erase), autonomy, AI, about.
import { Component, OnInit, computed, inject, signal } from '@angular/core';

import { CORE_API } from '../api/core-api';
import { IpcError } from '../api/types';
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
              [checked]="consents().has(c.id)"
              (change)="setConsent(c.id, $any($event.target).checked)"
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
          <button type="button" class="btn" (click)="exportAll()">
            Ver tudo o que está guardado
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
        @if (exported()) {
          <label for="exp" class="small">Seus dados (você pode copiar)</label>
          <textarea id="exp" readonly [value]="exported()"></textarea>
        }
      </div>

      <h2 id="auto">Autonomia</h2>
      <fieldset class="card">
        <legend>Quanto eu posso fazer sozinho</legend>
        @for (o of autonomyOptions; track o.value) {
          <label class="option">
            <input
              type="radio"
              name="autonomy"
              [checked]="autonomy() === o.value"
              (change)="setAutonomy(o.value)"
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
export class SettingsPage implements OnInit {
  private readonly api = inject(CORE_API);
  protected readonly autonomy = signal<Autonomy>('ask_always');
  protected readonly consents = signal<ReadonlySet<string>>(new Set());
  protected readonly exported = signal('');
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

  async ngOnInit(): Promise<void> {
    await this.load();
  }

  private async load(): Promise<void> {
    const s = await this.api.settings();
    this.autonomy.set(s.autonomy);
    this.consents.set(new Set(s.consents));
  }

  protected async setAutonomy(value: Autonomy): Promise<void> {
    await this.safely(() => this.api.setAutonomy(value));
  }

  protected async setConsent(id: string, granted: boolean): Promise<void> {
    await this.safely(() => this.api.setConsent(id, granted));
  }

  protected async exportAll(): Promise<void> {
    await this.safely(async () => this.exported.set(await this.api.exportAll()));
  }

  protected async erase(): Promise<void> {
    this.confirming.set(false);
    await this.safely(async () => {
      await this.api.eraseAll();
      this.exported.set('');
      this.message.set(
        'Pronto: apaguei o histórico, os relatórios e as permissões deste computador.',
      );
    });
  }

  private async safely(action: () => Promise<unknown>): Promise<void> {
    try {
      await action();
    } catch (e) {
      this.message.set(`Não consegui fazer isso (${(e as IpcError).code}).`);
    }
    await this.load();
  }
}
