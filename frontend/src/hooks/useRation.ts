import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { rationsApi } from '@/lib/api';
import type { OptimizeMode, Ration } from '@/types/ration';
import type { NormRange } from '@/types/nutrient';

/** Query key factory */
const rationKeys = {
  all: ['rations'] as const,
  lists: () => [...rationKeys.all, 'list'] as const,
  details: () => [...rationKeys.all, 'detail'] as const,
  detail: (id: number) => [...rationKeys.details(), id] as const,
  nutrients: (id: number) => [...rationKeys.detail(id), 'nutrients'] as const,
  economics: (id: number) => [...rationKeys.detail(id), 'economics'] as const,
};

/** Hook to list rations */
export function useRations() {
  return useQuery({
    queryKey: rationKeys.lists(),
    queryFn: () => rationsApi.list(),
  });
}

/** Hook to get single ration with items */
export function useRation(id: number) {
  return useQuery({
    queryKey: rationKeys.detail(id),
    queryFn: () => rationsApi.get(id),
    enabled: id > 0,
  });
}

/** Hook to get ration nutrients */
export function useRationNutrients(id: number) {
  return useQuery({
    queryKey: rationKeys.nutrients(id),
    queryFn: () => rationsApi.getNutrients(id),
    enabled: id > 0,
  });
}

/** Hook to get ration economics */
export function useRationEconomics(id: number) {
  return useQuery({
    queryKey: rationKeys.economics(id),
    queryFn: () => rationsApi.getEconomics(id),
    enabled: id > 0,
  });
}

/** Hook to create ration */
export function useCreateRation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (ration: Partial<Ration>) => rationsApi.create(ration),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: rationKeys.lists() });
    },
  });
}

/** Hook to update ration */
export function useUpdateRation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      id,
      ...data
    }: {
      id: number;
      items?: { feed_id: number; amount_kg: number; is_locked?: boolean }[];
    } & Partial<Ration>) => rationsApi.update(id, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: rationKeys.detail(variables.id) });
      queryClient.invalidateQueries({ queryKey: rationKeys.nutrients(variables.id) });
      queryClient.invalidateQueries({ queryKey: rationKeys.economics(variables.id) });
    },
  });
}

/** Hook to optimize ration */
export function useOptimizeRation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      id,
      mode,
      norms,
    }: {
      id: number;
      mode?: OptimizeMode;
      norms?: Record<string, NormRange>;
    }) => rationsApi.optimize(id, { mode, norms }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: rationKeys.detail(variables.id) });
      queryClient.invalidateQueries({ queryKey: rationKeys.nutrients(variables.id) });
      queryClient.invalidateQueries({ queryKey: rationKeys.economics(variables.id) });
    },
  });
}
