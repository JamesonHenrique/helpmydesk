// SPDX-License-Identifier: GPL-3.0-or-later
// Stage H: the "não consegui" report, editable before it leaves the computer. The sample text is
// rendered by the real Rust renderer (report@1) from fixed example data.
import { Component, signal } from '@angular/core';

import { SAMPLE_REPORT } from '../data/catalog.generated';

@Component({
  selector: 'app-report',
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Isso eu não consigo resolver com segurança</h1>
      <p>Preparei um relatório para você mostrar a quem for te ajudar.</p>
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
      <label for="rel"
        ><strong>Ver e editar o relatório</strong>
        <span class="small muted">(apague o que não quiser compartilhar)</span></label
      >
      <textarea id="rel" [value]="text()" (input)="text.set($any($event.target).value)"></textarea>
      <div class="actions">
        <button type="button" class="btn" (click)="copy()">Copiar texto</button>
        <button type="button" class="btn primary" (click)="save()">Salvar arquivo</button>
      </div>
      <p class="small muted" aria-live="polite">{{ status() }}</p>
      <p class="small">Nada é enviado pelo HelpMyDesk. Você decide se e para quem entregar.</p>
    </section>
  `,
})
export class ReportPage {
  protected readonly text = signal(SAMPLE_REPORT);
  protected readonly status = signal('');

  protected async copy(): Promise<void> {
    try {
      await navigator.clipboard.writeText(this.text());
      this.status.set('Texto copiado.');
    } catch {
      this.status.set('Não consegui copiar. Selecione o texto e copie com Ctrl+C.');
    }
  }

  protected save(): void {
    const blob = new Blob([this.text()], { type: 'text/markdown;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'relatorio-helpmydesk.md';
    a.click();
    URL.revokeObjectURL(url);
    this.status.set('Arquivo salvo como relatorio-helpmydesk.md.');
  }
}
