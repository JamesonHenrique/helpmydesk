// SPDX-License-Identifier: GPL-3.0-or-later
import { Type } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { App } from './app';
import { routes } from './app.routes';
import { FAMILIES, PROBES, probesFor } from './data/catalog';
import { HomePage } from './pages/home';
import { DiagnosisPage } from './pages/diagnosis';
import { PlanPage } from './pages/plan';
import { ReportPage } from './pages/report';
import { SettingsPage } from './pages/settings';
import { PrototypeState } from './state/prototype';

async function render<T>(
  component: Type<T>,
  inputs: Record<string, unknown> = {},
  setup: (state: PrototypeState) => void = () => undefined,
) {
  await TestBed.configureTestingModule({
    imports: [component],
    providers: [provideRouter(routes)],
  }).compileComponents();
  setup(TestBed.inject(PrototypeState));
  const fixture = TestBed.createComponent(component);
  for (const [k, v] of Object.entries(inputs)) {
    fixture.componentRef.setInput(k, v);
  }
  if (!vi.isFakeTimers()) {
    await fixture.whenStable();
  }
  fixture.detectChanges();
  return { fixture, el: fixture.nativeElement as HTMLElement };
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
  it('says it is a prototype and has a skip link', async () => {
    const { el } = await render(App);
    expect(el.textContent).toContain('Protótipo');
    expect(el.querySelector('a.skip')?.getAttribute('href')).toBe('#conteudo');
  });
});

describe('HomePage', () => {
  it('offers the five families plus "Não sei explicar" in basic mode', async () => {
    const { el } = await render(HomePage);
    const tiles = [...el.querySelectorAll('.tile strong')].map((t) => t.textContent?.trim());
    expect(tiles).toEqual([...FAMILIES.map((f) => f.title), 'Não sei explicar']);
    expect((el.querySelector('#free') as HTMLInputElement).disabled).toBe(true);
  });
});

describe('DiagnosisPage', () => {
  afterEach(() => vi.useRealTimers());

  it('runs the checks without changing anything, then explains', async () => {
    vi.useFakeTimers();
    const { fixture, el } = await render(DiagnosisPage, { family: 'storage' });
    expect(el.textContent).toContain('Nada está sendo alterado agora.');
    vi.advanceTimersByTime(3000);
    fixture.detectChanges();
    expect(el.querySelector('h1')?.textContent).toContain('O que eu encontrei');
    const first = FAMILIES.find((f) => f.code === 'storage')?.diagnoses[0];
    expect(el.textContent).toContain(first?.title ?? '???');
  });

  it('marks checks that do not exist on the platform as skipped', async () => {
    vi.useFakeTimers();
    const { fixture, el } = await render(DiagnosisPage, { family: 'audio_video' }, (s) =>
      s.platform.set('linux'),
    );
    vi.advanceTimersByTime(3000);
    fixture.detectChanges();
    expect(el.textContent).toContain('não existe neste sistema');
  });
});

describe('PlanPage', () => {
  it('shows catalog risk and undo texts and warns about the system prompt', async () => {
    const net = FAMILIES.find((f) => f.code === 'connectivity');
    const dnsIndex = net?.diagnoses.findIndex((d) => d.fixes.length > 0) ?? -1;
    expect(dnsIndex).toBeGreaterThanOrEqual(0);
    const { el } = await render(PlanPage, { family: 'connectivity' }, (s) =>
      s.scenario.set(dnsIndex),
    );
    expect(el.textContent).toContain('Limpar a memória de endereços da internet');
    expect(el.textContent).toContain('Risco');
    expect(el.textContent).toContain('O sistema vai pedir sua permissão');
    expect([...el.querySelectorAll('button, a')].map((b) => b.textContent?.trim())).toContain(
      'Não, obrigado',
    );
  });
});

describe('ReportPage', () => {
  it('shows the real report@1 sample in an editable box', async () => {
    const { el } = await render(ReportPage);
    const box = el.querySelector('textarea') as HTMLTextAreaElement;
    expect(box.value.startsWith('<!-- report@1 -->')).toBe(true);
    expect(box.value).toContain('O HelpMyDesk não alterou nada neste computador');
  });
});

describe('SettingsPage', () => {
  it('lists what each check looks at, per platform', async () => {
    const { fixture, el } = await render(SettingsPage);
    expect(el.querySelectorAll('.checks li').length).toBe(probesFor('windows').length);
    TestBed.inject(PrototypeState).platform.set('linux');
    fixture.detectChanges();
    expect(el.querySelectorAll('.checks li').length).toBe(probesFor('linux').length);
    expect(el.textContent).toContain('Não guardo nem envio');
  });

  it('asks before erasing everything', async () => {
    const { fixture, el } = await render(SettingsPage);
    const erase = [...el.querySelectorAll('button')].find(
      (b) => b.textContent?.trim() === 'Apagar tudo',
    );
    erase?.click();
    fixture.detectChanges();
    expect(el.textContent).toContain('Não dá para desfazer');
  });
});
