// SPDX-License-Identifier: GPL-3.0-or-later
// "Meu computador": read-only facts about the system (never user or computer names).
import { Component, OnInit, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { SystemDto } from '../api/types';

@Component({
  selector: 'app-computer',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <div class="page-head">
        <p class="rule">Ficha técnica · só leitura</p>
        <h1 id="t">Meu computador</h1>
        <p class="lead">Só leitura. É o que entra na parte "Sistema" do relatório.</p>
      </div>
      @if (info(); as i) {
        <div class="sheet">
          <dl class="facts">
            <dt>Sistema</dt>
            <dd>{{ i.osName ?? 'desconhecido' }} {{ i.osVersion ?? '' }}</dd>
            @if (i.kernelVersion) {
              <dt>Versão do núcleo</dt>
              <dd>{{ i.kernelVersion }}</dd>
            }
            <dt>Arquitetura</dt>
            <dd>{{ i.arch ?? 'desconhecida' }}</dd>
            <dt>Catálogo de verificações</dt>
            <dd>{{ i.catalogVersion }}</dd>
          </dl>
        </div>
      } @else {
        <p aria-live="polite">Lendo os dados do sistema…</p>
      }
      <div class="privacy-note">
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
          Não mostro nem guardo o nome do computador nem o seu nome de usuário.
          <a routerLink="/configuracoes">Ver o que cada verificação olha</a>.
        </p>
      </div>
    </section>
  `,
  styles: `
    .privacy-note {
      display: flex;
      align-items: flex-start;
      gap: 0.5rem;
      border-top: 1px dashed var(--line-strong);
      padding-top: 0.7rem;
      max-width: 52rem;
    }
    .privacy-note svg {
      width: 1.1rem;
      height: 1.1rem;
      flex: none;
      color: var(--ok);
      margin-top: 0.15rem;
    }
    .privacy-note p {
      margin: 0;
    }
  `,
})
export class ComputerPage implements OnInit {
  private readonly api = inject(CORE_API);
  protected readonly info = signal<SystemDto | null>(null);

  async ngOnInit(): Promise<void> {
    this.info.set(await this.api.system());
  }
}
