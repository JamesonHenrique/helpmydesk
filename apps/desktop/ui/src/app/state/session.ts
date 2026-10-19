// SPDX-License-Identifier: GPL-3.0-or-later
// The ticket being worked on, shared by the screens of one attendance.
import { Injectable, signal } from '@angular/core';

import { DiagnosisDto, OutcomeDto, ReportDto } from '../api/types';

@Injectable({ providedIn: 'root' })
export class Session {
  readonly diagnosis = signal<DiagnosisDto | null>(null);
  readonly outcome = signal<OutcomeDto | null>(null);
  readonly report = signal<ReportDto | null>(null);
  /** What the person said through the AI path (redacted again at the core before sending). */
  readonly story = signal('');
  /** Spec 021: at most one extra AI classification per attendance, when the checks
   * contradicted the family the AI picked first. */
  readonly extraClassify = signal(false);

  reset(): void {
    this.diagnosis.set(null);
    this.outcome.set(null);
    this.report.set(null);
  }

  /** A new text starts a new attendance; the checks of the previous one are discarded. */
  beginAttendance(story: string): void {
    this.story.set(story);
    this.extraClassify.set(false);
    this.reset();
  }
}
