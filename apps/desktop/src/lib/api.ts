import { invoke } from '@tauri-apps/api/core';
import type { CommandTrace, SuggestionItem } from '../types';

export type ModelProfile = Record<string, unknown>;
export type ModelRunResult = Record<string, unknown>;
export type BackendAppState = Record<string, unknown>;
export type CommandLogPolicy = 'off' | 'failed_only' | 'all';

export interface LogSettings {
  command_log_policy: CommandLogPolicy;
  crash_log_sharing_opt_in: boolean;
}

export interface CommandLogEntry {
  intent?: string;
  command?: string;
  status: string;
  risk?: string;
  reason?: string;
  error?: string;
  surface?: string;
}

export function backendStatus() { return invoke<string>('backend_status'); }
export function getAppState() { return invoke<BackendAppState>('get_app_state'); }
export function inspectProject() { return invoke<Record<string, unknown>>('inspect_project'); }
export function complete(prefix: string) { return invoke<SuggestionItem[]>('complete', { prefix }); }
export function checkCommandRisk(command: string) { return invoke<{ risk: string; needs_confirmation: boolean; reason: string }>('check_command_risk', { command }); }
export function executeShellCommand(command: string, allowMediumRisk = false) { return invoke<CommandTrace>('execute_shell_command', { command, allowMediumRisk }); }
export function listModelProfiles() { return invoke<ModelProfile[]>('list_model_profiles'); }
export function saveModelProfiles(profiles: ModelProfile[]) { return invoke<ModelProfile[]>('save_model_profiles', { profiles }); }
export function createAiCard(profileId: string, intent: string) { return invoke<ModelRunResult>('create_ai_card', { profileId, intent }); }
export function getLogSettings() { return invoke<LogSettings>('get_log_settings'); }
export function saveLogSettings(settings: LogSettings) { return invoke<LogSettings>('save_log_settings', { settings }); }
export function recordCommandLog(entry: CommandLogEntry) { return invoke<void>('record_command_log', { entry }); }

export function openPty(sessionId: string, cols: number, rows: number) {
  return invoke<void>('terminal_open', { sessionId, cols, rows, data: null, action: null });
}

export function sendPty(sessionId: string, data: string) {
  return invoke<void>('terminal_open', { sessionId, cols: 80, rows: 24, data, action: 'data' });
}

export function resizePty(sessionId: string, cols: number, rows: number) {
  return invoke<void>('terminal_open', { sessionId, cols, rows, data: null, action: 'resize' });
}

export function closePty(sessionId: string) {
  return invoke<void>('terminal_open', { sessionId, cols: 80, rows: 24, data: null, action: 'close' });
}
