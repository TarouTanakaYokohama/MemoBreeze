import type { RecordingOptions } from "@shared/types";
import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Switch } from "../../components/ui/switch";
import {
	defaultRecordingOptions,
	useTranscriptStore,
} from "../../stores/transcript";

export function VoskSettingsCard() {
	const { isRecording, savedOptions, setRecordingOptions } = useTranscriptStore(
		useShallow((state) => ({
			isRecording: state.isRecording,
			savedOptions: state.recordingOptions,
			setRecordingOptions: state.setRecordingOptions,
		})),
	);
	const { t } = useTranslation();
	const options = savedOptions ?? defaultRecordingOptions;
	const isMacOS =
		typeof navigator !== "undefined" &&
		navigator.userAgent.toLowerCase().includes("mac");

	const updateOptions = useCallback(
		(updater: (previous: RecordingOptions) => RecordingOptions) => {
			const current =
				useTranscriptStore.getState().recordingOptions ??
				defaultRecordingOptions;
			const next = updater(current);
			setRecordingOptions(next);
		},
		[setRecordingOptions],
	);

	useEffect(() => {
		if (isRecording || isMacOS) {
			return;
		}

		if (options.engine === "vosk" || options.enableOutput) {
			updateOptions((prev) => ({
				...prev,
				engine: "whisper",
				enableOutput: false,
			}));
		}
	}, [isRecording, isMacOS, options.enableOutput, options.engine, updateOptions]);

	return (
		<Card>
			<CardHeader>
				<CardTitle>{t("settings.vosk.title")}</CardTitle>
				<CardDescription>{t("settings.vosk.description")}</CardDescription>
			</CardHeader>
			<CardContent className="space-y-6">
				{isRecording && (
					<p className="text-sm text-muted-foreground font-medium">
						{t("settings.recordingDisabledWarning")}
					</p>
				)}

				<div className="space-y-2">
					<Label htmlFor="engine">{t("recording.engine.label")}</Label>
					<Select
						value={options.engine}
						onValueChange={(value) =>
							updateOptions((prev) => ({
								...prev,
								engine: value as RecordingOptions["engine"],
							}))
						}
						disabled={isRecording}
					>
						<SelectTrigger id="engine">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="vosk" disabled={!isMacOS}>
								{t("recording.engine.options.vosk")}
							</SelectItem>
							<SelectItem value="whisper">
								{t("recording.engine.options.whisper")}
							</SelectItem>
						</SelectContent>
					</Select>
					{!isMacOS && (
						<p className="text-xs text-muted-foreground">
							{t("recording.engine.voskUnsupported")}
						</p>
					)}
				</div>

				{options.engine === "vosk" && (
					<div className="space-y-2">
						<Label htmlFor="modelPath">{t("recording.modelPath")}</Label>
						<Input
							id="modelPath"
							value={options.modelPath}
							placeholder={t("recording.modelPathPlaceholder")}
							disabled={isRecording}
							onChange={(event) =>
								updateOptions((prev) => ({
									...prev,
									modelPath: event.target.value,
								}))
							}
						/>
					</div>
				)}

				{options.engine === "vosk" && (
					<div className="space-y-2">
						<Label htmlFor="speakerModelPath">
							{t("recording.speakerModelPath")}
						</Label>
						<Input
							id="speakerModelPath"
							value={options.speakerModelPath ?? ""}
							disabled={isRecording}
							onChange={(event) =>
								updateOptions((prev) => ({
									...prev,
									speakerModelPath: event.target.value.length
										? event.target.value
										: undefined,
								}))
							}
							placeholder={t("recording.speakerModelPlaceholder")}
						/>
					</div>
				)}

				{options.engine === "whisper" && (
					<>
						<div className="space-y-2">
							<Label htmlFor="whisperModelPath">
								{t("recording.whisperModelPath")}
							</Label>
							<Input
								id="whisperModelPath"
								value={options.whisperModelPath}
								disabled={isRecording}
								onChange={(event) =>
									updateOptions((prev) => ({
										...prev,
										whisperModelPath: event.target.value,
									}))
								}
								placeholder={t("recording.whisperModelPathPlaceholder")}
							/>
						</div>

						<div className="space-y-2">
							<Label htmlFor="whisperLanguage">
								{t("recording.whisperLanguage")}
							</Label>
							<Input
								id="whisperLanguage"
								value={options.whisperLanguage ?? ""}
								disabled={isRecording}
								onChange={(event) =>
									updateOptions((prev) => ({
										...prev,
										whisperLanguage: event.target.value.length
											? event.target.value
											: undefined,
									}))
								}
								placeholder={t("recording.whisperLanguagePlaceholder")}
							/>
						</div>

						<div className="space-y-2">
							<Label htmlFor="whisperCommand">
								{t("recording.whisperCommand")}
							</Label>
							<Input
								id="whisperCommand"
								value={options.whisperCommand}
								disabled={isRecording}
								onChange={(event) =>
									updateOptions((prev) => ({
										...prev,
										whisperCommand: event.target.value,
									}))
								}
								placeholder={t("recording.whisperCommandPlaceholder")}
							/>
						</div>
					</>
				)}

				<div className="grid gap-4 sm:grid-cols-2">
					<div className="flex items-center justify-between rounded-lg border border-border p-3">
						<div>
							<p className="text-sm font-medium">
								{t("recording.captureInput.title")}
							</p>
							<p className="text-xs text-muted-foreground">
								{t("recording.captureInput.description")}
							</p>
						</div>
						<Switch
							checked={options.enableInput}
							disabled={isRecording}
							onCheckedChange={(checked) =>
								updateOptions((prev) => ({ ...prev, enableInput: checked }))
							}
						/>
					</div>
					<div className="flex items-center justify-between rounded-lg border border-border p-3">
						<div>
							<p className="text-sm font-medium">
								{t("recording.captureOutput.title")}
							</p>
							<p className="text-xs text-muted-foreground">
								{t("recording.captureOutput.description")}
							</p>
						</div>
						<Switch
							checked={options.enableOutput}
							disabled={isRecording || !isMacOS}
							onCheckedChange={(checked) =>
								updateOptions((prev) => ({ ...prev, enableOutput: checked }))
							}
						/>
					</div>
				</div>
				{!isMacOS && (
					<p className="text-xs text-muted-foreground">
						{t("recording.captureOutput.unsupported")}
					</p>
				)}

				<div className="space-y-2">
					<Label htmlFor="energy">{t("recording.energyThreshold")}</Label>
					<Input
						id="energy"
						type="number"
						step="0.001"
						disabled={isRecording}
						value={options.energyThreshold}
						onChange={(event) =>
							updateOptions((prev) => ({
								...prev,
								energyThreshold: Number.isNaN(
									Number.parseFloat(event.target.value),
								)
									? 0
									: Number.parseFloat(event.target.value),
							}))
						}
					/>
				</div>
			</CardContent>
		</Card>
	);
}
