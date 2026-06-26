export type Mode = 'normal' | 'history' | 'ai';
export type AiSubmode = 'suggest' | 'run';
export type ContextLevel = 'off' | 'minimal' | 'project' | 'terminal' | 'selected';
export type CachePolicy = 'off' | 'project_only' | 'full_local';
export type Risk = 'low' | 'medium' | 'high';

export interface AppState {
  mode: Mode;
  aiSubmode: AiSubmode;
  contextLevel: ContextLevel;
  cachePolicy: CachePolicy;
  shell: string;
  cwd: string;
}

export interface SuggestionItem {
  command: string;
  label: string;
  source: string;
  risk: Risk;
  score: number;
}

export interface CommandTrace {
  intent: string;
  cardType: 'command' | 'plan' | 'script' | 'fallback';
  risk: Risk;
  contextUsed: string[];
  commands: string[];
  exitCode?: number;
  durationMs?: number;
  safetyDecision: string;
}
