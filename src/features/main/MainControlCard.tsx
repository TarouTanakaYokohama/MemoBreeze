import { save } from "@tauri-apps/plugin-dialog";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardFooter,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import {
	exportMinutes,
	generateMinutes,
	startSession,
	stopSession,
} from "../../lib/api";
import {
	defaultMinutesOptions,
	useTranscriptStore,
} from "../../stores/transcript";
import { MinutesOptionsForm } from "./MinutesOptionsForm";

const START_RECORDING_TIMEOUT_MS = 10_000;

const withTimeout = async <T,>(
	promise: Promise<T>,
	timeoutMs: number,
	timeoutMessage: string,
) =>
	new Promise<T>((resolve, reject) => {
		const timer = window.setTimeout(() => {
			reject(new Error(timeoutMessage));
		}, timeoutMs);

		promise.then(
			(value) => {
				window.clearTimeout(timer);
				resolve(value);
			},
			(error) => {
				window.clearTimeout(timer);
				reject(error);
			},
		);
	});

export function MainControlCard() {
	const {
		isRecording,
		segmentsCount,
		hasMinutes,
		error,
		minutesOptions,
		setMinutesOptions,
	} = useTranscriptStore(
		useShallow((state) => ({
			isRecording: state.isRecording,
			segmentsCount: state.segments.length,
			hasMinutes: !!state.minutes,
			error: state.lastError,
			minutesOptions: state.minutesOptions ?? defaultMinutesOptions,
			setMinutesOptions: state.setMinutesOptions,
		})),
	);
	const { t } = useTranslation();

	const [isGenerating, setIsGenerating] = useState(false);
	const [isTogglingRecording, setIsTogglingRecording] = useState(false);
	const [minutesError, setMinutesError] = useState<string>();
	const hasClearableData = segmentsCount > 0 || hasMinutes;

	const handleToggleRecording = useCallback(async () => {
		if (isTogglingRecording) return;
		const {
			isRecording,
			recordingOptions,
			setRecording,
			setSessionId,
			setError,
		} = useTranscriptStore.getState();

		setIsTogglingRecording(true);
		if (isRecording) {
			setError(undefined);
			try {
				await stopSession();
				setRecording(false);
				setSessionId(undefined);
			} catch (error) {
				setError(error instanceof Error ? error.message : String(error));
			} finally {
				setIsTogglingRecording(false);
			}
			return;
		}

		setError(undefined);
		if (!recordingOptions) {
			setError(t("minutes.errors.missingRecordingOptions"));
			setIsTogglingRecording(false);
			return;
		}

		try {
			const sessionId = await withTimeout(
				startSession(recordingOptions),
				START_RECORDING_TIMEOUT_MS,
				"Starting recording timed out",
			);
			setSessionId(sessionId);
			setRecording(true);
		} catch (error) {
			setRecording(false);
			setSessionId(undefined);
			setError(error instanceof Error ? error.message : String(error));
		} finally {
			setIsTogglingRecording(false);
		}
	}, [isTogglingRecording, t]);

	const handleGenerate = useCallback(async () => {
		const { isRecording, segments, setMinutes } = useTranscriptStore.getState();

		if (isRecording) {
			setMinutesError(t("minutes.errors.cannotGenerateWhileRecording"));
			return;
		}

		const startTime = Date.now();
		setIsGenerating(true);
		setMinutesError(undefined);
		try {
			const result = await generateMinutes(minutesOptions, segments);
			const durationSeconds = Math.round((Date.now() - startTime) / 1000);
			const mins = Math.floor(durationSeconds / 60);
			const secs = durationSeconds % 60;
			const durationText =
				mins > 0
					? t("minutes.duration.format", { minutes: mins, seconds: secs })
					: t("minutes.duration.secondsOnly", { seconds: secs });

			alert(t("minutes.alerts.generated", { duration: durationText }));
			setMinutes(result);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			setMinutesError(t("minutes.errors.generationFailed", { message }));
		} finally {
			setIsGenerating(false);
		}
	}, [minutesOptions, t]);

	const handleExport = useCallback(async () => {
		const minutes = useTranscriptStore.getState().minutes;
		if (!minutes) return;
		try {
			// Show file save dialog
			const timestamp = new Date(minutes.generatedAt)
				.toISOString()
				.replace(/[:.]/g, "-")
				.slice(0, -5);
			const defaultFilename = `minutes-${timestamp}.md`;

			const filePath = await save({
				defaultPath: defaultFilename,
				filters: [
					{
						name: "Markdown",
						extensions: ["md"],
					},
				],
			});

			if (!filePath) {
				return;
			}

			const savedPath = await exportMinutes(minutes, filePath);
			alert(t("minutes.alerts.exported", { path: savedPath }));
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			alert(t("minutes.alerts.exportFailed", { message }));
		}
	}, [t]);

	const handleClear = useCallback(() => {
		if (isRecording) return;
		useTranscriptStore.getState().reset();
		setMinutesError(undefined);
	}, [isRecording]);

	return (
		<Card>
			<CardHeader>
				<CardTitle className="text-2xl">{t("app.tabs.main")}</CardTitle>
			</CardHeader>
			<CardContent className="space-y-6">
				{/* Minutes Settings */}
				<MinutesOptionsForm
					options={minutesOptions}
					onOptionsChange={setMinutesOptions}
				/>

				{/* Error Messages */}
				{error && <p className="text-sm text-destructive">{error}</p>}
				{minutesError && (
					<p className="text-sm text-destructive">{minutesError}</p>
				)}
			</CardContent>

			<CardFooter className="flex flex-col gap-3 sm:flex-row">
				<Button
					type="button"
					onClick={handleToggleRecording}
					className="w-full sm:w-auto"
					variant={isRecording ? "destructive" : "default"}
					disabled={isTogglingRecording}
				>
					{isRecording
						? t("common.buttons.stopRecording")
						: t("common.buttons.startRecording")}
				</Button>
				<Button
					type="button"
					onClick={handleGenerate}
					disabled={segmentsCount === 0 || isGenerating || isRecording}
					className="w-full sm:w-auto"
				>
					{isGenerating
						? t("minutes.buttons.generating")
						: t("minutes.buttons.generate")}
				</Button>
				<Button
					type="button"
					variant="outline"
					onClick={handleExport}
					disabled={!hasMinutes}
					className="w-full sm:w-auto"
				>
					{t("minutes.buttons.export")}
				</Button>
				<Button
					type="button"
					variant="outline"
					onClick={handleClear}
					disabled={isRecording || !hasClearableData}
					className="w-full sm:w-auto"
				>
					{t("common.buttons.clear")}
				</Button>
			</CardFooter>
		</Card>
	);
}
