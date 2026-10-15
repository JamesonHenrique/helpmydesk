// SPDX-License-Identifier: GPL-3.0-or-later
import { Component, signal } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

/** Font sizes the person can choose (Plan v1 III.13: "fonte ajustável"). */
const SIZES = ['1rem', '1.0625rem', '1.2rem', '1.35rem'];

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './app.html',
  styleUrl: './app.css',
})
export class App {
  protected readonly size = signal(1);

  protected resize(delta: number): void {
    const next = Math.min(SIZES.length - 1, Math.max(0, this.size() + delta));
    this.size.set(next);
    document.documentElement.style.setProperty('--font-size', SIZES[next]);
  }
}
