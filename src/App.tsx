import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";
import { LanguageSwitcher } from "./components/LanguageSwitcher";
import { ModeToggle } from "./components/ModeToggle";
import { Badge } from "./components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";
import { MainPanel } from "./features/main/MainPanel";
import { SettingsPanel } from "./features/settings/SettingsPanel";
import { TranscriptPanel } from "./features/transcript/TranscriptPanel";
import { useTranscriptionListener } from "./hooks/useTranscriptionListener";
import { formatTimestamp } from "./lib/utils";
import { useTranscriptStore } from "./stores/transcript";

function App() {
	useTranscriptionListener();
	const {
		segmentsCount,
		isRecording,
		recordingStartedAt,
		recordedDurationSeconds,
	} = useTranscriptStore(
		useShallow((state) => ({
			segmentsCount: state.segments.length,
			isRecording: state.isRecording,
			recordingStartedAt: state.recordingStartedAt,
			recordedDurationSeconds: state.recordedDurationSeconds,
		})),
	);
	const { t } = useTranslation();
	const [now, setNow] = useState(() => Date.now());

	useEffect(() => {
		if (!isRecording || !recordingStartedAt) return;

		const intervalId = window.setInterval(() => {
			setNow(Date.now());
		}, 1000);

		return () => {
			window.clearInterval(intervalId);
		};
	}, [isRecording, recordingStartedAt]);

	const totalDuration = useMemo(() => {
		if (!isRecording || !recordingStartedAt) {
			if (recordedDurationSeconds === 0) return "00:00";
			return formatTimestamp(recordedDurationSeconds);
		}

		const elapsedSeconds = Math.max(
			0,
			Math.floor((now - recordingStartedAt) / 1000),
		);
		return formatTimestamp(recordedDurationSeconds + elapsedSeconds);
	}, [isRecording, now, recordedDurationSeconds, recordingStartedAt]);

	return (
		<div className="space-y-8 p-6">
			<header className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
				<div>
					<h1 className="text-3xl font-semibold tracking-tight">MemoBreeze</h1>
					<p className="text-sm text-muted-foreground">{t("app.tagline")}</p>
					<div className="mt-2 flex items-center gap-3 text-xs text-muted-foreground">
						<Badge variant={isRecording ? "default" : "secondary"}>
							{isRecording ? t("app.status.recording") : t("app.status.idle")}
						</Badge>
						<span>
							{t("app.status.totalDuration")}: {totalDuration}
						</span>
						<span>
							{t("app.status.segments")}: {segmentsCount}
						</span>
					</div>
				</div>
				<div className="flex items-center gap-2 self-start sm:self-auto">
					<LanguageSwitcher />
					<ModeToggle />
				</div>
			</header>

			<Tabs defaultValue="main" className="space-y-6">
				<TabsList>
					<TabsTrigger value="main">{t("app.tabs.main")}</TabsTrigger>
					<TabsTrigger value="transcript">
						{t("app.tabs.transcript")}
					</TabsTrigger>
					<TabsTrigger value="settings">{t("app.tabs.settings")}</TabsTrigger>
				</TabsList>
				<TabsContent value="main" forceMount>
					<MainPanel />
				</TabsContent>
				<TabsContent value="transcript" forceMount>
					<TranscriptPanel />
				</TabsContent>
				<TabsContent value="settings" forceMount>
					<SettingsPanel />
				</TabsContent>
			</Tabs>
		</div>
	);
}

export default App;
