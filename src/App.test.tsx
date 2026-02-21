import type { TranscriptSegment } from "@shared/types";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptStore } from "./stores/transcript";
import { act, render, screen } from "./test/utils";

const { transcriptionListenerMock, listOllamaModelsMock } = vi.hoisted(() => ({
	transcriptionListenerMock: vi.fn(),
	listOllamaModelsMock: vi.fn().mockResolvedValue([]),
}));

vi.mock("./hooks/useTranscriptionListener", () => ({
	useTranscriptionListener: transcriptionListenerMock,
}));

vi.mock("./lib/api", async () => {
	const actual = await vi.importActual<typeof import("./lib/api")>("./lib/api");
	return {
		...actual,
		listOllamaModels: listOllamaModelsMock,
	};
});

import App from "./App";

function seedSegment(): TranscriptSegment {
	return {
		id: "seg-1",
		speaker: "Speaker 1",
		text: "hello",
		start: 0,
		end: 1,
		tokens: [],
		isFinal: true,
	};
}

describe("App", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		transcriptionListenerMock.mockClear();
		listOllamaModelsMock.mockResolvedValue([]);
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("renders idle status and metrics from store", () => {
		useTranscriptStore.setState(
			{
				isRecording: false,
				recordedDurationSeconds: 75,
				segments: [seedSegment()],
			},
			false,
		);

		render(<App />);

		expect(screen.getByText("MemoBreeze")).toBeInTheDocument();
		expect(screen.getByText("Idle")).toBeInTheDocument();
		expect(screen.getByText(/Total Duration:\s*1:15/)).toBeInTheDocument();
		expect(screen.getByText("Segments: 1")).toBeInTheDocument();
		expect(transcriptionListenerMock).toHaveBeenCalledTimes(1);
	});

	it("updates total duration while recording", async () => {
		vi.useFakeTimers();
		vi.setSystemTime(new Date("2026-02-20T12:00:00.000Z"));

		useTranscriptStore.setState(
			{
				isRecording: true,
				recordingStartedAt: Date.parse("2026-02-20T11:59:57.000Z"),
				recordedDurationSeconds: 5,
				segments: [seedSegment(), { ...seedSegment(), id: "seg-2" }],
			},
			false,
		);

		render(<App />);
		expect(screen.getByText(/Total Duration:\s*0:08/)).toBeInTheDocument();
		expect(screen.getByText("Segments: 2")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(2_000);
		});

		expect(screen.getByText(/Total Duration:\s*0:10/)).toBeInTheDocument();
	});

	it("renders primary tab labels", () => {
		render(<App />);

		expect(
			screen.getByRole("tab", { name: "Recording & Minutes" }),
		).toBeInTheDocument();
		expect(screen.getByRole("tab", { name: "Transcript" })).toBeInTheDocument();
		expect(screen.getByRole("tab", { name: "Settings" })).toBeInTheDocument();
	});
});
