// SPDX-License-Identifier: GPL-3.0-or-later
import { Type } from '@angular/core';
import { Router } from '@angular/router';
import { TestBed } from '@angular/core/testing';
import { provideRouter, withComponentInputBinding } from '@angular/router';

import { App } from './app';
import { routes } from './app.routes';
import { CORE_API } from './api/core-api';
import { DiagnosisDto } from './api/types';
import { FAMILIES, PROBES, probesFor } from './data/catalog';
import { AiConnectComponent } from './components/ai-connect';
import { APPROVAL_ARM_MS, ApprovalPage } from './pages/approval';
import { DiagnosisPage } from './pages/diagnosis';
import { HomePage } from './pages/home';
import { PlanPage } from './pages/plan';
import { ReportPage } from './pages/report';
import { ResultPage } from './pages/result';
import { SettingsPage } from './pages/settings';
import { PrototypeState } from './state/prototype';
import { Session } from './state/session';

async function setup(prepare: () => Promise<void> | void = () => undefined): Promise<void> {
  await TestBed.configureTestingModule({
    providers: [provideRouter(routes, withComponentInputBinding())],
  }).compileComponents();
  await prepare();
}

async function render<T>(component: Type<T>, inputs: Record<string, unknown> = {}) {
  const fixture = TestBed.createComponent(component);
  for (const [k, v] of Object.entries(inputs)) {
    fixture.componentRef.setInput(k, v);
  }
  fixture.detectChanges();
  // The example core answers through promise chains the zoneless scheduler does not track.
  for (let i = 0; i < 5; i++) {
    await fixture.whenStable();
    await new Promise((resolve) => setTimeout(resolve, 0));
    fixture.detectChanges();
  }
  return { fixture, el: fixture.nativeElement as HTMLElement };
}

/** Runs the example core for a family and keeps the result in the session, as the app does. */
async function diagnosed(code: string, scenario = 0): Promise<void> {
  TestBed.inject(PrototypeState).scenario.set(scenario);
  const api = TestBed.inject(CORE_API);
  const t = await api.startTicket(code);
  TestBed.inject(Session).diagnosis.set(await api.diagnose(t.ticket.id));
}

describe('generated catalog', () => {
  it('has the five families and every probe disclosed', () => {
    expect(FAMILIES.map((f) => f.code)).toEqual([
      'connectivity',
      'performance',
      'storage',
      'audio_video',
      'app_error',
    ]);
    for (const p of PROBES) {
      expect(p.looksAt, p.id).toBeTruthy();
      expect(p.never, p.id).toBeTruthy();
    }
  });
});

describe('App shell', () => {
  it('says it is a prototype in the browser and has a skip link', async () => {
    await setup();
    const { el } = await render(App);
    expect(el.textContent).toContain('Protótipo');
    expect(el.querySelector('a.skip')?.getAttribute('href')).toBe('#conteudo');
    expect(TestBed.inject(CORE_API).real).toBe(false);
  });
});

describe('HomePage', () => {
  it('offers the five families plus "Não sei explicar" in basic mode', async () => {
    await setup();
    const { el } = await render(HomePage);
    const tiles = [...el.querySelectorAll('.tile strong')].map((t) => t.textContent?.trim());
    expect(tiles).toEqual([...FAMILIES.map((f) => f.title), 'Não sei explicar']);
    expect((el.querySelector('#free') as HTMLInputElement).disabled).toBe(true);
  });
});

describe('DiagnosisPage', () => {
  it('runs the checks in the core and explains the result', async () => {
    await setup();
    const { el } = await render(DiagnosisPage, { family: 'storage' });
    expect(el.querySelector('h1')?.textContent).toContain('O que eu encontrei');
    const first = FAMILIES.find((f) => f.code === 'storage')?.diagnoses[0];
    expect(el.textContent).toContain(first?.title ?? '???');
    expect(TestBed.inject(Session).diagnosis()?.ticket.family).toBe('storage');
  });

  it('marks checks that do not exist on the platform as skipped', async () => {
    await setup(() => TestBed.inject(PrototypeState).platform.set('linux'));
    const { el } = await render(DiagnosisPage, { family: 'audio_video' });
    expect(el.textContent).toContain('não existe neste sistema');
  });
});

describe('PlanPage', () => {
  it('shows catalog risk and undo texts and warns about the system prompt', async () => {
    const net = FAMILIES.find((f) => f.code === 'connectivity');
    const dnsIndex = net?.diagnoses.findIndex((d) => d.fixes.length > 0) ?? -1;
    expect(dnsIndex).toBeGreaterThanOrEqual(0);
    await setup(() => diagnosed('connectivity', dnsIndex));
    const { el } = await render(PlanPage, { family: 'connectivity' });
    expect(el.textContent).toContain('Limpar a memória de endereços da internet');
    expect(el.textContent).toContain('Risco');
    expect(el.textContent).toContain('O sistema vai pedir sua permissão');
    expect([...el.querySelectorAll('button, a')].map((b) => b.textContent?.trim())).toContain(
      'Não, obrigado',
    );
  });

  it('offers nothing to run when "Só olhar" is chosen', async () => {
    const net = FAMILIES.find((f) => f.code === 'connectivity');
    const dnsIndex = net?.diagnoses.findIndex((d) => d.fixes.length > 0) ?? -1;
    await setup(async () => {
      TestBed.inject(PrototypeState).autonomy.set('observe_only');
      await diagnosed('connectivity', dnsIndex);
    });
    const { el } = await render(PlanPage, { family: 'connectivity' });
    expect(el.textContent).toContain('não tenho correção para propor');
  });
});

describe('ResultPage', () => {
  it('says plainly when the fix is not available in this version', async () => {
    const net = FAMILIES.find((f) => f.code === 'connectivity');
    const dnsIndex = net?.diagnoses.findIndex((d) => d.fixes.length > 0) ?? -1;
    await setup(async () => {
      TestBed.inject(PrototypeState).mockOutcome.set('not_available');
      await diagnosed('connectivity', dnsIndex);
    });
    const { el } = await render(ResultPage, { family: 'connectivity' });
    expect(el.textContent).toContain('Essa correção ainda não está disponível');
    expect(el.textContent).toContain('Nada foi alterado.');
  });

  it('asks the person before calling it resolved', async () => {
    const net = FAMILIES.find((f) => f.code === 'connectivity');
    const dnsIndex = net?.diagnoses.findIndex((d) => d.fixes.length > 0) ?? -1;
    await setup(() => diagnosed('connectivity', dnsIndex));
    const { el } = await render(ResultPage, { family: 'connectivity' });
    expect(el.textContent).toContain('Para você também voltou a funcionar?');
  });
});

describe('ApprovalPage', () => {
  it('outside the desktop app there is no approval window plan', async () => {
    await setup();
    const { el } = await render(ApprovalPage, { ticket: '1', plan: '1' });
    expect(el.textContent).toContain('Este plano não está mais disponível');
  });

  it('nothing is pre-selected and "Sim" waits after showing or regaining focus', async () => {
    const approved: string[][] = [];
    const plan = {
      planId: 1,
      planHash: 'sha256:0000000000000000000000',
      steps: [
        {
          stepId: 's1',
          capability: 'net.dns.flush_cache',
          title: 'Limpar a memória de endereços da internet',
          risk: 'baixo',
          undo: 'não precisa',
          requiresElevation: true,
          perActionApproval: true,
        },
      ],
    };
    await TestBed.configureTestingModule({
      providers: [
        provideRouter(routes, withComponentInputBinding()),
        {
          provide: CORE_API,
          useValue: {
            real: false,
            plan: async () => plan,
            approve: async (_t: string, _p: number, steps: string[]) => {
              approved.push(steps);
            },
          },
        },
      ],
    }).compileComponents();
    const { fixture, el } = await render(ApprovalPage, { ticket: '1', plan: '1' });
    const yes = el.querySelector('button.primary') as HTMLButtonElement;
    const box = el.querySelector('input[type=checkbox]') as HTMLInputElement;
    expect(box.checked).toBe(false);
    box.click();
    fixture.detectChanges();
    expect(yes.disabled).toBe(true);
    yes.click();
    expect(approved.length).toBe(0);
    await new Promise((resolve) => setTimeout(resolve, APPROVAL_ARM_MS + 50));
    fixture.detectChanges();
    expect(yes.disabled).toBe(false);
    window.dispatchEvent(new Event('focus'));
    fixture.detectChanges();
    expect(yes.disabled).toBe(true);
    await new Promise((resolve) => setTimeout(resolve, APPROVAL_ARM_MS + 50));
    fixture.detectChanges();
    yes.click();
    await fixture.whenStable();
    expect(approved).toEqual([['s1']]);
  });
});

describe('ResultPage', () => {
  it('explains a refusal in plain words', async () => {
    await TestBed.configureTestingModule({
      providers: [
        provideRouter(routes, withComponentInputBinding()),
        {
          provide: CORE_API,
          useValue: {
            real: false,
            execute: async () => {
              throw { code: 'OBSERVE_ONLY', message: '' };
            },
          },
        },
      ],
    }).compileComponents();
    TestBed.inject(Session).diagnosis.set({
      ticket: { id: '1' },
      plan: { planId: 1, planHash: 'x', steps: [] },
    } as unknown as DiagnosisDto);
    const { el } = await render(ResultPage, { family: 'connectivity' });
    expect(el.textContent).toContain('só observar');
    expect(el.textContent).not.toContain('OBSERVE_ONLY');
  });
});

describe('ReportPage', () => {
  it('shows the real report@1 sample in an editable box', async () => {
    await setup();
    const { el } = await render(ReportPage);
    const box = el.querySelector('textarea') as HTMLTextAreaElement;
    expect(box.value.startsWith('<!-- report@1 -->')).toBe(true);
    expect(box.value).toContain('O HelpMyDesk não alterou nada neste computador');
  });

  it('a report the person asked for is not called a failure', async () => {
    await setup();
    const { el } = await render(ReportPage);
    expect(el.querySelector('h1')?.textContent).toContain('Relatório para levar a alguém');
  });
});

describe('SettingsPage', () => {
  it('lists what each check looks at, per platform', async () => {
    await setup();
    const { fixture, el } = await render(SettingsPage);
    expect(el.querySelectorAll('.checks li').length).toBe(probesFor('windows').length);
    TestBed.inject(PrototypeState).platform.set('linux');
    fixture.detectChanges();
    expect(el.querySelectorAll('.checks li').length).toBe(probesFor('linux').length);
    expect(el.textContent).toContain('Não guardo nem envio');
  });

  it('asks before erasing everything', async () => {
    await setup();
    const { fixture, el } = await render(SettingsPage);
    const erase = [...el.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === 'Apagar tudo',
    );
    erase?.click();
    fixture.detectChanges();
    expect(el.textContent).toContain('Não dá para desfazer');
  });
});

describe('MockApi', () => {
  it('continues an open ticket of the same family', async () => {
    await setup();
    const api = TestBed.inject(CORE_API);
    const a = await api.startTicket('storage');
    const b = await api.startTicket('storage');
    expect(b.reused).toBe(true);
    expect(b.ticket.id).toBe(a.ticket.id);
  });
});

function aiStub(over: Record<string, unknown> = {}) {
  let connected = false;
  return {
    real: false,
    aiStatus: async () => ({ connected, consoleUrl: 'https://opencode.ai/auth' }),
    aiConnect: async (key: string) => {
      if (key !== 'good') {
        throw { code: 'AI_KEY_REFUSED', message: '' };
      }
      connected = true;
      return { connected, consoleUrl: '' };
    },
    aiDisconnect: async () => {
      connected = false;
      return { connected, consoleUrl: '' };
    },
    openAiConsole: async () => undefined,
    aiPreview: async (t: string) => t.replace('james', '[usuário]'),
    aiLog: async () => [{ at: 1_760_400_000, model: 'm', outcome: 'ok:storage' }],
    aiConnectFromClipboard: async () => {
      throw { code: 'CLIPBOARD_UNAVAILABLE', message: '' };
    },
    ...over,
  };
}

async function setupWith(api: unknown): Promise<void> {
  await TestBed.configureTestingModule({
    providers: [
      provideRouter(routes, withComponentInputBinding()),
      { provide: CORE_API, useValue: api },
    ],
  }).compileComponents();
}

describe('AiConnectComponent (spec 018)', () => {
  it('a refused code is explained; a good one turns the AI on', async () => {
    await setupWith(aiStub());
    const { fixture, el } = await render(AiConnectComponent);
    const pasteBtn = [...el.querySelectorAll('button')].find((b) =>
      b.textContent?.includes('Colar'),
    ) as HTMLButtonElement;
    pasteBtn.click();
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r, 0));
    fixture.detectChanges();
    const input = el.querySelector('#ai-key') as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(input.type).toBe('password');
    input.value = 'bad';
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r, 0));
    fixture.detectChanges();
    expect(el.textContent).toContain('Esse código não funcionou');
    input.value = 'good';
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r, 0));
    fixture.detectChanges();
    expect(el.textContent).toContain('A ajuda inteligente está ligada');
    expect(el.textContent).not.toContain('good');
  });
});

describe('HomePage with the AI on (spec 018)', () => {
  it('free text goes to the diagnosis of the family the AI picked', async () => {
    await setupWith(
      aiStub({
        aiStatus: async () => ({ connected: true, consoleUrl: '' }),
        aiClassify: async () => ({ family: 'storage', confidence: 'high', question: null }),
      }),
    );
    const router = TestBed.inject(Router);
    const go: unknown[] = [];
    router.navigate = (async (commands: unknown[]) => {
      go.push(commands);
      return true;
    }) as typeof router.navigate;
    const { fixture, el } = await render(HomePage);
    const input = el.querySelector('#free') as HTMLInputElement;
    expect(input.disabled).toBe(false);
    input.value = 'apareceu disco cheio';
    input.dispatchEvent(new Event('input'));
    fixture.detectChanges();
    (el.querySelector('form.free') as HTMLFormElement).dispatchEvent(new Event('submit'));
    await fixture.whenStable();
    await new Promise((r) => setTimeout(r, 0));
    expect(go).toEqual([['/diagnostico', 'storage']]);
  });

  it('shows exactly what goes to the AI, already redacted', async () => {
    await setupWith(aiStub({ aiStatus: async () => ({ connected: true, consoleUrl: '' }) }));
    const { fixture, el } = await render(HomePage);
    const input = el.querySelector('#free') as HTMLInputElement;
    input.value = 'o PC do james travou';
    input.dispatchEvent(new Event('input'));
    for (let i = 0; i < 3; i++) {
      await fixture.whenStable();
      await new Promise((r) => setTimeout(r, 0));
      fixture.detectChanges();
    }
    expect(el.textContent).toContain('«o PC do [usuário] travou»');
  });

  it('a low-confidence answer asks instead of guessing', async () => {
    await setupWith(
      aiStub({
        aiStatus: async () => ({ connected: true, consoleUrl: '' }),
        aiClassify: async () => ({
          family: 'storage',
          confidence: 'low',
          question: 'O aviso fala de disco?',
        }),
      }),
    );
    const { fixture, el } = await render(HomePage);
    const input = el.querySelector('#free') as HTMLInputElement;
    input.value = 'algo estranho';
    input.dispatchEvent(new Event('input'));
    fixture.detectChanges();
    (el.querySelector('form.free') as HTMLFormElement).dispatchEvent(new Event('submit'));
    for (let i = 0; i < 3; i++) {
      await fixture.whenStable();
      await new Promise((r) => setTimeout(r, 0));
      fixture.detectChanges();
    }
    expect(el.textContent).toContain('O aviso fala de disco?');
  });
});
