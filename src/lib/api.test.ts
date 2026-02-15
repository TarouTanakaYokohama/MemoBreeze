import type {
	MinutesDocument,
	MinutesOptions,
	RecordingOptions,
	TranscriptSegment,
} from "@shared/types";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
	invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
	invoke: invokeMock,
}));

import {
	appendTranscriptToGoogleDoc,
	assignSpeaker,
	exportMinutes,
	exportTranscriptMarkdown,
	finalizeSegment,
	generateMinutes,
	googleAuthDisconnect,
	googleAuthSignIn,
	googleAuthStatus,
	listOllamaModels,
	loadSnapshot,
	saveSnapshot,
	startSession,
	stopSession,
	updateSegment,
} from "./api";

const recordingOptions: RecordingOptions = {
	engine: "vosk",
	modelPath: "/model",
	whisperModelPath: "/whisper",
	whisperLanguage: "ja",
	whisperCommand: "whisper-cli",
	enableInput: true,
	enableOutput: false,
	energyThreshold: 1,
};

const segment: TranscriptSegment = {
	id: "seg-1",
	speaker: "Speaker 1",
	text: "hello",
	start: 0,
	end: 1,
	tokens: [],
	isFinal: false,
};

const minutesOptions: MinutesOptions = {
	preset: "default",
	format: "meeting",
	blockSizeMinutes: 10,
	model: "llama3",
	temperature: 0.3,
};

const minutesDoc: MinutesDocument = {
	preset: "default",
	format: "meeting",
	model: "llama3",
	generatedAt: "2025-01-01T00:00:00Z",
	summary: { title: "Summary", content: "content" },
	decisions: { title: "Decisions", content: "content" },
	actions: { title: "Actions", content: "content" },
	timeline: [],
};

describe("apiラッパー", () => {
	beforeEach(() => {
		invokeMock.mockReset();
		invokeMock.mockResolvedValue(undefined);
	});

	it("セッション関連コマンドを呼び出す", async () => {
		await startSession(recordingOptions);
		await stopSession();

		expect(invokeMock).toHaveBeenNthCalledWith(1, "start_session", {
			options: recordingOptions,
		});
		expect(invokeMock).toHaveBeenNthCalledWith(2, "stop_session");
	});

	it("セグメント関連コマンドを呼び出す", async () => {
		await updateSegment(segment);
		await finalizeSegment(segment.id);
		await assignSpeaker(segment.id, "Speaker 2");

		expect(invokeMock).toHaveBeenNthCalledWith(1, "update_segment", {
			segment,
		});
		expect(invokeMock).toHaveBeenNthCalledWith(2, "finalize_segment", {
			id: segment.id,
		});
		expect(invokeMock).toHaveBeenNthCalledWith(3, "assign_speaker", {
			id: segment.id,
			speaker: "Speaker 2",
		});
	});

	it("議事録関連コマンドを呼び出す", async () => {
		await listOllamaModels();
		await generateMinutes(minutesOptions, [segment]);
		await exportMinutes(minutesDoc, "/tmp");
		await exportTranscriptMarkdown([segment], "/tmp/transcript.md");
		await googleAuthSignIn();
		await googleAuthStatus();
		await googleAuthDisconnect();
		await appendTranscriptToGoogleDoc("doc-1", {
			segmentId: segment.id,
			speaker: segment.speaker,
			text: segment.text,
			start: segment.start,
			end: segment.end,
			timestamp: "2025-01-01T00:00:00Z",
		});

		expect(invokeMock).toHaveBeenNthCalledWith(1, "list_ollama_models");
		expect(invokeMock).toHaveBeenNthCalledWith(2, "generate_minutes", {
			options: minutesOptions,
			segments: [segment],
		});
		expect(invokeMock).toHaveBeenNthCalledWith(3, "export_minutes", {
			document: minutesDoc,
			directory: "/tmp",
		});
		expect(invokeMock).toHaveBeenNthCalledWith(
			4,
			"export_transcript_markdown",
			{
				segments: [segment],
				path: "/tmp/transcript.md",
			},
		);
		expect(invokeMock).toHaveBeenNthCalledWith(5, "google_auth_sign_in");
		expect(invokeMock).toHaveBeenNthCalledWith(6, "google_auth_status");
		expect(invokeMock).toHaveBeenNthCalledWith(7, "google_auth_disconnect");
		expect(invokeMock).toHaveBeenNthCalledWith(
			8,
			"append_google_doc_transcript",
			{
				documentId: "doc-1",
				payload: {
					segmentId: segment.id,
					speaker: segment.speaker,
					text: segment.text,
					start: segment.start,
					end: segment.end,
					timestamp: "2025-01-01T00:00:00Z",
				},
			},
		);
	});

	it("スナップショット関連コマンドを呼び出す", async () => {
		await saveSnapshot("/tmp/snap.json");
		await loadSnapshot("/tmp/snap.json");

		expect(invokeMock).toHaveBeenNthCalledWith(1, "save_snapshot", {
			path: "/tmp/snap.json",
		});
		expect(invokeMock).toHaveBeenNthCalledWith(2, "load_snapshot", {
			path: "/tmp/snap.json",
		});
	});
});
