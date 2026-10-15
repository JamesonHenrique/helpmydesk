// SPDX-License-Identifier: GPL-3.0-or-later
// The ticket being worked on, shared by the screens of one attendance.
import { Injectable, signal } from '@angular/core';

import { DiagnosisDto, OutcomeDto, ReportDto } from '../api/types';

@Injectable({ providedIn: 'root' })
export class Session {
  readonly diagnosis = signal<DiagnosisDto | null>(null);
  readonly outcome = signal<OutcomeDto | null>(null);
  readonly report = signal<ReportDto | null>(null);

  reset(): void {
    this.diagnosis.set(null);
    this.outcome.set(null);
    this.report.set(null);
  }
}
