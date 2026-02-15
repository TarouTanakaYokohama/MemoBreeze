import * as React from "react";

type Theme = "light" | "dark" | "system";

interface ThemeProviderProps {
	children: React.ReactNode;
	defaultTheme?: Theme;
	storageKey?: string;
}

interface ThemeProviderState {
	theme: Theme;
	setTheme: (theme: Theme) => void;
}

const ThemeProviderContext = React.createContext<
	ThemeProviderState | undefined
>(undefined);

export const ThemeProvider = ({
	children,
	defaultTheme = "system",
	storageKey = "memo-breeze-theme",
}: ThemeProviderProps) => {
	const [theme, setThemeState] = React.useState<Theme>(() => {
		const stored = window.localStorage.getItem(storageKey) as Theme | null;
		return stored ?? defaultTheme;
	});

	const applyTheme = React.useCallback(
		(next: Theme) => {
			const root = window.document.documentElement;
			root.classList.remove("light", "dark");

			if (next === "system") {
				const systemPreference = window.matchMedia(
					"(prefers-color-scheme: dark)",
				).matches
					? "dark"
					: "light";
				root.classList.add(systemPreference);
			} else {
				root.classList.add(next);
			}

			window.localStorage.setItem(storageKey, next);
		},
		[storageKey],
	);

	React.useEffect(() => {
		applyTheme(theme);
	}, [applyTheme, theme]);

	const setTheme = React.useCallback((next: Theme) => {
		setThemeState(next);
	}, []);

	const value = React.useMemo(() => ({ theme, setTheme }), [theme, setTheme]);

	return (
		<ThemeProviderContext.Provider value={value}>
			<div className="min-h-screen bg-background text-foreground">
				{children}
			</div>
		</ThemeProviderContext.Provider>
	);
};

export const useTheme = () => {
	const context = React.use(ThemeProviderContext);
	if (!context) throw new Error("useTheme must be used within ThemeProvider");
	return context;
};
