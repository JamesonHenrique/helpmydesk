// SPDX-License-Identifier: GPL-3.0-or-later
// Stage H: the "não consegui" report, made by the core (report@1) and editable before it leaves
// the computer. Without a diagnosis, the browser prototype shows a sample.
import { Component, OnInit, computed, inject, signal } from '@angular/core';
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
      <div class="page-head">
        <p class="rule">Relatório técnico · gerado neste computador</p>
        <h1 id="t">{{ heading() }}</h1>
        <p class="lead">Preparo um relatório para você mostrar a quem for te ajudar.</p>
      </div>
      <div class="has-no">
        <div class="sheet has">
          <p class="rule has-rule">Contém</p>
          <ul class="checks">
            <li>
              <span class="mark ok" aria-hidden="true">✓</span
              ><span>Sua descrição do problema</span>
            </li>
            <li>
              <span class="mark ok" aria-hidden="true">✓</span
              ><span>O que eu verifiquei e encontrei</span>
            </li>
            <li>
              <span class="mark ok" aria-hidden="true">✓</span
              ><span>O que eu tentei e o resultado</span>
            </li>
            <li><span class="mark ok" aria-hidden="true">✓</span><span>Versão do sistema</span></li>
          </ul>
        </div>
        <div class="sheet">
          <p class="rule no-rule">Não contém</p>
          <ul class="checks">
            <li>
              <span class="mark bad" aria-hidden="true">✗</span
              ><span>Arquivos, senhas, nome de usuário</span>
            </li>
            <li>
              <span class="mark bad" aria-hidden="true">✗</span><span>Nome do computador</span>
            </li>
            <li>
              <span class="mark bad" aria-hidden="true">✗</span
              ><span>Nomes de programas abertos</span>
            </li>
          </ul>
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
          <div class="sheet sym-box">
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
            <div class="actions sym-actions">
              <button type="button" class="btn primary" (click)="generate()">
                Gerar relatório
              </button>
            </div>
          </div>
        }
        @if (text()) {
          <div class="report-paper">
            <div class="report-head" aria-hidden="true">
              <span class="report-title">HelpMyDesk · relatório do computador</span>
              <span class="report-dim">não enviado por e-mail — saída manual</span>
            </div>
            <label for="rel"
              ><strong>Ver e editar o relatório</strong>
              <span class="small muted">(apague o que não quiser compartilhar)</span></label
            >
            <textarea
              id="rel"
              maxlength="20000"
              [value]="text()"
              (input)="edit($any($event.target).value)"
            ></textarea>
          </div>
          <div class="actions">
            <button type="button" class="btn" (click)="copy()">Copiar texto</button>
            <button type="button" class="btn primary" (click)="save()">Salvar arquivo</button>
          </div>
        }
      }
      <p class="small muted" aria-live="polite">{{ status() }}</p>
      <p class="small send-note">
        Nada é enviado pelo HelpMyDesk. Você decide se e para quem entregar.
      </p>
    </section>
  `,
  styles: `
    .has-no {
      display: grid;
      gap: 0.9rem;
      grid-template-columns: 1fr 1fr;
      max-width: 52rem;
    }
    @media (max-width: 699px) {
      .has-no {
        grid-template-columns: 1fr;
      }
    }
    .has-rule {
      color: var(--ok);
      margin-bottom: 0.4rem;
    }
    .no-rule {
      color: var(--bad);
      margin-bottom: 0.4rem;
    }
    .sym-box {
      max-width: 52rem;
      display: grid;
      gap: 0.6rem;
    }
    .sym-actions {
      justify-content: flex-start;
    }
    .report-paper {
      max-width: 52rem;
      background: var(--surface-raised);
      border: 1px solid var(--line-strong);
      border-radius: var(--radius);
      padding: 1rem 1.25rem 1.25rem;
      display: grid;
      gap: 0.6rem;
    }
    .report-head {
      display: flex;
      justify-content: space-between;
      gap: 0.75rem;
      border-bottom: 2px solid var(--fg);
      padding-bottom: 0.45rem;
      font-family: var(--font-mono);
      font-size: 0.78rem;
      letter-spacing: 0.06em;
      text-transform: uppercase;
    }
    .report-title {
      font-weight: 700;
    }
    .report-dim {
      color: var(--muted);
      text-transform: none;
    }
    .send-note {
      border-top: 1px dashed var(--line-strong);
      padding-top: 0.6rem;
      max-width: 52rem;
    }
  `,
})
export class ReportPage implements OnInit {
  /** Only say "não consigo" when something really could not be solved; a report the person
   * asked for after a clean check is just a report. */
  protected readonly heading = computed(() => {
    const d = this.session.diagnosis();
    const o = this.session.outcome();
    const failed = (o && o.kind !== 'verified') || !!d?.trigger;
    return failed ? 'Isso eu não consigo resolver com segurança' : 'Relatório para levar a alguém';
  });
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
