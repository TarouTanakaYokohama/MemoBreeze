import type {
	MinutesDocument,
	MinutesOptions,
	RecordingOptions,
	SessionSnapshot,
	TranscriptSegment,
} from "@shared/types";
import { invoke } from "@tauri-apps/api/core";

export const startSession = (options: RecordingOptions) =>
	invoke<string>("start_session", { options });

export const stopSession = () => invoke<void>("stop_session");

export const updateSegment = (segment: TranscriptSegment) =>
	invoke<void>("update_segment", { segment });

export const finalizeSegment = (id: string) =>
	invoke<void>("finalize_segment", { id });

export const assignSpeaker = (id: string, speaker: string) =>
	invoke<void>("assign_speaker", { id, speaker });

export const listOllamaModels = () => invoke<string[]>("list_ollama_models");

export const generateMinutes = (
	options: MinutesOptions,
	segments: TranscriptSegment[],
) => invoke<MinutesDocument>("generate_minutes", { options, segments });

export const exportMinutes = (doc: MinutesDocument, directory?: string) =>
	invoke<string>("export_minutes", { document: doc, directory });

export const exportTranscriptMarkdown = (
	segments: TranscriptSegment[],
	path?: string,
) => invoke<string>("export_transcript_markdown", { segments, path });

export interface GoogleDocsAppendPayload {
	segmentId: string;
	speaker: string;
	text: string;
	start: number;
	end: number;
	timestamp: string;
}

export interface GoogleAuthStatus {
	connected: boolean;
}

export const googleAuthSignIn = () =>
	invoke<GoogleAuthStatus>("google_auth_sign_in");

export const googleAuthStatus = () =>
	invoke<GoogleAuthStatus>("google_auth_status");

export const googleAuthDisconnect = () =>
	invoke<void>("google_auth_disconnect");

export const appendTranscriptToGoogleDoc = (
	documentId: string,
	payload: GoogleDocsAppendPayload,
) =>
	invoke<void>("append_google_doc_transcript", {
		documentId,
		payload,
	});

export const saveSnapshot = (path?: string) =>
	invoke<string>("save_snapshot", { path });

export const loadSnapshot = (path: string) =>
	invoke<SessionSnapshot>("load_snapshot", { path });
