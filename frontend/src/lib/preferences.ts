export interface NotificationPreferences {
  nutrientWarnings: boolean;
  priceSync: boolean;
  exportReady: boolean;
  agentStatus: boolean;
}

export const DEFAULT_NOTIFICATION_PREFERENCES: NotificationPreferences = {
  nutrientWarnings: true,
  priceSync: true,
  exportReady: true,
  agentStatus: true,
};

const NOTIFICATION_PREFERENCES_KEY = 'felex_notification_preferences';

export function loadNotificationPreferences(): NotificationPreferences {
  if (typeof window === 'undefined') {
    return DEFAULT_NOTIFICATION_PREFERENCES;
  }

  try {
    const raw = localStorage.getItem(NOTIFICATION_PREFERENCES_KEY);
    if (!raw) {
      return DEFAULT_NOTIFICATION_PREFERENCES;
    }

    const parsed = JSON.parse(raw) as Partial<NotificationPreferences>;
    return {
      ...DEFAULT_NOTIFICATION_PREFERENCES,
      ...parsed,
    };
  } catch {
    return DEFAULT_NOTIFICATION_PREFERENCES;
  }
}

export function saveNotificationPreferences(preferences: NotificationPreferences): void {
  if (typeof window === 'undefined') {
    return;
  }

  localStorage.setItem(NOTIFICATION_PREFERENCES_KEY, JSON.stringify(preferences));
}

export function isNotificationEnabled(key: keyof NotificationPreferences): boolean {
  return loadNotificationPreferences()[key];
}
