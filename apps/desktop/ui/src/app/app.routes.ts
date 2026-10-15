// SPDX-License-Identifier: GPL-3.0-or-later
import { Routes } from '@angular/router';

import { DiagnosisPage } from './pages/diagnosis';
import { HelpPage } from './pages/help';
import { HistoryPage } from './pages/history';
import { HomePage } from './pages/home';
import { OnboardingPage } from './pages/onboarding';
import { PlanPage } from './pages/plan';
import { ReportPage } from './pages/report';
import { ResultPage } from './pages/result';
import { SettingsPage } from './pages/settings';

export const routes: Routes = [
  { path: '', component: HomePage, title: 'HelpMyDesk' },
  { path: 'boas-vindas', component: OnboardingPage, title: 'Boas-vindas · HelpMyDesk' },
  { path: 'diagnostico/:family', component: DiagnosisPage, title: 'Diagnóstico · HelpMyDesk' },
  { path: 'plano/:family', component: PlanPage, title: 'O que eu proponho · HelpMyDesk' },
  { path: 'resultado/:family', component: ResultPage, title: 'Resultado · HelpMyDesk' },
  { path: 'relatorio', component: ReportPage, title: 'Relatório · HelpMyDesk' },
  { path: 'historico', component: HistoryPage, title: 'Histórico · HelpMyDesk' },
  { path: 'configuracoes', component: SettingsPage, title: 'Configurações · HelpMyDesk' },
  { path: 'ajuda', component: HelpPage, title: 'Ajuda · HelpMyDesk' },
  { path: '**', redirectTo: '' },
];
