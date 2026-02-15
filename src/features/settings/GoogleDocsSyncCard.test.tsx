import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTranscriptStore } from "@/stores/transcript";
import { fireEvent, render, screen } from "@/test/utils";
import { GoogleDocsSyncCard } from "./GoogleDocsSyncCard";

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

describe("GoogleDocsSyncCard", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		googleAuthStatusMock.mockResolvedValue({ connected: true });
		googleAuthSignInMock.mockResolvedValue({ connected: true });
		googleAuthDisconnectMock.mockResolvedValue(undefined);
	});

	it("入力変更時にstoreのGoogle Docs設定を更新する", () => {
		render(<GoogleDocsSyncCard />);

		fireEvent.change(screen.getByLabelText("Google Doc URL"), {
			target: { value: "https://docs.google.com/document/d/doc-1/edit" },
		});
		fireEvent.blur(screen.getByLabelText("Google Doc URL"));

		expect(useTranscriptStore.getState().googleDocsSyncOptions).toEqual({
			enabled: false,
			documentId: "doc-1",
		});
	});
});
