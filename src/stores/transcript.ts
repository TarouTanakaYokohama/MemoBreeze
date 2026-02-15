import type {
	MinutesDocument,
	MinutesOptions,
	RecordingOptions,
	TranscriptSegment,
} from "@shared/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

export const defaultRecordingOptions: RecordingOptions = {
	engine: "whisper",
	modelPath: "~/vosk-model",
	speakerModelPath: undefined,
	whisperModelPath: "~/whisper.cpp/models/ggml-base.bin",
	whisperLanguage: "ja",
	whisperCommand: "whisper-cli",
	enableInput: true,
	enableOutput: false,
	energyThreshold: 0,
};

export interface GoogleDocsSyncOptions {
	enabled: boolean;
	documentId: string;
}

export const defaultGoogleDocsSyncOptions: GoogleDocsSyncOptions = {
	enabled: false,
	documentId: "",
};

export const defaultMinutesOptions: MinutesOptions = {
	preset: "default",
	format: "meeting",
	blockSizeMinutes: 5,
	model: "llama3",
	temperature: 0.2,
};

interface TranscriptState {
	segments: TranscriptSegment[];
	activeSessionId?: string;
	isRecording: boolean;
	recordingStartedAt?: number;
	recordedDurationSeconds: number;
	lastError?: string;
	minutes?: MinutesDocument;
	recordingOptions?: RecordingOptions;
	minutesOptions?: MinutesOptions;
	googleDocsSyncOptions: GoogleDocsSyncOptions;
	setRecording(flag: boolean): void;
	setSessionId(id: string | undefined): void;
	upsertSegment(segment: TranscriptSegment): void;
	finalizeSegment(id: string): void;
	updateSegmentText(id: string, text: string): void;
	reassignSpeaker(id: string, speaker: TranscriptSegment["speaker"]): void;
	setSegments(segments: TranscriptSegment[]): void;
	setError(message?: string): void;
	setMinutes(doc?: MinutesDocument): void;
	setRecordingOptions(options: RecordingOptions): void;
	setMinutesOptions(options: MinutesOptions): void;
	setGoogleDocsSyncOptions(options: GoogleDocsSyncOptions): void;
	reset(): void;
}

const TRANSCRIPT_SETTINGS_STORAGE_KEY = "memo-breeze-transcript-settings";

export const useTranscriptStore = create<TranscriptState>()(
	persist(
		(set, get) => ({
			segments: [],
			isRecording: false,
			recordedDurationSeconds: 0,
			recordingOptions: defaultRecordingOptions,
			minutesOptions: defaultMinutesOptions,
			googleDocsSyncOptions: defaultGoogleDocsSyncOptions,
			setRecording: (flag) => {
				const { isRecording, recordingStartedAt, recordedDurationSeconds } =
					get();

				if (flag) {
					if (isRecording) return;
					set({ isRecording: true, recordingStartedAt: Date.now() });
					return;
				}

				if (!isRecording) return;
				const elapsedSeconds = recordingStartedAt
					? Math.max(0, Math.floor((Date.now() - recordingStartedAt) / 1000))
					: 0;

				set({
					isRecording: false,
					recordingStartedAt: undefined,
					recordedDurationSeconds: recordedDurationSeconds + elapsedSeconds,
				});
			},
			setSessionId: (activeSessionId) => set({ activeSessionId }),
			upsertSegment: (segment) => {
				const existing = get().segments.findIndex((s) => s.id === segment.id);
				if (existing >= 0) {
					set((state) => ({
						segments: state.segments.map((s) =>
							s.id === segment.id ? { ...s, ...segment } : s,
						),
					}));
				} else {
					set((state) => ({ segments: [...state.segments, segment] }));
				}
			},
			finalizeSegment: (id) => {
				set((state) => ({
					segments: state.segments.map((segment) =>
						segment.id === id ? { ...segment, isFinal: true } : segment,
					),
				}));
			},
			updateSegmentText: (id, text) =>
				set((state) => ({
					segments: state.segments.map((segment) =>
						segment.id === id ? { ...segment, text } : segment,
					),
				})),
			reassignSpeaker: (id, speaker) => {
				set((state) => ({
					segments: state.segments.map((segment) =>
						segment.id === id ? { ...segment, speaker } : segment,
					),
				}));
			},
			setSegments: (segments) => set({ segments }),
			setError: (message) => set({ lastError: message }),
			setMinutes: (minutes) => set({ minutes }),
			setRecordingOptions: (recordingOptions) => set({ recordingOptions }),
			setMinutesOptions: (minutesOptions) => set({ minutesOptions }),
			setGoogleDocsSyncOptions: (googleDocsSyncOptions) =>
				set({ googleDocsSyncOptions }),
			reset: () =>
				set({
					segments: [],
					minutes: undefined,
					activeSessionId: undefined,
					isRecording: false,
					recordingStartedAt: undefined,
					recordedDurationSeconds: 0,
					lastError: undefined,
				}),
		}),
		{
			name: TRANSCRIPT_SETTINGS_STORAGE_KEY,
			partialize: (state) => ({
				recordingOptions: state.recordingOptions,
				minutesOptions: state.minutesOptions,
				googleDocsSyncOptions: state.googleDocsSyncOptions,
			}),
			merge: (persisted, current) => {
				const persistedState = persisted as
					| Partial<TranscriptState>
					| undefined;
				return {
					...current,
					recordingOptions:
						persistedState?.recordingOptions ?? current.recordingOptions,
					minutesOptions:
						persistedState?.minutesOptions ?? current.minutesOptions,
					googleDocsSyncOptions:
						persistedState?.googleDocsSyncOptions ??
						current.googleDocsSyncOptions,
				};
			},
		},
	),
);
