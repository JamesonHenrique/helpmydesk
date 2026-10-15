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
      <h1 id="t">Meu computador</h1>
      <p class="lead">Só leitura. É o que entra na parte "Sistema" do relatório.</p>
      @if (info(); as i) {
        <dl class="facts card">
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
      } @else {
        <p aria-live="polite">Lendo os dados do sistema…</p>
      }
      <p class="small">
        Não mostro nem guardo o nome do computador nem o seu nome de usuário.
        <a routerLink="/configuracoes">Ver o que cada verificação olha</a>.
      </p>
    </section>
  `,
})
export class ComputerPage implements OnInit {
  private readonly api = inject(CORE_API);
  protected readonly info = signal<SystemDto | null>(null);

  async ngOnInit(): Promise<void> {
    this.info.set(await this.api.system());
  }
}
