// SPDX-License-Identifier: GPL-3.0-or-later
// Browser prototype: answers like the core would, from the catalog texts and example data.
// Nothing is checked or changed on the computer.
import { CATALOG_VERSION, SAMPLE_REPORT } from '../data/catalog.generated';
import { FAMILIES, confidencePhrase, family, fix, probe } from '../data/catalog';
import { PrototypeState } from '../state/prototype';
import {
  AiClassificationDto,
  AiStatusDto,
  ApprovalDto,
  CoreApi,
  DiagnosisDto,
  IpcError,
  OutcomeDto,
  PlanDto,
  ReportDto,
  SettingsDto,
  StartDto,
  StartupDto,
  SystemDto,
  TicketDto,
} from './types';

const NOW = 1_760_400_000;

export class MockApi implements CoreApi {
  readonly real = false;
  private tickets: TicketDto[] = [];
  private nextId = 1;
  private nextPlan = 1;

  constructor(private readonly state: PrototypeState) {}

  async startup(): Promise<StartupDto> {
    return { opened: 'opened', interrupted: 0, housekept: 0 };
  }

  async startTicket(code: string): Promise<StartDto> {
    const f = family(code);
    if (!f) {
      throw { code: 'UNKNOWN_FAMILY', message: code };
    }
    const open = this.tickets.find(
      (t) =>
        t.family === code && !['closed', 'cancelled', 'resolved', 'unresolved'].includes(t.state),
    );
    if (open) {
      return { ticket: open, reused: true, reopened: false };
    }
    const ticket: TicketDto = {
      id: String(this.nextId++),
      family: code,
      familyTitle: f.title,
      state: 'open',
      reopenCount: 0,
      lastTrigger: null,
      createdAt: NOW,
      lastChangedAt: NOW,
    };
    this.tickets.push(ticket);
    return { ticket, reused: false, reopened: false };
  }

  private ticket(id: string): TicketDto {
    const t = this.tickets.find((x) => x.id === id);
    if (!t) {
      throw { code: 'NO_SUCH_TICKET', message: id };
    }
    return t;
  }

  async diagnose(ticketId: string): Promise<DiagnosisDto> {
    const t = this.ticket(ticketId);
    const f = family(t.family) ?? FAMILIES[0];
    const d = f.diagnoses[this.state.scenario()] ?? f.diagnoses[0];
    const platform = this.state.platform();
    const steps =
      this.state.autonomy() === 'observe_only'
        ? []
        : d.fixes.flatMap((id, i) => {
            const x = fix(id);
            return x
              ? [
                  {
                    stepId: `s${i + 1}`,
                    capability: x.id,
                    title: x.title,
                    risk: x.risk ?? '',
                    undo: x.undo ?? '',
                    requiresElevation: x.requiresElevation,
                    perActionApproval: true,
                  },
                ]
              : [];
          });
    const plan: PlanDto | null = steps.length
      ? { planId: this.nextPlan++, planHash: 'sha256:exemplo', steps }
      : null;
    t.state = plan ? 'awaiting_user' : 'in_progress';
    return {
      ticket: { ...t },
      sessionId: 1,
      diagnosisId: d.id,
      confidence: d.confidence,
      confidencePhrase: confidencePhrase(d.confidence),
      title: d.title,
      explanation: d.explanation,
      steps: [...d.steps],
      isFallback: d === f.diagnoses[f.diagnoses.length - 1],
      trigger: d.report,
      probes: f.probes.map((id) => {
        const p = probe(id);
        const here = p ? (p.platforms as readonly string[]).includes(platform) : false;
        return {
          probeId: id,
          title: p?.title ?? id,
          status: here ? 'ok' : 'skipped',
          findings: [],
          reason: here ? null : 'PLATFORM_UNSUPPORTED',
        };
      }),
      plan,
    };
  }

  async requestApproval(
    ticketId: string,
    plan: PlanDto,
    stepIds: string[],
  ): Promise<'approved' | 'declined'> {
    await this.approve(ticketId, plan.planId, stepIds);
    return 'approved';
  }

  async plan(): Promise<PlanDto> {
    throw { code: 'NO_PENDING_PLAN', message: 'prototype' };
  }

  async approve(ticketId: string, planId: number, stepIds: string[]): Promise<ApprovalDto> {
    this.ticket(ticketId).state = 'in_progress';
    return { nonce: planId, planHash: 'sha256:exemplo', stepIds, expiresAt: NOW + 600 };
  }

  async decline(ticketId: string): Promise<void> {
    this.ticket(ticketId).state = 'in_progress';
  }

  async execute(ticketId: string): Promise<OutcomeDto> {
    const t = this.ticket(ticketId);
    const kind = this.state.mockOutcome();
    if (kind === 'verified') {
      t.state = 'resolved_pending_confirmation';
      return { kind, stepId: null, trigger: null };
    }
    if (kind === 'not_available') {
      return { kind, stepId: 's1', trigger: null };
    }
    t.state = 'unresolved';
    t.lastTrigger = 'REP-03';
    return { kind, stepId: null, trigger: 'REP-03' };
  }

  async confirm(ticketId: string, fixed: boolean): Promise<TicketDto> {
    const t = this.ticket(ticketId);
    t.state = fixed ? 'resolved' : 'in_progress';
    return { ...t };
  }

  async makeReport(ticketId: string): Promise<ReportDto> {
    const t = this.tickets.find((x) => x.id === ticketId);
    if (t && t.state === 'in_progress') {
      t.state = 'unresolved';
    }
    return { id: 1, text: SAMPLE_REPORT };
  }

  async editReport(): Promise<void> {}

  async saveReport(_reportId: number, text: string): Promise<string> {
    const name = 'relatorio-helpmydesk.md';
    const url = URL.createObjectURL(new Blob([text], { type: 'text/markdown;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = name;
    a.click();
    URL.revokeObjectURL(url);
    return name;
  }

  async history(): Promise<TicketDto[]> {
    return this.tickets.map((t) => ({ ...t }));
  }

  async settings(): Promise<SettingsDto> {
    return { autonomy: this.state.autonomy(), consents: [...this.state.consents()] };
  }

  async aiStatus(): Promise<AiStatusDto> {
    return { connected: this.state.aiConnected(), consoleUrl: 'https://opencode.ai/auth' };
  }

  async aiConnect(key: string): Promise<AiStatusDto> {
    if (key.trim() === '' || key.includes('errad')) {
      throw { code: 'AI_KEY_REFUSED', message: 'example' } as IpcError;
    }
    this.state.aiConnected.set(true);
    return this.aiStatus();
  }

  /** Prototype: there is no clipboard access, so the screen falls back to pasting by hand. */
  async aiConnectFromClipboard(): Promise<AiStatusDto> {
    throw { code: 'CLIPBOARD_UNAVAILABLE', message: 'prototype' } as IpcError;
  }

  async aiDisconnect(): Promise<AiStatusDto> {
    this.state.aiConnected.set(false);
    return this.aiStatus();
  }

  /** Prototype: a few keywords stand in for the model. */
  async aiClassify(text: string): Promise<AiClassificationDto> {
    const t = text.toLowerCase();
    const family = /internet|wi-?fi|site/.test(t)
      ? 'connectivity'
      : /lent|trav/.test(t)
        ? 'performance'
        : /espaço|disco|cheio/.test(t)
          ? 'storage'
          : /som|microfone|câmera|camera/.test(t)
            ? 'audio_video'
            : /erro|não abre/.test(t)
              ? 'app_error'
              : null;
    return {
      family,
      confidence: family ? 'medium' : 'low',
      question: family ? null : 'Pode contar um pouco mais? Por exemplo, o que aparece na tela.',
    };
  }

  async openAiConsole(): Promise<void> {}

  async system(): Promise<SystemDto> {
    const linux = this.state.platform() === 'linux';
    return {
      osName: linux ? 'Ubuntu (exemplo)' : 'Windows (exemplo)',
      osVersion: linux ? '24.04' : '11',
      kernelVersion: null,
      arch: 'x86_64',
      catalogVersion: CATALOG_VERSION,
    };
  }

  async setAutonomy(mode: SettingsDto['autonomy']): Promise<void> {
    this.state.autonomy.set(mode);
  }

  async setConsent(scope: string, granted: boolean): Promise<void> {
    this.state.setConsent(scope, granted);
  }

  async exportAll(): Promise<string> {
    return `<!-- export@1 -->\n# Meus dados no HelpMyDesk\n\n(Protótipo: ${this.tickets.length} chamado(s) de exemplo.)\n`;
  }

  async eraseAll(): Promise<number> {
    const n = this.tickets.length;
    this.tickets = [];
    this.state.consents.set(new Set());
    this.state.autonomy.set('ask_always');
    this.state.aiConnected.set(false);
    return n;
  }
}
