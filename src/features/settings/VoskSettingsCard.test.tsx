import { beforeEach, describe, expect, it } from "vitest";
import {
	defaultRecordingOptions,
	useTranscriptStore,
} from "@/stores/transcript";
import { fireEvent, render, screen } from "@/test/utils";
import { VoskSettingsCard } from "./VoskSettingsCard";

describe("VoskSettingsCard", () => {
	beforeEach(() => {
		useTranscriptStore.setState(useTranscriptStore.getInitialState(), true);
		useTranscriptStore.setState(
			{
				recordingOptions: defaultRecordingOptions,
			},
			false,
		);
	});

	it("録音中は設定変更を無効化し警告を表示する", () => {
		useTranscriptStore.setState({ isRecording: true }, false);

		render(<VoskSettingsCard />);

		expect(
			screen.getByText("Cannot change settings while recording"),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Whisper Model Path")).toBeDisabled();
	});

	it("入力変更時にstoreの録音設定を更新する", () => {
		render(<VoskSettingsCard />);
		const modelPathInput = screen.getByLabelText("Whisper Model Path");

		fireEvent.change(modelPathInput, {
			target: { value: "/tmp/whisper-model" },
		});

		expect(
			useTranscriptStore.getState().recordingOptions?.whisperModelPath,
		).toBe("/tmp/whisper-model");
	});
});
