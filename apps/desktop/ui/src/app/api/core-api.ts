// SPDX-License-Identifier: GPL-3.0-or-later
import { InjectionToken, inject } from '@angular/core';

import { PrototypeState } from '../state/prototype';
import { MockApi } from './mock';
import { TauriApi, isTauri } from './tauri';
import { CoreApi } from './types';

/** The core the screens talk to: the real one inside the desktop app, examples in a browser. */
export const CORE_API = new InjectionToken<CoreApi>('CoreApi', {
  providedIn: 'root',
  factory: () => (isTauri() ? new TauriApi() : new MockApi(inject(PrototypeState))),
});
