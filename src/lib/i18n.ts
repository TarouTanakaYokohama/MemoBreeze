import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import enTranslation from "../locales/en/translation.json";
import jaTranslation from "../locales/ja/translation.json";

void i18n
	.use(LanguageDetector)
	.use(initReactI18next)
	.init({
		resources: {
			en: { translation: enTranslation },
			ja: { translation: jaTranslation },
		},
		fallbackLng: "en",
		supportedLngs: ["en", "ja"],
		load: "languageOnly",
		detection: {
			order: ["localStorage", "navigator"],
			caches: ["localStorage"],
		},
		interpolation: {
			escapeValue: false,
		},
		returnNull: false,
	});

export default i18n;
