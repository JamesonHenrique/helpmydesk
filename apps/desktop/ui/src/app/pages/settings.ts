// SPDX-License-Identifier: GPL-3.0-or-later
// Settings: privacy ("o que eu olho", consents, export, erase), autonomy, AI, about.
import { Component, OnInit, computed, inject, signal } from '@angular/core';

import { CORE_API } from '../api/core-api';
import { AiConnectComponent } from '../components/ai-connect';
import { IpcError } from '../api/types';
import { CATALOG_VERSION } from '../data/catalog.generated';
import { CONSENTS, probesFor } from '../data/catalog';
import { Autonomy, Platform, PrototypeState } from '../state/prototype';

@Component({
  selector: 'app-settings',
  imports: [AiConnectComponent],
  template: `
    <section class="page" aria-labelledby="t">
      <div class="page-head">
        <p class="rule">Preferências · 01 a 05</p>
        <h1 id="t">Configurações</h1>
        <p class="lead">
          Privacidade, autonomia e a ajuda inteligente — tudo à vista, tudo revogável.
        </p>
      </div>

      <div class="settings-grid">
        <nav class="toc" aria-label="Seções das configurações">
          <a class="toc-link" href="#priv">01 · Privacidade</a>
          <a class="toc-link" href="#auto">02 · Autonomia</a>
          <a class="toc-link" href="#ia">03 · Inteligência artificial</a>
          <a class="toc-link" href="#sobre">04 · Sobre</a>
        </nav>
        <div class="settings-body">
          <h2 id="priv">Privacidade</h2>
          <div class="sheet">
            <h3 class="no-top">O que eu olho no seu computador</h3>
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
          <div class="sheet">
            <h3 class="no-top">Permissões opcionais</h3>
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
          <div class="sheet">
            <h3 class="no-top">Meus dados</h3>
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
                <span class="erase-confirm" role="alert"
                  >Apagar histórico, relatórios, permissões e o código da ajuda inteligente? Não dá
                  para desfazer.</span
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
            <app-ai-connect />
          </div>

          <h2 id="sobre">Sobre</h2>
          <dl class="facts card">
            <dt>Versão</dt>
            <dd>0.1.0 (pré-alfa)</dd>
            <dt>Catálogo</dt>
            <dd>{{ catalogVersion }}</dd>
            <dt>Licença</dt>
            <dd>GPL-3.0-or-later</dd>
            <dt>Código</dt>
            <dd>github.com/JamesonHenrique/helpmydesk</dd>
            <dt>Ao abrir</dt>
            <dd>
              Consulto o GitHub para saber se alguma correção foi desligada por segurança. Nenhum
              dado seu vai junto (o GitHub vê o endereço de internet, como em qualquer download).
            </dd>
          </dl>
        </div>
      </div>
    </section>
  `,
  styles: `
    .no-top {
      margin-top: 0;
    }
    .settings-grid {
      display: grid;
      gap: 1.5rem;
      grid-template-columns: minmax(10rem, 12rem) minmax(0, 1fr);
      align-items: start;
    }
    .toc {
      position: sticky;
      top: 1rem;
      display: grid;
      gap: 0.4rem;
      border-left: 2px solid var(--line);
      padding-left: 0.8rem;
    }
    .toc-link {
      color: var(--muted);
      text-decoration: none;
      font-size: 0.9rem;
      font-family: var(--font-mono);
    }
    .toc-link:hover {
      color: var(--accent);
      text-decoration: underline;
    }
    .erase-confirm {
      color: var(--bad);
      font-weight: 600;
      max-width: 30rem;
    }
    .settings-body .sheet,
    .settings-body .card {
      margin-bottom: 1rem;
    }
    @media (max-width: 899px) {
      .settings-grid {
        grid-template-columns: 1fr;
      }
      .toc {
        position: static;
        display: flex;
        flex-wrap: wrap;
        gap: 0.3rem 1rem;
        border-left: none;
        border-bottom: 1px dashed var(--line);
        padding: 0 0 0.6rem;
      }
    }
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
