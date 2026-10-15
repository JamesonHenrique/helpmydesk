// SPDX-License-Identifier: GPL-3.0-or-later
// Shapes of the Rust `hmd-ipc` views (camelCase JSON). Keep in sync with crates/hmd-ipc.

export interface IpcError {
  code: string;
  message: string;
}

export interface TicketDto {
  id: string;
  family: string;
  familyTitle: string;
  state: string;
  reopenCount: number;
  lastTrigger: string | null;
  createdAt: number;
  lastChangedAt: number;
}

export interface StartDto {
  ticket: TicketDto;
  reused: boolean;
  reopened: boolean;
}

export interface ProbeRunDto {
  probeId: string;
  title: string;
  status: 'ok' | 'found' | 'failed' | 'skipped';
  findings: string[];
  reason: string | null;
}

export interface StepDto {
  stepId: string;
  capability: string;
  title: string;
  risk: string;
  undo: string;
  requiresElevation: boolean;
  perActionApproval: boolean;
}

export interface PlanDto {
  planId: number;
  planHash: string;
  steps: StepDto[];
}

export interface DiagnosisDto {
  ticket: TicketDto;
  sessionId: number;
  diagnosisId: string;
  confidence: string;
  confidencePhrase: string;
  title: string;
  explanation: string;
  steps: string[];
  isFallback: boolean;
  trigger: string | null;
  probes: ProbeRunDto[];
  plan: PlanDto | null;
}

export interface ApprovalDto {
  nonce: number;
  planHash: string;
  stepIds: string[];
  expiresAt: number;
}

export interface OutcomeDto {
  kind: 'verified' | 'not_available' | 'not_resolved';
  stepId: string | null;
  trigger: string | null;
}

export interface ReportDto {
  id: number;
  text: string;
}

export interface StartupDto {
  opened: 'created' | 'opened' | 'migrated' | 'recreated';
  interrupted: number;
  housekept: number;
}

export interface SystemDto {
  osName: string | null;
  osVersion: string | null;
  kernelVersion: string | null;
  arch: string | null;
  catalogVersion: string;
}

/** Spec 018: "Ligar a ajuda inteligente". */
export interface AiStatusDto {
  connected: boolean;
  consoleUrl: string;
}

export interface AiClassificationDto {
  family: string | null;
  confidence: 'low' | 'medium' | 'high';
  question: string | null;
}

export interface SettingsDto {
  autonomy: 'observe_only' | 'ask_always' | 'auto_simple';
  consents: string[];
}

/** What the screens can ask the core. One method per Rust command. */
export interface CoreApi {
  /** `true` inside the desktop app; `false` in the browser prototype (example data). */
  readonly real: boolean;
  startup(): Promise<StartupDto>;
  startTicket(family: string): Promise<StartDto>;
  diagnose(ticketId: string): Promise<DiagnosisDto>;
  /** Desktop: opens the separate approval window. Prototype: approves in place. */
  requestApproval(
    ticketId: string,
    plan: PlanDto,
    stepIds: string[],
  ): Promise<'approved' | 'declined'>;
  plan(ticketId: string, planId: number): Promise<PlanDto>;
  approve(ticketId: string, planId: number, stepIds: string[]): Promise<ApprovalDto>;
  decline(ticketId: string, planId: number): Promise<void>;
  execute(ticketId: string, planId: number): Promise<OutcomeDto>;
  confirm(ticketId: string, fixed: boolean): Promise<TicketDto>;
  makeReport(ticketId: string, symptom: string | null): Promise<ReportDto>;
  editReport(reportId: number, text: string): Promise<void>;
  /** Desktop: saves into the Downloads folder and returns the path. Prototype: browser download. */
  saveReport(reportId: number, text: string): Promise<string>;
  history(): Promise<TicketDto[]>;
  settings(): Promise<SettingsDto>;
  system(): Promise<SystemDto>;
  aiStatus(): Promise<AiStatusDto>;
  /** Tests the key with the service and keeps it in the system vault only if it works. */
  aiConnect(key: string): Promise<AiStatusDto>;
  aiDisconnect(): Promise<AiStatusDto>;
  /** Desktop: reads the key from the clipboard in the core (it never reaches the screen). */
  aiConnectFromClipboard(): Promise<AiStatusDto>;
  aiClassify(text: string): Promise<AiClassificationDto>;
  /** Opens the OpenCode console (fixed URL, chosen by the core). */
  openAiConsole(): Promise<void>;
  setAutonomy(mode: SettingsDto['autonomy']): Promise<void>;
  setConsent(scope: string, granted: boolean): Promise<void>;
  exportAll(): Promise<string>;
  eraseAll(): Promise<number>;
}
