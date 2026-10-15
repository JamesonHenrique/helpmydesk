// SPDX-License-Identifier: GPL-3.0-or-later
import { Component, OnInit, inject, signal } from '@angular/core';
import { Router, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

import { CORE_API } from './api/core-api';

/** Font sizes the person can choose (Plan v1 III.13: "fonte ajustável"). */
const SIZES = ['1rem', '1.0625rem', '1.2rem', '1.35rem'];

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App implements OnInit {
  protected readonly api = inject(CORE_API);
  private readonly router = inject(Router);
  protected readonly size = signal(1);
  protected readonly notice = signal('');

  async ngOnInit(): Promise<void> {
    try {
      const s = await this.api.startup();
      if (this.api.real && s.opened === 'created') {
        // First run: explain what the app does and does not do before anything else.
        void this.router.navigate(['/boas-vindas']);
      }
      const parts: string[] = [];
      if (s.opened === 'recreated') {
        parts.push('O histórico estava danificado: guardei a cópia antiga e comecei um novo.');
      }
      if (s.interrupted > 0) {
        parts.push('Uma correção foi interrompida da última vez. Não vou repeti-la sozinho.');
      }
      this.notice.set(parts.join(' '));
    } catch {
      this.notice.set('');
    }
  }

  protected resize(delta: number): void {
    const next = Math.min(SIZES.length - 1, Math.max(0, this.size() + delta));
    this.size.set(next);
    document.documentElement.style.setProperty('--font-size', SIZES[next]);
  }
}
