import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import ru from './locales/ru.json';
import en from './locales/en.json';

const resources = {
  ru: { translation: ru },
  en: { translation: en },
};

// Get saved language or default to Russian
const savedLanguage = localStorage.getItem('felex-language') || 'ru';

i18n.use(initReactI18next).init({
  resources,
  lng: savedLanguage,
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false,
  },
});

// Save language preference when changed
i18n.on('languageChanged', (lng) => {
  localStorage.setItem('felex-language', lng);
});

export default i18n;
