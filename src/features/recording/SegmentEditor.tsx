import type { TranscriptSegment } from "@shared/types";
import { memo, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { Label } from "../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { Textarea } from "../../components/ui/textarea";
import { assignSpeaker, updateSegment } from "../../lib/api";
import { cn, formatTimestamp } from "../../lib/utils";
import { useTranscriptStore } from "../../stores/transcript";

interface SegmentEditorProps {
	className?: string;
}

interface SpeakerOption {
	value: TranscriptSegment["speaker"];
	label: string;
}

interface SegmentLabels {
	finalStatus: string;
	partialStatus: string;
	speakerLabel: string;
	unknownSpeaker: string;
}

interface SegmentRowProps {
	segment: TranscriptSegment;
	speakerOptions: SpeakerOption[];
	labels: SegmentLabels;
	onTextChange: (segment: TranscriptSegment, text: string) => void;
	onSpeakerChange: (segmentId: string, speaker: string) => void;
}

const SegmentRow = memo(function SegmentRow({
	segment,
	speakerOptions,
	labels,
	onTextChange,
	onSpeakerChange,
}: SegmentRowProps) {
	return (
		<div
			className={cn(
				"space-y-2 rounded-lg border border-border p-4",
				!segment.isFinal && "border-dashed",
			)}
		>
			<div className="flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2 text-sm text-muted-foreground">
					<Badge variant={segment.isFinal ? "default" : "secondary"}>
						{segment.isFinal ? labels.finalStatus : labels.partialStatus}
					</Badge>
					<span>
						{formatTimestamp(segment.start)} - {formatTimestamp(segment.end)}
					</span>
				</div>
				<div className="flex items-center gap-2">
					<Label
						htmlFor={`speaker-${segment.id}`}
						className="text-xs uppercase tracking-wide"
					>
						{labels.speakerLabel}
					</Label>
					<Select
						value={segment.speaker}
						onValueChange={(value) => onSpeakerChange(segment.id, value)}
					>
						<SelectTrigger id={`speaker-${segment.id}`} className="w-[140px]">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{speakerOptions.map((option) => (
								<SelectItem key={option.value} value={option.value}>
									{option.label}
								</SelectItem>
							))}
							<SelectItem value="Unknown">{labels.unknownSpeaker}</SelectItem>
						</SelectContent>
					</Select>
				</div>
			</div>
			<Textarea
				defaultValue={segment.text}
				onBlur={(event) => onTextChange(segment, event.target.value)}
			/>
		</div>
	);
});

export const SegmentEditor = ({ className }: SegmentEditorProps) => {
	const segments = useTranscriptStore((state) => state.segments);
	const { t } = useTranslation();

	const sortedSegments = useMemo(
		() => [...segments].sort((a, b) => a.start - b.start),
		[segments],
	);

	const speakerOptions = useMemo(
		() =>
			Array.from({ length: 8 }, (_, index) => ({
				value: `Speaker ${index + 1}` as TranscriptSegment["speaker"],
				label: t("segmentEditor.speakerOption", { index: index + 1 }),
			})),
		[t],
	);

	const labels = useMemo<SegmentLabels>(
		() => ({
			finalStatus: t("segmentEditor.status.final"),
			partialStatus: t("segmentEditor.status.partial"),
			speakerLabel: t("segmentEditor.speakerLabel"),
			unknownSpeaker: t("segmentEditor.speakerUnknown"),
		}),
		[t],
	);

	const handleTextChange = useCallback(
		(segment: TranscriptSegment, text: string) => {
			const updated: TranscriptSegment = { ...segment, text };
			updateSegment(updated);
		},
		[],
	);

	const handleSpeakerChange = useCallback(
		(segmentId: string, speaker: string) => {
			assignSpeaker(segmentId, speaker);
		},
		[],
	);

	return (
		<Card className={className}>
			<CardHeader className="flex flex-row items-center justify-between">
				<div>
					<CardTitle>{t("segmentEditor.title")}</CardTitle>
					<p className="text-sm text-muted-foreground">
						{t("segmentEditor.description")}
					</p>
				</div>
				<Badge variant="secondary">
					{t("segmentEditor.segmentCount", { count: segments.length })}
				</Badge>
			</CardHeader>
			<CardContent className="space-y-4">
				{sortedSegments.map((segment) => (
					<SegmentRow
						key={segment.id}
						segment={segment}
						speakerOptions={speakerOptions}
						labels={labels}
						onTextChange={handleTextChange}
						onSpeakerChange={handleSpeakerChange}
					/>
				))}
				{sortedSegments.length === 0 && (
					<p className="text-sm text-muted-foreground">
						{t("segmentEditor.empty")}
					</p>
				)}
			</CardContent>
		</Card>
	);
};
