// SPDX-License-Identifier: GPL-3.0-or-later
// Automatic accessibility checks (Plan v1 VIII.6: axe-core) on every screen. Color contrast is
// not computed in jsdom; it is covered by the reviewed color tokens and the manual pass.
import { Type } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter, withComponentInputBinding } from '@angular/router';
import axe from 'axe-core';

import { App } from './app';
import { routes } from './app.routes';
import { CORE_API } from './api/core-api';
import { FAMILIES } from './data/catalog';
import { ComputerPage } from './pages/computer';
import { DiagnosisPage } from './pages/diagnosis';
import { HelpPage } from './pages/help';
import { HistoryPage } from './pages/history';
import { HomePage } from './pages/home';
import { OnboardingPage } from './pages/onboarding';
import { PlanPage } from './pages/plan';
import { ReportPage } from './pages/report';
import { ResultPage } from './pages/result';
import { SettingsPage } from './pages/settings';
import { PrototypeState } from './state/prototype';
import { Session } from './state/session';

async function violations<T>(
  component: Type<T>,
  inputs: Record<string, unknown> = {},
): Promise<string[]> {
  const fixture = TestBed.createComponent(component);
  for (const [k, v] of Object.entries(inputs)) {
    fixture.componentRef.setInput(k, v);
  }
  fixture.detectChanges();
  for (let i = 0; i < 5; i++) {
    await fixture.whenStable();
    await new Promise((resolve) => setTimeout(resolve, 0));
    fixture.detectChanges();
  }
  const el = fixture.nativeElement as HTMLElement;
  document.body.appendChild(el);
  const result = await axe.run(el, {
    rules: {
      'color-contrast': { enabled: false },
      // A component is checked alone, not inside the page with its <main> and <h1> landmarks.
      region: { enabled: false },
      'landmark-one-main': { enabled: false },
      'page-has-heading-one': { enabled: false },
    },
  });
  el.remove();
  return result.violations.map(
    (v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`,
  );
}

describe('accessibility (axe-core)', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      providers: [provideRouter(routes, withComponentInputBinding())],
    }).compileComponents();
  });

  const net = FAMILIES.find((f) => f.code === 'connectivity');
  const dnsIndex = net?.diagnoses.findIndex((d) => d.fixes.length > 0) ?? 0;

  async function diagnosed(): Promise<void> {
    TestBed.inject(PrototypeState).scenario.set(dnsIndex);
    const api = TestBed.inject(CORE_API);
    const t = await api.startTicket('connectivity');
    TestBed.inject(Session).diagnosis.set(await api.diagnose(t.ticket.id));
  }

  it('shell', async () => expect(await violations(App)).toEqual([]));
  it('home', async () => expect(await violations(HomePage)).toEqual([]));
  it('onboarding', async () => expect(await violations(OnboardingPage)).toEqual([]));
  it('diagnosis', async () =>
    expect(await violations(DiagnosisPage, { family: 'connectivity' })).toEqual([]));
  it('plan', async () => {
    await diagnosed();
    expect(await violations(PlanPage, { family: 'connectivity' })).toEqual([]);
  });
  it('result', async () => {
    await diagnosed();
    expect(await violations(ResultPage, { family: 'connectivity' })).toEqual([]);
  });
  it('report', async () => expect(await violations(ReportPage)).toEqual([]));
  it('history', async () => expect(await violations(HistoryPage)).toEqual([]));
  it('settings', async () => expect(await violations(SettingsPage)).toEqual([]));
  it('computer', async () => expect(await violations(ComputerPage)).toEqual([]));
  it('help', async () => expect(await violations(HelpPage)).toEqual([]));
});
