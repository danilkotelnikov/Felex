import { create } from 'zustand';
import { agentApi } from '@/lib/api';

const SUPPORTED_MODELS = ['qwen3.5:4b', 'qwen3.5:9b'] as const;
const SUPPORTED_CONTEXT_SIZES = [1024, 2048, 4096, 8192, 16384] as const;
const DEFAULT_MODEL = 'qwen3.5:4b';
const DEFAULT_CONTEXT_SIZE = 8192;

type SupportedModel = typeof SUPPORTED_MODELS[number];
type SupportedContextSize = typeof SUPPORTED_CONTEXT_SIZES[number];

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

interface AgentStatus {
  modelLoaded: boolean;
  modelName: string;
  backend: string;
  webEnabled: boolean;
  contextSize: SupportedContextSize;
}

interface AgentSettings {
  model: SupportedModel;
  backend: 'ollama';
  webSearch: boolean;
  autoSuggest: boolean;
  contextSize: SupportedContextSize;
}

interface ReloadAgentOptions {
  model?: SupportedModel;
  backend?: 'ollama';
  webEnabled?: boolean;
  contextSize?: SupportedContextSize;
}

interface AgentStore {
  messages: Message[];
  status: AgentStatus;
  isStreaming: boolean;
  statusLoading: boolean;
  settings: AgentSettings;
  addMessage: (role: 'user' | 'assistant', content: string) => void;
  updateLastMessage: (content: string) => void;
  clearMessages: () => void;
  setStatus: (status: Partial<AgentStatus>) => void;
  setStreaming: (streaming: boolean) => void;
  setSettings: (settings: Partial<AgentSettings>) => void;
  fetchStatus: () => Promise<void>;
  reloadAgent: (options?: ReloadAgentOptions) => Promise<void>;
  switchModel: (model: SupportedModel) => Promise<void>;
}

let messageCounter = 0;

function normalizeModel(model: string | undefined): SupportedModel {
  return SUPPORTED_MODELS.includes(model as SupportedModel)
    ? (model as SupportedModel)
    : DEFAULT_MODEL;
}

function normalizeContextSize(value: number | string | undefined): SupportedContextSize {
  const numeric = typeof value === 'string' ? Number.parseInt(value, 10) : value;
  return SUPPORTED_CONTEXT_SIZES.includes(numeric as SupportedContextSize)
    ? (numeric as SupportedContextSize)
    : DEFAULT_CONTEXT_SIZE;
}

function recommendedContextSize(model: SupportedModel): SupportedContextSize {
  return model === 'qwen3.5:9b' ? 2048 : 8192;
}

function loadSettings(): AgentSettings {
  try {
    const stored = localStorage.getItem('felex_agent_settings');
    if (stored) {
      const parsed = JSON.parse(stored) as Partial<AgentSettings>;
      const model = normalizeModel(parsed.model);
      return {
        model,
        backend: 'ollama',
        webSearch: parsed.webSearch ?? true,
        autoSuggest: parsed.autoSuggest ?? false,
        contextSize:
          parsed.contextSize === undefined
            ? recommendedContextSize(model)
            : normalizeContextSize(parsed.contextSize),
      };
    }
  } catch {
    // Ignore malformed local storage.
  }

  return {
    model: DEFAULT_MODEL,
    backend: 'ollama',
    webSearch: true,
    autoSuggest: false,
    contextSize: recommendedContextSize(DEFAULT_MODEL),
  };
}

function toAgentStatus(apiStatus: {
  model_loaded: boolean;
  model_name: string;
  backend: string;
  web_enabled: boolean;
  context_size: number;
}): AgentStatus {
  return {
    modelLoaded: apiStatus.model_loaded,
    modelName: apiStatus.model_name,
    backend: apiStatus.backend,
    webEnabled: apiStatus.web_enabled,
    contextSize: normalizeContextSize(apiStatus.context_size),
  };
}

export const useAgentStore = create<AgentStore>((set, get) => ({
  messages: [],
  status: {
    modelLoaded: false,
    modelName: '',
    backend: 'unknown',
    webEnabled: false,
    contextSize: DEFAULT_CONTEXT_SIZE,
  },
  isStreaming: false,
  statusLoading: true,
  settings: loadSettings(),

  addMessage: (role, content) =>
    set((state) => ({
      messages: [
        ...state.messages,
        {
          id: `msg-${++messageCounter}`,
          role,
          content,
          timestamp: new Date(),
        },
      ],
    })),

  updateLastMessage: (content) =>
    set((state) => {
      const messages = [...state.messages];
      if (messages.length > 0) {
        messages[messages.length - 1] = {
          ...messages[messages.length - 1],
          content,
        };
      }
      return { messages };
    }),

  clearMessages: () => set({ messages: [] }),

  setStatus: (status) =>
    set((state) => ({
      status: { ...state.status, ...status },
    })),

  setStreaming: (streaming) => set({ isStreaming: streaming }),

  setSettings: (partial) => {
    const current = get().settings;
    const nextModel = normalizeModel(partial.model ?? current.model);
    const requestedContext = partial.contextSize ?? current.contextSize;
    const normalizedContext = normalizeContextSize(requestedContext);
    const recommended = recommendedContextSize(nextModel);
    const shouldClampForModel =
      partial.model !== undefined && partial.contextSize === undefined && normalizedContext > recommended;
    const updated: AgentSettings = {
      ...current,
      ...partial,
      model: nextModel,
      backend: 'ollama',
      contextSize: shouldClampForModel ? recommended : normalizedContext,
    };
    localStorage.setItem('felex_agent_settings', JSON.stringify(updated));
    set({ settings: updated });
  },

  fetchStatus: async () => {
    set({ statusLoading: true });

    const MAX_RETRIES = 5;
    const RETRY_DELAY = 3000;

    for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
      try {
        const apiStatus = await agentApi.getStatus();
        const desired = get().settings;

        if (
          apiStatus.model_name !== desired.model ||
          apiStatus.backend !== desired.backend ||
          apiStatus.web_enabled !== desired.webSearch ||
          normalizeContextSize(apiStatus.context_size) !== desired.contextSize
        ) {
          await get().reloadAgent({
            model: desired.model,
            backend: desired.backend,
            webEnabled: desired.webSearch,
            contextSize: desired.contextSize,
          });
          return;
        }

        set({
          status: toAgentStatus(apiStatus),
          statusLoading: false,
        });
        return;
      } catch {
        if (attempt < MAX_RETRIES - 1) {
          await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY));
        }
      }
    }

    try {
      const apiStatus = await agentApi.getStatus();
      set({
        status: toAgentStatus(apiStatus),
        statusLoading: false,
      });
    } catch {
      set({
        status: {
          modelLoaded: false,
          modelName: '',
          backend: 'unavailable',
          webEnabled: false,
          contextSize: DEFAULT_CONTEXT_SIZE,
        },
        statusLoading: false,
      });
    }
  },

  reloadAgent: async (options) => {
    set({ statusLoading: true });
    try {
      const settings = get().settings;
      const payload = {
        model: normalizeModel(options?.model ?? settings.model),
        backend: 'ollama' as const,
        web_enabled: options?.webEnabled ?? settings.webSearch,
        context_size: normalizeContextSize(options?.contextSize ?? settings.contextSize),
      };

      const apiStatus = await agentApi.reload(payload);

      set({
        status: toAgentStatus(apiStatus),
        statusLoading: false,
      });
    } catch (error) {
      console.error('Failed to reload agent:', error);
      set({ statusLoading: false });
    }
  },

  switchModel: async (model) => {
    const { setSettings, reloadAgent, settings, status, statusLoading } = get();

    if (statusLoading) {
      return;
    }

    const alreadyActive =
      model === settings.model &&
      model === status.modelName &&
      status.modelLoaded;
    if (alreadyActive) {
      return;
    }

    setSettings({ model, backend: 'ollama' });
    const nextSettings = get().settings;
    await reloadAgent({
      model: nextSettings.model,
      backend: 'ollama',
      webEnabled: nextSettings.webSearch,
      contextSize: nextSettings.contextSize,
    });
  },
}));

if (typeof window !== 'undefined') {
  useAgentStore.getState().fetchStatus();
}
