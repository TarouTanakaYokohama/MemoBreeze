import { save } from "@tauri-apps/plugin-dialog";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { exportTranscriptMarkdown } from "../../lib/api";
import { useTranscriptStore } from "../../stores/transcript";
import { SegmentEditor } from "../recording/SegmentEditor";

export function TranscriptPanel() {
	const segments = useTranscriptStore((state) => state.segments);
	const { t } = useTranslation();

	const handleExport = useCallback(async () => {
		if (segments.length === 0) return;

		try {
			const timestamp = new Date()
				.toISOString()
				.replace(/[:.]/g, "-")
				.slice(0, -5);
			const defaultFilename = `transcript-${timestamp}.md`;

			const filePath = await save({
				defaultPath: defaultFilename,
				filters: [
					{
						name: "Markdown",
						extensions: ["md"],
					},
				],
			});

			if (!filePath) {
				return;
			}

			const savedPath = await exportTranscriptMarkdown(segments, filePath);
			alert(t("transcript.alerts.exported", { path: savedPath }));
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			alert(t("transcript.alerts.exportFailed", { message }));
		}
	}, [segments, t]);

	return (
		<section className="space-y-4">
			<div className="flex justify-end">
				<Button
					type="button"
					variant="outline"
					onClick={handleExport}
					disabled={segments.length === 0}
				>
					{t("common.buttons.exportMarkdown")}
				</Button>
			</div>
			<SegmentEditor className="max-w-none" />
		</section>
	);
}
