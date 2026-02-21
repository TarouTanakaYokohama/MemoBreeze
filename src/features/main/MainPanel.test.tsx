import type { MinutesDocument } from "@shared/types";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptStore } from "@/stores/transcript";
import { render, screen } from "@/test/utils";
import { MainPanel } from "./MainPanel";

const { listOllamaModelsMock } = vi.hoisted(() => ({
	listOllamaModelsMock: vi.fn().mockResolvedValue([]),
}));

vi.mock("../../lib/api", async () => {
	const actual =
		await vi.importActual<typeof import("../../lib/api")>("../../lib/api");
	return {
		...actual,
		listOllamaModels: listOllamaModelsMock,
	};
});

const sampleMinutes: MinutesDocument = {
	preset: "default",
	format: "meeting",
	model: "llama3",
	generatedAt: "2026-02-20T11:00:00.000Z",
	summary: { title: "Summary", content: "summary body" },
	decisions: { title: "Decisions", content: "decision body" },
	actions: { title: "Actions", content: "action body" },
	timeline: [],
};

describe("MainPanel", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		listOllamaModelsMock.mockResolvedValue([]);
	});

	it("does not render generated minutes when minutes are missing", () => {
		render(<MainPanel />);

		expect(screen.getByText("Recording & Minutes")).toBeInTheDocument();
		expect(screen.queryByText("Generated Minutes")).not.toBeInTheDocument();
	});

	it("renders generated minutes when minutes exist", () => {
		useTranscriptStore.setState({ minutes: sampleMinutes }, false);

		render(<MainPanel />);

		expect(screen.getByText("Generated Minutes")).toBeInTheDocument();
		expect(screen.getByDisplayValue("summary body")).toBeInTheDocument();
	});
});
