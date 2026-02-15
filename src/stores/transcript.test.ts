import type { TranscriptSegment } from "@shared/types";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
	defaultGoogleDocsSyncOptions,
	defaultMinutesOptions,
	defaultRecordingOptions,
	useTranscriptStore,
} from "./transcript";

const segmentA: TranscriptSegment = {
	id: "seg-1",
	speaker: "Speaker 1",
	text: "hello",
	start: 1,
	end: 2,
	tokens: [],
	isFinal: false,
};

describe("useTranscriptStore", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("録音オプションのデフォルト値を持つ", () => {
		expect(useTranscriptStore.getState().recordingOptions).toEqual(
			defaultRecordingOptions,
		);
		expect(useTranscriptStore.getState().minutesOptions).toEqual(
			defaultMinutesOptions,
		);
		expect(useTranscriptStore.getState().googleDocsSyncOptions).toEqual(
			defaultGoogleDocsSyncOptions,
		);
	});

	it("新規セグメントを追加し、既存セグメントは更新する", () => {
		const store = useTranscriptStore.getState();

		store.upsertSegment(segmentA);
		store.upsertSegment({ ...segmentA, text: "updated", isFinal: true });

		expect(useTranscriptStore.getState().segments).toHaveLength(1);
		expect(useTranscriptStore.getState().segments[0]).toMatchObject({
			id: "seg-1",
			text: "updated",
			isFinal: true,
		});
	});

	it("セグメント確定・本文更新・話者再割り当てができる", () => {
		const store = useTranscriptStore.getState();
		store.upsertSegment(segmentA);

		store.updateSegmentText(segmentA.id, "changed");
		store.reassignSpeaker(segmentA.id, "Unknown");
		store.finalizeSegment(segmentA.id);

		expect(useTranscriptStore.getState().segments[0]).toMatchObject({
			text: "changed",
			speaker: "Unknown",
			isFinal: true,
		});
	});

	it("セッション関連データをリセットする", () => {
		const store = useTranscriptStore.getState();
		store.upsertSegment(segmentA);
		store.setSessionId("session-1");
		store.setRecording(true);
		store.setError("error");

		store.reset();

		expect(useTranscriptStore.getState()).toMatchObject({
			segments: [],
			activeSessionId: undefined,
			isRecording: false,
			lastError: undefined,
		});
	});

	it("録音開始時刻からの経過秒を累積する", () => {
		vi.useFakeTimers();
		vi.setSystemTime(new Date("2026-02-15T10:00:00.000Z"));

		const store = useTranscriptStore.getState();
		store.setRecording(true);

		expect(useTranscriptStore.getState().isRecording).toBe(true);
		expect(useTranscriptStore.getState().recordingStartedAt).toBe(
			Date.parse("2026-02-15T10:00:00.000Z"),
		);

		vi.advanceTimersByTime(5_400);
		store.setRecording(false);

		expect(useTranscriptStore.getState()).toMatchObject({
			isRecording: false,
			recordingStartedAt: undefined,
			recordedDurationSeconds: 5,
		});
	});
});
