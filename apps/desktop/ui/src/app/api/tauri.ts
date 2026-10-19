// SPDX-License-Identifier: GPL-3.0-or-later
// The real core, through Tauri commands (apps/desktop/src-tauri). Approval happens only in the
// separate approval window, which reports back with an event (INV-04).
import { invoke } from '@tauri-apps/api/core';
import { UnlistenFn, emit, listen } from '@tauri-apps/api/event';

import {
  AiClassificationDto,
  AiLogDto,
  AiStatusDto,
  ApprovalDto,
  CoreApi,
  DiagnosisDto,
  OutcomeDto,
  PlanDto,
  ReportDto,
  SettingsDto,
  StartDto,
  StartupDto,
  SystemDto,
  TicketDto,
} from './types';

/** Minutes east of UTC on this computer, so reports show local dates (validated in the core). */
function utcOffsetMinutes(): number {
  return -new Date().getTimezoneOffset();
}

/** Event the approval window emits when the person decides. */
export const APPROVAL_EVENT = 'hmd://approval-decided';

export interface ApprovalDecision {
  ticketId: string;
  planId: number;
  /** `closed`: the window was closed without a decision (counts as declined). */
  decision: 'approved' | 'declined' | 'closed';
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export class TauriApi implements CoreApi {
  readonly real = true;

  startup(): Promise<StartupDto> {
    return invoke('startup');
  }
  startTicket(family: string): Promise<StartDto> {
    return invoke('start_ticket', { family });
  }
  diagnose(ticketId: string): Promise<DiagnosisDto> {
    return invoke('diagnose', { ticketId });
  }
  async requestApproval(ticketId: string, plan: PlanDto): Promise<'approved' | 'declined'> {
    let resolveDecision: (d: 'approved' | 'declined') => void = () => undefined;
    const decided = new Promise<'approved' | 'declined'>((resolve) => (resolveDecision = resolve));
    // Listen before opening the window, and only for this plan; stop at the first answer.
    let unlisten: UnlistenFn | undefined;
    let answered = false;
    unlisten = await listen<ApprovalDecision>(APPROVAL_EVENT, (event) => {
      if (answered || event.payload.ticketId !== ticketId || event.payload.planId !== plan.planId) {
        return;
      }
      answered = true;
      unlisten?.();
      resolveDecision(event.payload.decision === 'approved' ? 'approved' : 'declined');
    });
    try {
      await invoke('open_approval', { ticketId, planId: plan.planId });
    } catch (e) {
      unlisten();
      throw e;
    }
    return decided;
  }
  plan(ticketId: string, planId: number): Promise<PlanDto> {
    return invoke('plan', { ticketId, planId });
  }
  async approve(ticketId: string, planId: number, stepIds: string[]): Promise<ApprovalDto> {
    const approval = await invoke<ApprovalDto>('approve', { ticketId, planId, stepIds });
    await emit(APPROVAL_EVENT, {
      ticketId,
      planId,
      decision: 'approved',
    } satisfies ApprovalDecision);
    return approval;
  }
  async decline(ticketId: string, planId: number): Promise<void> {
    await invoke('decline', { ticketId, planId });
    await emit(APPROVAL_EVENT, {
      ticketId,
      planId,
      decision: 'declined',
    } satisfies ApprovalDecision);
  }
  execute(ticketId: string, planId: number): Promise<OutcomeDto> {
    return invoke('execute', { ticketId, planId });
  }
  confirm(ticketId: string, fixed: boolean): Promise<TicketDto> {
    return invoke('confirm', { ticketId, fixed });
  }
  makeReport(ticketId: string, symptom: string | null): Promise<ReportDto> {
    return invoke('make_report', { ticketId, symptom, utcOffsetMinutes: utcOffsetMinutes() });
  }
  editReport(reportId: number, text: string): Promise<void> {
    return invoke('edit_report', { reportId, text });
  }
  saveReport(reportId: number, text: string): Promise<string> {
    return invoke('save_report', { reportId, text, utcOffsetMinutes: utcOffsetMinutes() });
  }
  history(): Promise<TicketDto[]> {
    return invoke('history');
  }
  settings(): Promise<SettingsDto> {
    return invoke('settings');
  }
  system(): Promise<SystemDto> {
    return invoke('system');
  }
  aiStatus(): Promise<AiStatusDto> {
    return invoke('ai_status');
  }
  aiConnect(key: string): Promise<AiStatusDto> {
    return invoke('ai_connect', { key });
  }
  aiConnectFromClipboard(): Promise<AiStatusDto> {
    return invoke('ai_connect_from_clipboard');
  }
  aiConnectDevice(): Promise<AiStatusDto> {
    return invoke('ai_connect_device');
  }
  aiDisconnect(): Promise<AiStatusDto> {
    return invoke('ai_disconnect');
  }
  aiClassify(text: string, findings: string[] = []): Promise<AiClassificationDto> {
    return invoke('ai_classify', { text, findings });
  }
  aiPreview(text: string): Promise<string> {
    return invoke('ai_preview', { text });
  }
  aiLog(): Promise<AiLogDto[]> {
    return invoke('ai_log');
  }
  openAiConsole(): Promise<void> {
    return invoke('open_ai_console');
  }
  setAutonomy(mode: SettingsDto['autonomy']): Promise<void> {
    return invoke('set_autonomy', { mode });
  }
  setConsent(scope: string, granted: boolean): Promise<void> {
    return invoke('set_consent', { scope, granted });
  }
  exportAll(): Promise<string> {
    return invoke('export_all');
  }
  eraseAll(): Promise<number> {
    return invoke('erase_all');
  }
}
