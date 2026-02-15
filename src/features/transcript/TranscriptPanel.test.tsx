import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptStore } from "@/stores/transcript";
import { fireEvent, render, screen, waitFor } from "@/test/utils";

const { exportTranscriptMarkdownMock, saveMock } = vi.hoisted(() => ({
	exportTranscriptMarkdownMock: vi.fn(),
	saveMock: vi.fn(),
}));

vi.mock("../../lib/api", async () => {
	const actual =
		await vi.importActual<typeof import("../../lib/api")>("../../lib/api");
	return {
		...actual,
		exportTranscriptMarkdown: exportTranscriptMarkdownMock,
	};
});

vi.mock("@tauri-apps/plugin-dialog", () => ({
	save: saveMock,
}));

import { TranscriptPanel } from "./TranscriptPanel";

describe("TranscriptPanel", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		exportTranscriptMarkdownMock.mockReset();
		saveMock.mockReset();
		vi.stubGlobal("alert", vi.fn());
	});

	it("セグメントがない場合はエクスポートできない", () => {
		render(<TranscriptPanel />);

		expect(
			screen.getByRole("button", { name: "Export Markdown" }),
		).toBeDisabled();
	});

	it("Markdown エクスポートを実行する", async () => {
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
			},
			false,
		);
		saveMock.mockResolvedValueOnce("/tmp/transcript.md");
		exportTranscriptMarkdownMock.mockResolvedValueOnce("/tmp/transcript.md");

		render(<TranscriptPanel />);
		fireEvent.click(screen.getByRole("button", { name: "Export Markdown" }));

		await waitFor(() => {
			expect(saveMock).toHaveBeenCalledTimes(1);
			expect(exportTranscriptMarkdownMock).toHaveBeenCalledWith(
				[
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
				"/tmp/transcript.md",
			);
		});
	});
});
