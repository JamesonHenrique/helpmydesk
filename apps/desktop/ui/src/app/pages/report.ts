// SPDX-License-Identifier: GPL-3.0-or-later
// Stage H: the "não consegui" report, made by the core (report@1) and editable before it leaves
// the computer. Without a diagnosis, the browser prototype shows a sample.
import { Component, OnInit, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { CORE_API } from '../api/core-api';
import { IpcError } from '../api/types';
import { SAMPLE_REPORT } from '../data/catalog.generated';
import { Session } from '../state/session';

@Component({
  selector: 'app-report',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Isso eu não consigo resolver com segurança</h1>
      <p>Preparo um relatório para você mostrar a quem for te ajudar.</p>
      <div class="card small">
        <div class="row">
          <div>
            <strong>Ele tem</strong>
            <ul class="steps">
              <li>Sua descrição do problema</li>
              <li>O que eu verifiquei e encontrei</li>
              <li>O que eu tentei e o resultado</li>
              <li>Versão do sistema</li>
            </ul>
          </div>
          <div>
            <strong>Não tem</strong>
            <ul class="steps">
              <li>Arquivos, senhas, nome de usuário</li>
              <li>Nome do computador</li>
              <li>Nomes de programas abertos</li>
            </ul>
          </div>
        </div>
      </div>
      @if (needsDiagnosis()) {
        <p class="notice">
          Primeiro escolha o tipo de problema no início: o relatório usa as verificações que eu
          fizer.
        </p>
        <div class="actions"><a class="btn primary" routerLink="/">Ir para o início</a></div>
      } @else {
        @if (!reportId() && session.diagnosis()) {
          <label for="sym"
            ><strong>Conte com suas palavras o que aconteceu</strong>
            <span class="small muted">(opcional)</span></label
          >
          <input
            id="sym"
            type="text"
            [value]="symptom()"
            (input)="symptom.set($any($event.target).value)"
          />
          <div class="actions">
            <button type="button" class="btn primary" (click)="generate()">Gerar relatório</button>
          </div>
        }
        @if (text()) {
          <label for="rel"
            ><strong>Ver e editar o relatório</strong>
            <span class="small muted">(apague o que não quiser compartilhar)</span></label
          >
          <textarea id="rel" [value]="text()" (input)="edit($any($event.target).value)"></textarea>
          <div class="actions">
            <button type="button" class="btn" (click)="copy()">Copiar texto</button>
            <button type="button" class="btn primary" (click)="save()">Salvar arquivo</button>
          </div>
        }
      }
      <p class="small muted" aria-live="polite">{{ status() }}</p>
      <p class="small">Nada é enviado pelo HelpMyDesk. Você decide se e para quem entregar.</p>
    </section>
  `,
})
export class ReportPage implements OnInit {
  private readonly api = inject(CORE_API);
  protected readonly session = inject(Session);
  protected readonly text = signal('');
  protected readonly symptom = signal('');
  protected readonly status = signal('');
  protected readonly reportId = signal<number | null>(null);
  protected readonly needsDiagnosis = signal(false);
  private edited = false;

  ngOnInit(): void {
    const existing = this.session.report();
    if (existing) {
      this.reportId.set(existing.id);
      this.text.set(existing.text);
    } else if (!this.session.diagnosis()) {
      if (this.api.real) {
        this.needsDiagnosis.set(true);
      } else {
        this.text.set(SAMPLE_REPORT);
      }
    }
  }

  protected async generate(): Promise<void> {
    const d = this.session.diagnosis();
    if (!d) {
      return;
    }
    try {
      const r = await this.api.makeReport(d.ticket.id, this.symptom().trim() || null);
      this.session.report.set(r);
      this.reportId.set(r.id);
      this.text.set(r.text);
    } catch (e) {
      this.status.set(`Não consegui gerar o relatório (${(e as IpcError).code}).`);
    }
  }

  protected edit(value: string): void {
    this.text.set(value);
    this.edited = true;
  }

  private async keepEdits(): Promise<void> {
    const id = this.reportId();
    if (id !== null && this.edited) {
      await this.api.editReport(id, this.text());
      this.edited = false;
    }
  }

  protected async copy(): Promise<void> {
    try {
      await this.keepEdits();
      await navigator.clipboard.writeText(this.text());
      this.status.set('Texto copiado.');
    } catch {
      this.status.set('Não consegui copiar. Selecione o texto e copie com Ctrl+C.');
    }
  }

  protected async save(): Promise<void> {
    try {
      const id = this.reportId();
      if (id === null) {
        // Prototype sample without a stored report.
        const name = await this.api.saveReport(0, this.text());
        this.status.set(`Arquivo salvo como ${name}.`);
        return;
      }
      const where = await this.api.saveReport(id, this.text());
      this.edited = false;
      this.status.set(`Salvei o relatório em ${where}.`);
    } catch (e) {
      this.status.set(
        `Não consegui salvar o arquivo (${(e as IpcError).code}). Use "Copiar texto".`,
      );
    }
  }
}
