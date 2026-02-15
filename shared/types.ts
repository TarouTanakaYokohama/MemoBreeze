export type SpeakerLabel = `Speaker ${number}` | "Unknown";

export interface TranscriptToken {
	text: string;
	start: number;
	end: number;
	confidence: number;
}

export interface TranscriptSegment {
	id: string;
	speaker: SpeakerLabel;
	text: string;
	start: number;
	end: number;
	tokens: TranscriptToken[];
	isFinal: boolean;
}

export type MarkerType = "decision" | "action" | "note";

export interface TimelineMarker {
	id: string;
	label: string;
	type: MarkerType;
	timestamp: number;
}

export interface TopicSummary {
	id: string;
	title: string;
	description: string;
	start: number;
	end: number;
	markers: TimelineMarker[];
}

export interface MinutesSection {
	title: string;
	content: string;
}

export interface MinutesDocument {
	preset: string;
	format: "meeting" | "block";
	model: string;
	generatedAt: string;
	summary: MinutesSection;
	decisions: MinutesSection;
	actions: MinutesSection;
	timeline: TopicSummary[];
}

export interface RecordingOptions {
	engine: "vosk" | "whisper";
	modelPath: string;
	speakerModelPath?: string;
	whisperModelPath: string;
	whisperLanguage?: string;
	whisperCommand: string;
	enableInput: boolean;
	enableOutput: boolean;
	energyThreshold: number;
}

export interface MinutesOptions {
	preset: string;
	format: "meeting" | "block";
	blockSizeMinutes: number;
	model: string;
	temperature: number;
}

export interface SessionSnapshot {
	id: string;
	startedAt: string;
	segments: TranscriptSegment[];
}
