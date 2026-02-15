import type { RenderOptions } from "@testing-library/react";
import { render } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import { I18nextProvider } from "react-i18next";
import { ThemeProvider } from "@/components/theme-provider";
import i18n from "@/lib/i18n";

function AllTheProviders({ children }: { children: ReactNode }) {
	return (
		<I18nextProvider i18n={i18n}>
			<ThemeProvider>{children}</ThemeProvider>
		</I18nextProvider>
	);
}

function customRender(
	ui: ReactElement,
	options?: Omit<RenderOptions, "wrapper">,
) {
	return render(ui, {
		wrapper: AllTheProviders,
		...options,
	});
}

export * from "@testing-library/react";
export { customRender as render };
