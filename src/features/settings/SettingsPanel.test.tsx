import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptStore } from "@/stores/transcript";
import { render, screen } from "@/test/utils";
import { SettingsPanel } from "./SettingsPanel";

const { googleAuthStatusMock, googleAuthSignInMock, googleAuthDisconnectMock } =
	vi.hoisted(() => ({
		googleAuthStatusMock: vi.fn().mockResolvedValue({ connected: false }),
		googleAuthSignInMock: vi.fn().mockResolvedValue({ connected: true }),
		googleAuthDisconnectMock: vi.fn().mockResolvedValue(undefined),
	}));

vi.mock("../../lib/api", () => ({
	googleAuthStatus: googleAuthStatusMock,
	googleAuthSignIn: googleAuthSignInMock,
	googleAuthDisconnect: googleAuthDisconnectMock,
}));

describe("SettingsPanel", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		googleAuthStatusMock.mockResolvedValue({ connected: false });
	});

	it("renders both Vosk and Google Docs settings cards", () => {
		render(<SettingsPanel />);

		expect(screen.getByText("Vosk Settings")).toBeInTheDocument();
		expect(screen.getByText("Google Docs Sync")).toBeInTheDocument();
		expect(screen.getByLabelText("Google Doc URL")).toBeInTheDocument();
	});
});
