import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { feedsApi, type ListFeedsParams } from '@/lib/api';
import type { Feed } from '@/types/feed';

/** Query key factory */
const feedKeys = {
  all: ['feeds'] as const,
  lists: () => [...feedKeys.all, 'list'] as const,
  list: (params: ListFeedsParams) => [...feedKeys.lists(), params] as const,
  details: () => [...feedKeys.all, 'detail'] as const,
  detail: (id: number) => [...feedKeys.details(), id] as const,
};

/** Hook to list feeds */
export function useFeeds(params: ListFeedsParams = {}) {
  return useQuery({
    queryKey: feedKeys.list(params),
    queryFn: () => feedsApi.list(params),
  });
}

/** Hook to get single feed */
export function useFeed(id: number) {
  return useQuery({
    queryKey: feedKeys.detail(id),
    queryFn: () => feedsApi.get(id),
    enabled: id > 0,
  });
}

/** Hook to create feed */
export function useCreateFeed() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (feed: Partial<Feed>) => feedsApi.create(feed),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: feedKeys.lists() });
    },
  });
}

/** Hook to update feed */
export function useUpdateFeed() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, ...feed }: Partial<Feed> & { id: number }) =>
      feedsApi.update(id, feed),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: feedKeys.detail(variables.id) });
      queryClient.invalidateQueries({ queryKey: feedKeys.lists() });
    },
  });
}

/** Hook to delete feed */
export function useDeleteFeed() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => feedsApi.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: feedKeys.lists() });
    },
  });
}

/** Hook to import the local feed catalog through the compatibility endpoint */
export function useImportCatalog() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => feedsApi.importCatalog(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: feedKeys.all });
    },
  });
}

export const useImportCapRu = useImportCatalog;
