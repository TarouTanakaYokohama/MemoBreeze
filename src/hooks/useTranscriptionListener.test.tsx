import type { TranscriptSegment } from "@shared/types";
import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { listenMock } = vi.hoisted(() => ({
	listenMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
	listen: listenMock,
}));

const { appendTranscriptToGoogleDocMock } = vi.hoisted(() => ({
	appendTranscriptToGoogleDocMock: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("../lib/api", () => ({
	appendTranscriptToGoogleDoc: appendTranscriptToGoogleDocMock,
}));

import { useTranscriptStore } from "@/stores/transcript";
import { useTranscriptionListener } from "./useTranscriptionListener";

describe("useTranscriptionListener", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		listenMock.mockReset();
		appendTranscriptToGoogleDocMock.mockReset();
		appendTranscriptToGoogleDocMock.mockResolvedValue(undefined);
	});

	it("イベントリスナーを登録し、イベント内容をstoreへ反映する", async () => {
		const callbacks = new Map<string, (event: { payload: unknown }) => void>();
		const unsubs = [vi.fn(), vi.fn(), vi.fn()];

		listenMock.mockImplementation(
			async (
				event: string,
				callback: (event: { payload: unknown }) => void,
			) => {
				callbacks.set(event, callback);
				return unsubs[callbacks.size - 1];
			},
		);

		const { unmount } = renderHook(() => useTranscriptionListener());

		await waitFor(() => {
			expect(listenMock).toHaveBeenCalledTimes(3);
		});

		const partial: TranscriptSegment = {
			id: "seg-1",
			speaker: "Speaker 1",
			text: "partial",
			start: 0,
			end: 1,
			tokens: [],
			isFinal: false,
		};

		const final: TranscriptSegment = {
			...partial,
			text: "final",
			isFinal: true,
		};

		callbacks.get("transcription:partial")?.({ payload: partial });
		callbacks.get("transcription:final")?.({ payload: final });
		callbacks.get("transcription:error")?.({ payload: "boom" });

		expect(useTranscriptStore.getState().segments).toHaveLength(1);
		expect(useTranscriptStore.getState().segments[0]).toMatchObject({
			id: "seg-1",
			text: "final",
			isFinal: true,
		});
		expect(useTranscriptStore.getState().lastError).toBe("boom");
		expect(appendTranscriptToGoogleDocMock).not.toHaveBeenCalled();

		unmount();
		for (const unsub of unsubs) {
			expect(unsub).toHaveBeenCalledTimes(1);
		}
	});

	it("Google Docs連携が有効な時はfinalセグメントを送信する", async () => {
		const callbacks = new Map<string, (event: { payload: unknown }) => void>();

		listenMock.mockImplementation(
			async (
				event: string,
				callback: (event: { payload: unknown }) => void,
			) => {
				callbacks.set(event, callback);
				return vi.fn();
			},
		);

		useTranscriptStore.setState(
			{
				googleDocsSyncOptions: {
					enabled: true,
					documentId: "doc-123",
				},
			},
			false,
		);

		renderHook(() => useTranscriptionListener());

		await waitFor(() => {
			expect(listenMock).toHaveBeenCalledTimes(3);
		});

		const final: TranscriptSegment = {
			id: "seg-2",
			speaker: "Speaker 2",
			text: "final segment",
			start: 3,
			end: 5,
			tokens: [],
			isFinal: true,
		};

		callbacks.get("transcription:final")?.({ payload: final });

		await waitFor(() => {
			expect(appendTranscriptToGoogleDocMock).toHaveBeenCalledWith(
				"doc-123",
				expect.objectContaining({
					segmentId: "seg-2",
					speaker: "Speaker 2",
					text: "final segment",
					start: 3,
					end: 5,
				}),
			);
		});
	});
});
