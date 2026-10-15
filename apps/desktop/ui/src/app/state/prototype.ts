// SPDX-License-Identifier: GPL-3.0-or-later
// In-memory state of the prototype. The real app reads and writes these through the Rust core
// (hmd-store) over Tauri IPC; nothing here is persisted.
import { Injectable, signal } from '@angular/core';

export type Autonomy = 'observe_only' | 'ask_always' | 'auto_simple';
export type Platform = 'windows' | 'linux';

@Injectable({ providedIn: 'root' })
export class PrototypeState {
  readonly platform = signal<Platform>('windows');
  readonly autonomy = signal<Autonomy>('ask_always');
  readonly consents = signal<ReadonlySet<string>>(new Set());
  readonly aiConnected = signal(false);
  /** Which diagnosis of the family the prototype shows (index in the generated list). */
  readonly scenario = signal(0);

  setConsent(id: string, granted: boolean): void {
    const next = new Set(this.consents());
    if (granted) {
      next.add(id);
    } else {
      next.delete(id);
    }
    this.consents.set(next);
  }
}
