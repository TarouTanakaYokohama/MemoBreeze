import { beforeEach, describe, expect, it, vi } from "vitest";
import {
	defaultRecordingOptions,
	useTranscriptStore,
} from "@/stores/transcript";
import { fireEvent, render, screen, waitFor } from "@/test/utils";

const {
	startSessionMock,
	stopSessionMock,
	generateMinutesMock,
	exportMinutesMock,
} = vi.hoisted(() => ({
	startSessionMock: vi.fn(),
	stopSessionMock: vi.fn(),
	generateMinutesMock: vi.fn(),
	exportMinutesMock: vi.fn(),
}));

vi.mock("../../lib/api", async () => {
	const actual =
		await vi.importActual<typeof import("../../lib/api")>("../../lib/api");
	return {
		...actual,
		startSession: startSessionMock,
		stopSession: stopSessionMock,
		generateMinutes: generateMinutesMock,
		exportMinutes: exportMinutesMock,
		listOllamaModels: vi.fn().mockResolvedValue(["llama3"]),
	};
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
	save: vi.fn(),
}));

import { MainControlCard } from "./MainControlCard";

describe("MainControlCard", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		startSessionMock.mockReset();
		stopSessionMock.mockReset();
		generateMinutesMock.mockReset();
		exportMinutesMock.mockReset();
	});

	it("録音オプション未設定時は録音開始エラーを表示する", async () => {
		useTranscriptStore.setState(
			{
				recordingOptions: undefined,
			},
			false,
		);

		render(<MainControlCard />);
		fireEvent.click(screen.getByRole("button", { name: "Start Recording" }));

		await waitFor(() => {
			expect(
				screen.getByText(
					"Set up recording options on the Recording tab before starting.",
				),
			).toBeInTheDocument();
		});
	});

	it("録音開始成功時にセッション状態を更新する", async () => {
		useTranscriptStore.setState(
			{
				recordingOptions: defaultRecordingOptions,
			},
			false,
		);
		startSessionMock.mockResolvedValueOnce("session-123");

		render(<MainControlCard />);
		fireEvent.click(screen.getByRole("button", { name: "Start Recording" }));

		await waitFor(() => {
			expect(startSessionMock).toHaveBeenCalledTimes(1);
			expect(useTranscriptStore.getState().isRecording).toBe(true);
			expect(useTranscriptStore.getState().activeSessionId).toBe("session-123");
		});
	});

	it("停止に失敗したときは録音状態を維持して再試行できる", async () => {
		useTranscriptStore.setState(
			{
				recordingOptions: defaultRecordingOptions,
				isRecording: true,
				activeSessionId: "session-123",
				segments: [],
			},
			false,
		);
		stopSessionMock.mockRejectedValueOnce(new Error("stop failed"));

		render(<MainControlCard />);
		fireEvent.click(screen.getByRole("button", { name: "Stop Recording" }));

		await waitFor(() => {
			expect(useTranscriptStore.getState().isRecording).toBe(true);
			expect(useTranscriptStore.getState().activeSessionId).toBe("session-123");
			expect(
				screen.getByRole("button", { name: "Stop Recording" }),
			).toBeEnabled();
		});
	});

	it("クリアボタンで書き起こしと議事録を初期化する", () => {
		useTranscriptStore.setState(
			{
				segments: [
					{
						id: "seg-1",
						speaker: "Speaker 1",
						text: "hello",
						start: 0,
						end: 1,
						tokens: [],
						isFinal: true,
					},
				],
				minutes: {
					preset: "default",
					format: "meeting",
					model: "llama3",
					generatedAt: new Date().toISOString(),
					summary: { title: "Summary", content: "summary" },
					decisions: { title: "Decisions", content: "none" },
					actions: { title: "Actions", content: "none" },
					timeline: [],
				},
			},
			false,
		);

		render(<MainControlCard />);
		fireEvent.click(screen.getByRole("button", { name: "Clear" }));

		const state = useTranscriptStore.getState();
		expect(state.segments).toEqual([]);
		expect(state.minutes).toBeUndefined();
		expect(state.recordedDurationSeconds).toBe(0);
	});
});
