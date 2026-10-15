// SPDX-License-Identifier: GPL-3.0-or-later
// Spec 018: "Ligar a ajuda inteligente". The person never needs to know what an API key is:
// open OpenCode, copy, come back and paste. The key is tested by the core and kept only in the
// system vault; it is never shown again.
import { Component, OnInit, inject, output, signal } from '@angular/core';

import { CORE_API } from '../api/core-api';
import { AiLogDto, IpcError } from '../api/types';
import { CONSENTS, family } from '../data/catalog';

/** Plain words for what can go wrong. */
const WHY: Record<string, string> = {
  AI_KEY_REFUSED: 'Esse código não funcionou. Volte ao OpenCode e copie de novo.',
  AI_NO_KEY: 'Não encontrei um código. Volte ao OpenCode, clique em Copiar e tente de novo.',
  AI_NETWORK: 'Não consegui falar com o OpenCode agora. Confira a internet e tente de novo.',
  AI_RATE_LIMITED: 'O OpenCode pediu para esperar um pouco. Tente de novo em alguns minutos.',
  AI_KEY_STORE_FAILED: 'Não consegui guardar o código com segurança neste computador.',
};

@Component({
  selector: 'app-ai-connect',
  styles: `
    .steps li {
      margin-block: 0.5rem;
    }
    .steps li .btn {
      margin-right: 0.75rem;
    }
    input {
      display: block;
      width: 100%;
      margin-top: 0.35rem;
    }
  `,
  template: `
    @if (connected()) {
      <p class="ok"><strong>A ajuda inteligente está ligada.</strong></p>
      <p class="small">
        Só vai para a IA o que você escreve na tela inicial, sem seu nome nem o do computador. O
        texto não fica guardado aqui; só a data e o resultado de cada uso:
      </p>
      @if (log().length) {
        <ul class="small log">
          @for (e of log(); track $index) {
            <li>{{ when(e.at) }}: {{ what(e.outcome) }}</li>
          }
        </ul>
      }
      <button type="button" class="btn" (click)="disconnect()">Desligar e apagar o código</button>
    } @else {
      <p class="small">{{ consent }}</p>
      <ol class="steps">
        <li>
          <button type="button" class="btn primary" (click)="open()">Abrir o OpenCode</button>
          <span class="small">Entre com sua conta (ou crie uma, de graça).</span>
        </li>
        <li>Lá, clique em <strong>Copiar</strong> no seu código de acesso.</li>
        <li>
          <button type="button" class="btn primary" [disabled]="busy()" (click)="paste()">
            Colar o código e ligar
          </button>
        </li>
      </ol>
      @if (manual()) {
        <label class="small" for="ai-key">Se o botão não colar, clique aqui e aperte Ctrl+V:</label>
        <input
          id="ai-key"
          type="password"
          autocomplete="off"
          (paste)="pasted($event)"
          (keydown.enter)="connect($any($event.target).value)"
        />
      }
      @if (busy()) {
        <p aria-live="polite">Testando o código…</p>
      }
      @if (error()) {
        <p class="notice" role="alert">{{ error() }}</p>
      }
      <p class="small muted">
        Ao ligar, você concorda com o texto acima. Sem a IA, o modo básico continua.
      </p>
    }
  `,
})
export class AiConnectComponent implements OnInit {
  private readonly api = inject(CORE_API);
  readonly changed = output<boolean>();
  protected readonly consent = CONSENTS.find((c) => c.id === 'ai_opencode')?.text ?? '';
  protected readonly connected = signal(false);
  protected readonly busy = signal(false);
  protected readonly manual = signal(false);
  protected readonly error = signal('');
  protected readonly log = signal<AiLogDto[]>([]);

  async ngOnInit(): Promise<void> {
    try {
      this.connected.set((await this.api.aiStatus()).connected);
    } catch {
      this.connected.set(false);
    }
    try {
      this.log.set(await this.api.aiLog());
    } catch {
      this.log.set([]);
    }
  }

  protected when(at: number): string {
    return new Date(at * 1000).toLocaleString('pt-BR', {
      dateStyle: 'short',
      timeStyle: 'short',
    });
  }

  /** The recorded outcome in plain words. */
  protected what(outcome: string): string {
    if (outcome === 'ok:connection_test') {
      return 'teste do código';
    }
    if (outcome.startsWith('ok:')) {
      const code = outcome.slice(3);
      return code === 'unknown'
        ? 'não deu para saber o tipo de problema'
        : `entendeu: ${family(code)?.title ?? code}`;
    }
    return `não funcionou (${outcome})`;
  }

  protected async open(): Promise<void> {
    try {
      await this.api.openAiConsole();
    } catch {
      this.error.set('Não consegui abrir o navegador. Abra opencode.ai/auth no seu navegador.');
    }
  }

  protected async paste(): Promise<void> {
    this.error.set('');
    this.busy.set(true);
    try {
      const s = await this.api.aiConnectFromClipboard();
      this.connected.set(s.connected);
      this.changed.emit(s.connected);
    } catch (e) {
      const code = (e as IpcError).code;
      if (code === 'CLIPBOARD_UNAVAILABLE') {
        // No clipboard access (prototype, some desktops): paste by hand.
        this.manual.set(true);
      } else {
        this.error.set(WHY[code] ?? `Algo deu errado do meu lado (${code}).`);
        this.manual.set(true);
      }
    } finally {
      this.busy.set(false);
    }
  }

  protected pasted(event: ClipboardEvent): void {
    const text = event.clipboardData?.getData('text') ?? '';
    event.preventDefault();
    void this.connect(text);
  }

  protected async connect(key: string): Promise<void> {
    this.error.set('');
    this.busy.set(true);
    try {
      const s = await this.api.aiConnect(key.trim());
      this.connected.set(s.connected);
      this.manual.set(false);
      this.changed.emit(s.connected);
    } catch (e) {
      const code = (e as IpcError).code;
      this.error.set(WHY[code] ?? `Algo deu errado do meu lado (${code}).`);
    } finally {
      this.busy.set(false);
    }
  }

  protected async disconnect(): Promise<void> {
    const s = await this.api.aiDisconnect();
    this.connected.set(s.connected);
    this.changed.emit(s.connected);
  }
}
