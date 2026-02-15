import React from "react";
import ReactDOM from "react-dom/client";
import { ErrorBoundary } from "react-error-boundary";
import App from "./App";
import { ThemeProvider } from "./components/theme-provider";
import "./styles.css";
import "./lib/i18n";

function Fallback({
	error,
	resetErrorBoundary,
}: {
	error: Error;
	resetErrorBoundary: () => void;
}) {
	return (
		<div className="flex min-h-screen flex-col items-center justify-center gap-4 p-6 text-center">
			<h2 className="text-lg font-semibold">Something went wrong</h2>
			<pre className="max-w-full overflow-auto rounded bg-muted p-4 text-left text-sm">
				{error.message}
			</pre>
			<button
				type="button"
				onClick={resetErrorBoundary}
				className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
			>
				Try again
			</button>
		</div>
	);
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<ErrorBoundary
			fallbackRender={({ error, resetErrorBoundary }) => (
				<Fallback error={error} resetErrorBoundary={resetErrorBoundary} />
			)}
			onError={(error) => {
				console.error("App error boundary:", error);
			}}
		>
			<ThemeProvider>
				<App />
			</ThemeProvider>
		</ErrorBoundary>
	</React.StrictMode>,
);
