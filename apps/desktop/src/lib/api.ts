import { invoke } from '@tauri-apps/api/core';
import type { CommandTrace, SuggestionItem } from '../types';

export type ModelProfile = Record<string, unknown>;
export type ModelRunResult = Record<string, unknown>;
export type BackendAppState = Record<string, unknown>;

export function backendStatus() { return invoke<string>('backend_status'); }
export function getAppState() { return invoke<BackendAppState>('get_app_state'); }
export function inspectProject() { return invoke<Record<string, unknown>>('inspect_project'); }
export function complete(prefix: string) { return invoke<SuggestionItem[]>('complete', { prefix }); }
export function executeShellCommand(command: string, allowMediumRisk = false) { return invoke<CommandTrace>('execute_shell_command', { command, allowMediumRisk }); }
export function listModelProfiles() { return invoke<ModelProfile[]>('list_model_profiles'); }
export function saveModelProfiles(profiles: ModelProfile[]) { return invoke<ModelProfile[]>('save_model_profiles', { profiles }); }
export function createAiCard(profileId: string, intent: string) { return invoke<ModelRunResult>('create_ai_card', { profileId, intent }); }

export function openPty(sessionId: string, cols: number, rows: number) {
  return invoke<void>('terminal_open', { sessionId, cols, rows });
}

export function sendPty(sessionId: string, data: string) {
  return invoke<void>('terminal_' + 'write', { sessionId, data });
}

export function closePty(sessionId: string) {
  return invoke<void>('terminal_close', { sessionId });
}
