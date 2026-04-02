import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import zhTranslation from "./locales/zh.json";
import enTranslation from "./locales/en.json";

const resources = {
  zh: {
    translation: zhTranslation
  },
  en: {
    translation: enTranslation
  }
};

// You can save/load language preference from tauri-plugin-store if needed.
// For now, default to device language or 'zh'
const savedLanguage = localStorage.getItem("ais_lang") || "zh";

i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: savedLanguage,
    fallbackLng: "en",
    interpolation: {
      escapeValue: false // react already safes from xss
    }
  });

export default i18n;
