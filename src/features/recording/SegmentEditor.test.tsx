import type { TranscriptSegment } from "@shared/types";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptStore } from "@/stores/transcript";
import { fireEvent, render, screen } from "@/test/utils";

const { updateSegmentMock, assignSpeakerMock } = vi.hoisted(() => ({
	updateSegmentMock: vi.fn(),
	assignSpeakerMock: vi.fn(),
}));

vi.mock("../../lib/api", async () => {
	const actual =
		await vi.importActual<typeof import("../../lib/api")>("../../lib/api");
	return {
		...actual,
		updateSegment: updateSegmentMock,
		assignSpeaker: assignSpeakerMock,
	};
});

import { SegmentEditor } from "./SegmentEditor";

const lateSegment: TranscriptSegment = {
	id: "seg-2",
	speaker: "Speaker 1",
	text: "later",
	start: 30,
	end: 40,
	tokens: [],
	isFinal: true,
};

const earlySegment: TranscriptSegment = {
	id: "seg-1",
	speaker: "Speaker 2",
	text: "earlier",
	start: 10,
	end: 20,
	tokens: [],
	isFinal: false,
};

describe("SegmentEditor", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		updateSegmentMock.mockReset();
		assignSpeakerMock.mockReset();
	});

	it("セグメントがない時は空状態を表示する", () => {
		render(<SegmentEditor />);

		expect(
			screen.getByText("Segments will appear here as transcription starts."),
		).toBeInTheDocument();
	});

	it("開始時刻順で表示し、編集内容をonBlurで更新する", () => {
		useTranscriptStore.setState(
			{ segments: [lateSegment, earlySegment] },
			false,
		);
		render(<SegmentEditor />);

		const textareas = screen.getAllByRole("textbox");
		expect(textareas[0]).toHaveValue("earlier");
		expect(textareas[1]).toHaveValue("later");

		fireEvent.change(textareas[0], { target: { value: "edited text" } });
		fireEvent.blur(textareas[0]);

		expect(updateSegmentMock).toHaveBeenCalledWith({
			...earlySegment,
			text: "edited text",
		});
	});
});
