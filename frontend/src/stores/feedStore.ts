import { create } from 'zustand';

interface FeedStore {
  // Search and filters
  searchQuery: string;
  selectedCategory: string | null;

  // Selected feed for details
  selectedFeedId: number | null;

  // Panel state
  isPanelCollapsed: boolean;

  // Actions
  setSearchQuery: (query: string) => void;
  setSelectedCategory: (category: string | null) => void;
  setSelectedFeedId: (id: number | null) => void;
  togglePanel: () => void;
}

export const useFeedStore = create<FeedStore>((set) => ({
  searchQuery: '',
  selectedCategory: null,
  selectedFeedId: null,
  isPanelCollapsed: false,

  setSearchQuery: (query) => set({ searchQuery: query }),
  setSelectedCategory: (category) => set({ selectedCategory: category }),
  setSelectedFeedId: (id) => set({ selectedFeedId: id }),
  togglePanel: () => set((state) => ({ isPanelCollapsed: !state.isPanelCollapsed })),
}));
