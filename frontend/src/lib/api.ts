/** API client for Felex backend */

// In Tauri production build, frontend is served via tauri:// protocol.
// There is no Vite proxy there, so requests must go to the embedded Axum server.
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
const API_BASE = isTauri ? 'http://localhost:7432/api/v1' : '/api/v1';

/** Generic API response */
interface ApiResponse<T> {
  data: T;
}

/** API error */
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/** Make API request */
async function request<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE}${path}`;

  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const raw = await response.text();
    let error: { error?: string; message?: string } = {};

    if (raw) {
      try {
        error = JSON.parse(raw);
      } catch {
        error = { message: raw };
      }
    }

    throw new ApiError(
      response.status,
      error.error || 'unknown',
      error.message || response.statusText
    );
  }

  if (response.status === 204) {
    return {} as T;
  }

  const raw = await response.text();
  if (!raw.trim()) {
    return {} as T;
  }

  try {
    return JSON.parse(raw) as T;
  } catch {
    return raw as T;
  }
}

// ============ Feeds API ============

export interface ListFeedsParams {
  category?: string;
  search?: string;
  limit?: number;
  offset?: number;
  species?: string;
  stageContext?: string;
}

export interface PriceAnchorSource {
  kind: string;
  label: string;
  count: number;
}

export interface PriceProvenance {
  kind: string;
  is_precise_source: boolean;
  source_url?: string;
  source_domain?: string;
  benchmark_level?: string;
  anchor_count?: number;
  anchor_sources?: PriceAnchorSource[];
}

const importCatalog = () =>
  request<ApiResponse<{ imported: number; errors: number; total: number }>>('/feeds/import/capru', {
    method: 'POST',
  });

export const feedsApi = {
  list: (params: ListFeedsParams = {}) => {
    const searchParams = new URLSearchParams();
    if (params.category) searchParams.set('category', params.category);
    if (params.search) searchParams.set('search', params.search);
    if (params.limit) searchParams.set('limit', params.limit.toString());
    if (params.offset) searchParams.set('offset', params.offset.toString());
    if (params.species) searchParams.set('species', params.species);
    if (params.stageContext) searchParams.set('stage_context', params.stageContext);

    const query = searchParams.toString();
    return request<{ data: Feed[]; total: number }>(`/feeds${query ? `?${query}` : ''}`);
  },

  get: (
    id: number,
    context: { species?: string; stageContext?: string } = {},
  ) => {
    const searchParams = new URLSearchParams();
    if (context.species) searchParams.set('species', context.species);
    if (context.stageContext) searchParams.set('stage_context', context.stageContext);
    const query = searchParams.toString();
    return request<ApiResponse<Feed>>(`/feeds/${id}${query ? `?${query}` : ''}`);
  },

  create: (feed: Partial<Feed>) =>
    request<ApiResponse<number>>('/feeds', {
      method: 'POST',
      body: JSON.stringify(feed),
    }),

  update: (id: number, feed: Partial<Feed>) =>
    request<void>(`/feeds/${id}`, {
      method: 'PUT',
      body: JSON.stringify(feed),
    }),

  delete: (id: number) =>
    request<void>(`/feeds/${id}`, { method: 'DELETE' }),

  fetchFeedPrice: (id: number, locale: string = 'RU') =>
    request<ApiResponse<{
      feed_id: number;
      price: number;
      currency: string;
      source_url?: string;
      confidence_score?: number | null;
      locale: string;
      price_date?: string;
      source?: string;
      region?: string;
      provenance?: PriceProvenance;
    }>>(`/feeds/${id}/price?locale=${locale}`),

  importCatalog,
  importCapRu: importCatalog,

  sync: () =>
    request<ApiResponse<{
      feeds_imported: number;
      feeds_errors: number;
      feeds_total: number;
      prices_updated: number;
      prices_total: number;
      seed_imported: number;
    }>>('/feeds/sync', {
      method: 'POST',
    }),
};

// ============ Rations API ============

import type { Feed } from '@/types/feed';
import type {
  AutoPopulatePlan,
  DietSolution,
  EconomicAnalysis,
  NutrientSummary,
  OptimizeRationRequest,
  Ration,
  RationFull,
  ScreeningReport,
} from '@/types/ration';
import type { OptimizationResult } from '@/types/optimization';

export interface PresetAnimalParams {
  live_weight_kg?: number;
  daily_gain_g?: number;
  milk_yield_kg?: number;
  age_days?: number;
  age_weeks?: number;
  target_weight_g?: number;
  production_pct?: number;
  days_pregnant?: number;
  days_to_calving?: number;
  piglets?: number;
  lactation_stage?: string;
}

export interface MatchReasonParts {
  name_fragment?: string;
  category?: string;
  nutrient_key?: string;
  nutrient_value?: number;
  verified: boolean;
}

export interface PresetFeedMatch {
  feed: Feed;
  match_score: number;
  match_reason: MatchReasonParts;
}

export interface PresetRecommendationMatches {
  key: string;
  label_ru: string;
  label_en: string;
  matches: PresetFeedMatch[];
}

export interface MatchedPresetSubcategory {
  id: string;
  name_ru: string;
  name_en: string;
  animal_group_id: string;
  norm_preset_id?: string | null;
  legacy_preset_id?: string | null;
  params: PresetAnimalParams;
  research_source?: string | null;
  recommendations: PresetRecommendationMatches[];
  matched_feed_count: number;
  fully_matched: boolean;
}

export interface MatchedPresetCategory {
  species: string;
  production_type: string;
  subcategories: MatchedPresetSubcategory[];
}

export interface PresetCatalogResponse {
  categories: MatchedPresetCategory[];
}

export const rationsApi = {
  list: () => request<ApiResponse<Ration[]>>('/rations'),

  get: (id: number) => request<ApiResponse<RationFull>>(`/rations/${id}`),

  create: (ration: Partial<Ration>) =>
    request<ApiResponse<number>>('/rations', {
      method: 'POST',
      body: JSON.stringify(ration),
    }),

  update: (id: number, ration: Partial<Ration> & { items?: { feed_id: number; amount_kg: number; is_locked?: boolean }[] }) =>
    request<void>(`/rations/${id}`, {
      method: 'PUT',
      body: JSON.stringify(ration),
    }),

  optimize: (id: number, payload: OptimizeRationRequest = { mode: 'tiered' }) =>
    request<ApiResponse<DietSolution>>(`/rations/${id}/optimize`, {
      method: 'POST',
      body: JSON.stringify(payload),
    }),

  optimizeAlternatives: (id: number, payload: OptimizeRationRequest = { mode: 'tiered' }) =>
    request<ApiResponse<OptimizationResult>>(`/rations/${id}/alternatives`, {
      method: 'POST',
      body: JSON.stringify(payload),
    }),

  autoPopulate: (id: number, payload: OptimizeRationRequest = {}) =>
    request<ApiResponse<AutoPopulatePlan>>(`/rations/${id}/auto-populate`, {
      method: 'POST',
      body: JSON.stringify(payload),
    }),

  screen: (id: number, payload: OptimizeRationRequest = {}) =>
    request<ApiResponse<ScreeningReport>>(`/rations/${id}/screen`, {
      method: 'POST',
      body: JSON.stringify(payload),
    }),

  getNutrients: (id: number) =>
    request<ApiResponse<NutrientSummary>>(`/rations/${id}/nutrients`),

  getEconomics: (id: number) =>
    request<ApiResponse<EconomicAnalysis>>(`/rations/${id}/economics`),
};

export const presetsApi = {
  list: () => request<ApiResponse<PresetCatalogResponse>>('/presets'),
};

// ============ Animals API ============

import type { AnimalNorm, NormMethodology } from '@/types/nutrient';

export interface AnimalGroup {
  id: string;
  species: string;
  production_type?: string;
  name_ru: string;
  name_en?: string;
  description?: string;
}

export const animalsApi = {
  list: (species?: string) => {
    const query = species ? `?species=${species}` : '';
    return request<ApiResponse<AnimalGroup[]>>(`/animals${query}`);
  },

  get: (id: string) => request<ApiResponse<AnimalGroup>>(`/animals/${id}`),

  getNorms: (groupId: string) =>
    request<ApiResponse<AnimalNorm>>(`/norms/${groupId}`),

  resolveNorms: (
    groupId: string,
    payload: Pick<OptimizeRationRequest, 'norm_preset_id' | 'animal_properties'> = {},
  ) =>
    request<ApiResponse<{ resolved_group_id: string; norm: AnimalNorm; methodology?: NormMethodology | null }>>(
      `/norms/${groupId}/resolve`,
      {
        method: 'POST',
        body: JSON.stringify(payload),
      },
    ),
};

// ============ Prices API ============

export interface FeedPrice {
  id: number;
  feed_id: number;
  region?: string;
  price_rubles_per_ton: number;
  price_date?: string;
  source?: string;
  notes?: string;
  provenance: PriceProvenance;
}

export const pricesApi = {
  list: (region?: string) => {
    const query = region ? `?region=${region}` : '';
    return request<ApiResponse<FeedPrice[]>>(`/prices${query}`);
  },

  update: (feedId: number, price: number, region?: string) =>
    request<void>(`/prices/${feedId}`, {
      method: 'PUT',
      body: JSON.stringify({
        price_rubles_per_ton: price,
        region,
        source: 'manual',
      }),
    }),

  fetch: () =>
    request<ApiResponse<{ updated: number; errors: number }>>('/prices/fetch', {
      method: 'POST',
    }),

  getHistory: (feedId: number) =>
    request<ApiResponse<{ date: string; price: number; source: string }[]>>(`/prices/${feedId}/history`),
};

// ============ App API ============

export interface AppMeta {
  version: string;
  database_path: string;
  workspace_root: string;
  feed_count: number;
  last_sync_at?: string | null;
  catalog_quality: {
    source_counts: {
      normalized: number;
      curated: number;
      custom: number;
      imported: number;
    };
    translation_counts: {
      ready: number;
      source_only: number;
    };
    profile_counts: {
      complete: number;
      partial: number;
      limited: number;
    };
    priced_feed_count: number;
    unpriced_feed_count: number;
    benchmark_critical_contexts: {
      id: string;
      species: string;
      stage_context: string;
      audited_feed_count: number;
      coverage_counts: {
        complete: number;
        partial: number;
        limited: number;
      };
      top_missing_keys: {
        key: import('@/types/feed').FeedCriticalNutrientKey;
        count: number;
      }[];
    }[];
  };
}

export const appApi = {
  getMeta: () => request<ApiResponse<AppMeta>>('/app/meta'),
};

// ============ Agent API ============

export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface AgentStatus {
  model_loaded: boolean;
  model_name: string;
  backend: string;
  web_enabled: boolean;
  context_size: number;
}

export interface ChatContext {
  animal_type?: string;
  production_level?: string;
  current_ration?: string;
  nutrient_status?: string;
}

export interface ChatRequest {
  messages: ChatMessage[];
  stream?: boolean;
  context?: ChatContext;
}

export interface ChatResponse {
  message: string;
  done: boolean;
}

export interface ChatStreamChunk {
  content: string;
  done: boolean;
  error?: string;
}

export const agentApi = {
  getStatus: () => request<AgentStatus>('/agent/status'),

  chat: (req: ChatRequest) =>
    request<ChatResponse>('/agent/chat', {
      method: 'POST',
      body: JSON.stringify(req),
    }),

  chatStream: async function* (
    req: ChatRequest,
    signal?: AbortSignal
  ): AsyncGenerator<ChatStreamChunk> {
    const url = `${API_BASE}/agent/chat/stream`;

    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
      signal,
    });

    if (!response.ok) {
      const raw = await response.text();
      let error: { error?: string; message?: string } = {};

      if (raw) {
        try {
          error = JSON.parse(raw);
        } catch {
          error = { message: raw };
        }
      }

      throw new ApiError(
        response.status,
        error.error || 'unknown',
        error.message || response.statusText
      );
    }

    const reader = response.body?.getReader();
    if (!reader) throw new Error('No response body');

    const decoder = new TextDecoder();
    let buffer = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim();
          if (data) {
            try {
              const chunk: ChatStreamChunk = JSON.parse(data);
              yield chunk;
              if (chunk.done) return;
            } catch {
              // Ignore parse errors.
            }
          }
        }
      }
    }
  },

  reload: (config?: { model?: string; backend?: string; web_enabled?: boolean; context_size?: number }) =>
    request<AgentStatus>('/agent/reload', {
      method: 'POST',
      body: config ? JSON.stringify(config) : undefined,
    }),
};
