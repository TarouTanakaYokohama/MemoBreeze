import type { MinutesOptions } from "@shared/types";
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@/test/utils";

const { listOllamaModelsMock } = vi.hoisted(() => ({
	listOllamaModelsMock: vi.fn(),
}));

vi.mock("../../lib/api", async () => {
	const actual =
		await vi.importActual<typeof import("../../lib/api")>("../../lib/api");
	return {
		...actual,
		listOllamaModels: listOllamaModelsMock,
	};
});

import { MinutesOptionsForm } from "./MinutesOptionsForm";

const baseOptions: MinutesOptions = {
	preset: "default",
	format: "block",
	blockSizeMinutes: 5,
	model: "old-model",
	temperature: 0.2,
};

describe("MinutesOptionsForm", () => {
	it("モデル一覧取得後に利用可能なモデルへ補正する", async () => {
		listOllamaModelsMock.mockResolvedValueOnce(["llama3", "qwen3"]);
		const onOptionsChange = vi.fn();

		render(
			<MinutesOptionsForm
				options={baseOptions}
				onOptionsChange={onOptionsChange}
			/>,
		);

		await waitFor(() => {
			expect(onOptionsChange).toHaveBeenCalledWith({
				...baseOptions,
				model: "llama3",
			});
		});
	});

	it("ブロックサイズと温度を仕様どおり補正して更新する", async () => {
		listOllamaModelsMock.mockResolvedValueOnce([]);
		const onOptionsChange = vi.fn();

		render(
			<MinutesOptionsForm
				options={baseOptions}
				onOptionsChange={onOptionsChange}
			/>,
		);

		const [blockSize, temperature] = screen.getAllByRole("spinbutton");
		fireEvent.change(blockSize, { target: { value: "0" } });
		fireEvent.change(temperature, { target: { value: "2.7" } });

		expect(onOptionsChange).toHaveBeenCalledWith({
			...baseOptions,
			blockSizeMinutes: 5,
		});
		expect(onOptionsChange).toHaveBeenCalledWith({
			...baseOptions,
			temperature: 1,
		});
	});
});
