import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";

const supportedLanguages: Array<{ value: "en" | "ja"; labelKey: string }> = [
	{ value: "en", labelKey: "languageSwitcher.languages.en" },
	{ value: "ja", labelKey: "languageSwitcher.languages.ja" },
];

export const LanguageSwitcher = () => {
	const { i18n, t } = useTranslation();
	const resolved = (i18n.resolvedLanguage ?? i18n.language ?? "en").split(
		"-",
	)[0];
	const currentLanguage: "en" | "ja" = resolved === "ja" ? "ja" : "en";

	const options = useMemo(
		() =>
			supportedLanguages.map((item) => ({
				...item,
				label: t(item.labelKey),
			})),
		[t],
	);

	return (
		<Select
			value={currentLanguage}
			onValueChange={(value) => void i18n.changeLanguage(value)}
		>
			<SelectTrigger
				className="w-[130px]"
				aria-label={t("languageSwitcher.ariaLabel")}
			>
				<SelectValue placeholder={t("languageSwitcher.trigger")} />
			</SelectTrigger>
			<SelectContent>
				{options.map((option) => (
					<SelectItem key={option.value} value={option.value}>
						{option.label}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
};
