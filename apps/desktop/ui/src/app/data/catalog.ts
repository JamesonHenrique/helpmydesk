// SPDX-License-Identifier: GPL-3.0-or-later
// Typed access to the generated catalog texts (see catalog.generated.ts).
import { CONFIDENCE_PHRASES, CONSENTS, FAMILIES, FIXES, PROBES } from './catalog.generated';

export type Family = (typeof FAMILIES)[number];
export type Diagnosis = Family['diagnoses'][number];
export type Probe = (typeof PROBES)[number];
export type Fix = (typeof FIXES)[number];
export type Consent = (typeof CONSENTS)[number];
export type ConfidenceCode = keyof typeof CONFIDENCE_PHRASES;

/** Short hints under each menu option, in the person's words (prototype copy, not catalog). */
export const FAMILY_HINTS: Record<string, string> = {
  connectivity: 'Sites não abrem, Wi-Fi caiu',
  performance: 'Demora para abrir, trava',
  storage: 'Aviso de disco cheio',
  audio_video: 'Ninguém me ouve, sem som, câmera',
  app_error: 'Mensagem de erro, programa não abre',
};

export function family(code: string | null | undefined): Family | undefined {
  return FAMILIES.find((f) => f.code === code);
}

export function probe(id: string): Probe | undefined {
  return PROBES.find((p) => p.id === id);
}

export function fix(id: string): Fix | undefined {
  return FIXES.find((f) => f.id === id);
}

export function confidencePhrase(code: string): string {
  return CONFIDENCE_PHRASES[code as ConfidenceCode] ?? CONFIDENCE_PHRASES.unsure;
}

export function probesFor(platform: 'windows' | 'linux'): Probe[] {
  return PROBES.filter((p) => (p.platforms as readonly string[]).includes(platform));
}

export { CONSENTS, FAMILIES, FIXES, PROBES };
