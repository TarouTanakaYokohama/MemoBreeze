import { Moon, Sun } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useTheme } from "./theme-provider";
import { Button } from "./ui/button";

const order: Array<"light" | "dark" | "system"> = ["light", "dark", "system"];

export const ModeToggle = () => {
	const { theme, setTheme } = useTheme();
	const currentIndex = order.indexOf(theme);
	const nextTheme = order[(currentIndex + 1) % order.length];
	const { t } = useTranslation();

	return (
		<Button
			variant="ghost"
			size="icon"
			aria-label={t("accessibility.toggleTheme")}
			onClick={() => setTheme(nextTheme)}
		>
			<Sun className="h-4 w-4 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
			<Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
			<span className="sr-only">{t("accessibility.toggleTheme")}</span>
		</Button>
	);
};
