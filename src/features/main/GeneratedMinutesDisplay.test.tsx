import type { MinutesDocument } from "@shared/types";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@/test/utils";
import { useTranscriptStore } from "../../stores/transcript";
import { GeneratedMinutesDisplay } from "./GeneratedMinutesDisplay";

const minutesWithTimeline: MinutesDocument = {
	preset: "default",
	format: "meeting",
	model: "llama3",
	generatedAt: "2026-02-20T11:00:00.000Z",
	summary: { title: "Summary", content: "Summary body" },
	decisions: { title: "Decisions", content: "Decision body" },
	actions: { title: "Actions", content: "Action body" },
	timeline: [
		{
			id: "topic-1",
			title: "Architecture",
			description: "Discussed migration strategy",
			start: 62,
			end: 133,
			markers: [
				{
					id: "marker-1",
					type: "decision",
					label: "Move to feature flags",
					timestamp: 90,
				},
			],
		},
	],
};

describe("GeneratedMinutesDisplay", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
	});

	it("renders summary/decision/action sections and timeline markers", async () => {
		useTranscriptStore.setState({ minutes: minutesWithTimeline }, false);
		render(<GeneratedMinutesDisplay />);
		const user = userEvent.setup();

		expect(screen.getByText("Generated Minutes")).toBeInTheDocument();
		expect(screen.getByDisplayValue("Summary body")).toBeInTheDocument();

		await user.click(screen.getByRole("tab", { name: "Decisions" }));
		expect(
			await screen.findByDisplayValue("Decision body"),
		).toBeInTheDocument();

		await user.click(screen.getByRole("tab", { name: "Actions" }));
		expect(await screen.findByDisplayValue("Action body")).toBeInTheDocument();

		await user.click(screen.getByRole("tab", { name: "Timeline" }));
		expect(await screen.findByText("Architecture")).toBeInTheDocument();
		expect(
			screen.getByText("Discussed migration strategy"),
		).toBeInTheDocument();
		expect(screen.getByText(/Move to feature flags/)).toBeInTheDocument();
	});

	it("renders empty timeline message when no timeline exists", async () => {
		useTranscriptStore.setState(
			{
				minutes: {
					...minutesWithTimeline,
					timeline: [],
				},
			},
			false,
		);
		render(<GeneratedMinutesDisplay />);
		const user = userEvent.setup();

		await user.click(screen.getByRole("tab", { name: "Timeline" }));
		expect(await screen.findByText("No timeline items.")).toBeInTheDocument();
	});
});
