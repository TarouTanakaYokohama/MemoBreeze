import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useShallow } from "zustand/react/shallow";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import { Switch } from "../../components/ui/switch";
import {
	googleAuthDisconnect,
	googleAuthSignIn,
	googleAuthStatus,
} from "../../lib/api";
import {
	defaultGoogleDocsSyncOptions,
	type GoogleDocsSyncOptions,
	useTranscriptStore,
} from "../../stores/transcript";

function parseDocumentId(value: string): string {
	const trimmed = value.trim();
	if (!trimmed) return "";

	const fromUrlMatch = trimmed.match(/\/document\/d\/([a-zA-Z0-9_-]+)/);
	if (fromUrlMatch?.[1]) return fromUrlMatch[1];

	return trimmed;
}

export function GoogleDocsSyncCard() {
	const { isRecording, savedOptions, setGoogleDocsSyncOptions } =
		useTranscriptStore(
			useShallow((state) => ({
				isRecording: state.isRecording,
				savedOptions: state.googleDocsSyncOptions,
				setGoogleDocsSyncOptions: state.setGoogleDocsSyncOptions,
			})),
		);
	const { t } = useTranslation();
	const [isAuthenticating, setIsAuthenticating] = useState(false);
	const [isConnected, setIsConnected] = useState(false);
	const [documentUrl, setDocumentUrl] = useState("");
	const authAttemptRef = useRef(0);
	const options = savedOptions ?? defaultGoogleDocsSyncOptions;

	useEffect(() => {
		if (!savedOptions?.documentId) {
			setDocumentUrl("");
			return;
		}
		setDocumentUrl(
			`https://docs.google.com/document/d/${savedOptions.documentId}/edit`,
		);
	}, [savedOptions?.documentId]);

	useEffect(() => {
		void googleAuthStatus()
			.then((status) => setIsConnected(status.connected))
			.catch(() => setIsConnected(false));
	}, []);

	const updateOptions = useCallback(
		(updater: (previous: GoogleDocsSyncOptions) => GoogleDocsSyncOptions) => {
			const current =
				useTranscriptStore.getState().googleDocsSyncOptions ??
				defaultGoogleDocsSyncOptions;
			const next = updater(current);
			setGoogleDocsSyncOptions(next);
		},
		[setGoogleDocsSyncOptions],
	);

	const handleSignIn = useCallback(() => {
		if (isConnected || isAuthenticating) return;

		const attemptId = authAttemptRef.current + 1;
		authAttemptRef.current = attemptId;
		setIsAuthenticating(true);

		void googleAuthSignIn()
			.then((status) => {
				if (authAttemptRef.current !== attemptId) return;
				setIsConnected(status.connected);
			})
			.catch((error) => {
				if (authAttemptRef.current !== attemptId) return;
				const message = error instanceof Error ? error.message : String(error);
				alert(message);
			})
			.finally(() => {
				if (authAttemptRef.current === attemptId) {
					setIsAuthenticating(false);
				}
			});
	}, [isAuthenticating, isConnected]);

	const handleCancelAuth = useCallback(() => {
		authAttemptRef.current += 1;
		setIsAuthenticating(false);
	}, []);

	const handleDisconnect = useCallback(async () => {
		try {
			await googleAuthDisconnect();
			setIsConnected(false);
			updateOptions((prev) => ({ ...prev, enabled: false }));
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			alert(message);
		}
	}, [updateOptions]);

	return (
		<Card>
			<CardHeader>
				<CardTitle>{t("settings.googleDocs.title")}</CardTitle>
				<CardDescription>
					{t("settings.googleDocs.description")}
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4">
				<div className="flex flex-wrap items-center gap-3">
					<Button
						type="button"
						onClick={handleSignIn}
						disabled={isAuthenticating || isConnected}
					>
						{isAuthenticating
							? t("settings.googleDocs.connecting")
							: t("settings.googleDocs.connect")}
					</Button>
					<Button
						type="button"
						variant="outline"
						onClick={handleCancelAuth}
						disabled={!isAuthenticating}
					>
						{t("settings.googleDocs.cancelConnecting")}
					</Button>
					<Button
						type="button"
						variant="outline"
						onClick={handleDisconnect}
						disabled={!isConnected || isAuthenticating}
					>
						{t("settings.googleDocs.disconnect")}
					</Button>
					<span className="text-xs text-muted-foreground">
						{isConnected
							? t("settings.googleDocs.statusConnected")
							: t("settings.googleDocs.statusDisconnected")}
					</span>
				</div>

				<div className="flex items-center justify-between rounded-lg border border-border p-3">
					<div>
						<p className="text-sm font-medium">
							{t("settings.googleDocs.enabledLabel")}
						</p>
						<p className="text-xs text-muted-foreground">
							{t("settings.googleDocs.enabledHint")}
						</p>
					</div>
					<Switch
						checked={options.enabled}
						disabled={isRecording || !isConnected}
						onCheckedChange={(checked) =>
							updateOptions((prev) => ({ ...prev, enabled: checked }))
						}
					/>
				</div>

				<div className="space-y-2">
					<Label htmlFor="googleDocsDocumentUrl">
						{t("settings.googleDocs.documentUrl")}
					</Label>
					<Input
						id="googleDocsDocumentUrl"
						type="url"
						value={documentUrl}
						disabled={isRecording}
						onChange={(event) => {
							const next = event.target.value;
							setDocumentUrl(next);
							updateOptions((prev) => ({
								...prev,
								documentId: parseDocumentId(next),
							}));
						}}
						onBlur={(event) =>
							updateOptions((prev) => ({
								...prev,
								documentId: parseDocumentId(event.target.value),
							}))
						}
						placeholder={t("settings.googleDocs.documentUrlPlaceholder")}
					/>
				</div>

				<p className="text-xs text-muted-foreground">
					{t("settings.googleDocs.note")}
				</p>
			</CardContent>
		</Card>
	);
}
