import type { ModelProfile } from './api';

const home = 'C:/Users/Amaan';
const modelRoot = `${home}/Downloads/aish-model/models`;
const llamaCli = `${home}/Downloads/llama.cpp/build/bin/Release/llama-cli.exe`;

function profile(id: string, label: string, family: string, file: string, contextTokens = 32768): ModelProfile {
  return {
    id,
    label,
    family,
    model_path: `${modelRoot}/${file}`,
    llama_cli_path: llamaCli,
    context_tokens: contextTokens,
    max_tokens: 512,
    temperature: 0.1,
  };
}

export const DEFAULT_MODEL_PROFILES: ModelProfile[] = [
  profile('qwen25-coder-15b-q4-k-m', 'Qwen2.5 Coder 1.5B Instruct Q4_K_M', 'qwen2.5-coder', 'Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf'),
];
