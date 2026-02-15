import type { TranscriptSegment } from "@shared/types";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { appendTranscriptToGoogleDoc } from "../lib/api";
import { useTranscriptStore } from "../stores/transcript";

export const useTranscriptionListener = () => {
	const sentSegmentIdsRef = useRef<Set<string>>(new Set());

	useEffect(() => {
		const unsubs: Array<() => void> = [];

		const attach = async () => {
			unsubs.push(
				await listen<TranscriptSegment>(
					"transcription:partial",
					({ payload }) => {
						useTranscriptStore
							.getState()
							.upsertSegment({ ...payload, isFinal: false });
					},
				),
			);

			unsubs.push(
				await listen<TranscriptSegment>(
					"transcription:final",
					({ payload }) => {
						const { upsertSegment, finalizeSegment } =
							useTranscriptStore.getState();
						upsertSegment({ ...payload, isFinal: true });
						finalizeSegment(payload.id);

						if (sentSegmentIdsRef.current.has(payload.id)) {
							return;
						}

						const { googleDocsSyncOptions } = useTranscriptStore.getState();
						if (
							!googleDocsSyncOptions.enabled ||
							!googleDocsSyncOptions.documentId.trim()
						) {
							return;
						}

						sentSegmentIdsRef.current.add(payload.id);
						void appendTranscriptToGoogleDoc(googleDocsSyncOptions.documentId, {
							segmentId: payload.id,
							speaker: payload.speaker,
							text: payload.text,
							start: payload.start,
							end: payload.end,
							timestamp: new Date().toISOString(),
						}).catch((error) => {
							sentSegmentIdsRef.current.delete(payload.id);
							const message =
								error instanceof Error ? error.message : String(error);
							useTranscriptStore
								.getState()
								.setError(`Google Docs sync failed: ${message}`);
						});
					},
				),
			);

			unsubs.push(
				await listen<string>("transcription:error", ({ payload }) => {
					useTranscriptStore.getState().setError(payload);
				}),
			);
		};

		attach();

		return () => {
			for (const unsub of unsubs) unsub();
		};
	}, []);
};
