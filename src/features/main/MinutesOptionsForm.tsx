import type { MinutesOptions } from "@shared/types";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "../../components/ui/input";
import { Label } from "../../components/ui/label";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import { listOllamaModels } from "../../lib/api";

const presets = [{ id: "default" }, { id: "detailed" }];

interface MinutesOptionsFormProps {
	options: MinutesOptions;
	onOptionsChange: (options: MinutesOptions) => void;
}

export function MinutesOptionsForm({
	options,
	onOptionsChange,
}: MinutesOptionsFormProps) {
	const { t } = useTranslation();
	const [models, setModels] = useState<string[]>([]);

	// biome-ignore lint/correctness/useExhaustiveDependencies: fetch models once on mount only
	useEffect(() => {
		const fetchModels = async () => {
			try {
				const fetched = await listOllamaModels();
				setModels(fetched);
				onOptionsChange({
					...options,
					model: fetched.includes(options.model)
						? options.model
						: (fetched[0] ?? options.model),
				});
			} catch (error) {
				console.error("Failed to fetch Ollama models", error);
			}
		};
		fetchModels();
	}, []);

	const setOption = <K extends keyof MinutesOptions>(
		key: K,
		value: MinutesOptions[K],
	) => {
		onOptionsChange({ ...options, [key]: value });
	};

	return (
		<div className="space-y-4">
			<div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
				<div className="space-y-2">
					<Label className="text-xs">{t("minutes.preset.label")}</Label>
					<Select
						value={options.preset}
						onValueChange={(preset) => setOption("preset", preset)}
					>
						<SelectTrigger className="h-9">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{presets.map((preset) => (
								<SelectItem key={preset.id} value={preset.id}>
									{t(`minutes.preset.options.${preset.id}.name` as const)}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>

				<div className="space-y-2">
					<Label className="text-xs">{t("minutes.model.label")}</Label>
					<Select
						value={options.model}
						onValueChange={(model) => setOption("model", model)}
					>
						<SelectTrigger className="h-9">
							<SelectValue placeholder={t("minutes.model.placeholder")} />
						</SelectTrigger>
						<SelectContent>
							{models.map((model) => (
								<SelectItem key={model} value={model}>
									{model}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				</div>

				<div className="space-y-2">
					<Label className="text-xs">{t("minutes.format.label")}</Label>
					<Select
						value={options.format}
						onValueChange={(value) =>
							setOption("format", value as MinutesOptions["format"])
						}
					>
						<SelectTrigger className="h-9">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="meeting">
								{t("minutes.format.options.meeting")}
							</SelectItem>
							<SelectItem value="block">
								{t("minutes.format.options.block")}
							</SelectItem>
						</SelectContent>
					</Select>
				</div>

				{options.format === "block" && (
					<div className="space-y-2">
						<Label className="text-xs">{t("minutes.format.blockSize")}</Label>
						<Input
							type="number"
							min={1}
							className="h-9"
							value={options.blockSizeMinutes}
							onChange={(event) =>
								setOption(
									"blockSizeMinutes",
									Math.max(1, Number.parseInt(event.target.value, 10) || 5),
								)
							}
						/>
					</div>
				)}

				<div className="space-y-2">
					<Label className="text-xs">{t("minutes.temperature")}</Label>
					<Input
						type="number"
						step={0.1}
						min={0}
						max={1}
						className="h-9"
						value={options.temperature}
						onChange={(event) =>
							setOption(
								"temperature",
								Math.max(
									0,
									Math.min(1, Number.parseFloat(event.target.value) || 0.2),
								),
							)
						}
					/>
				</div>
			</div>
		</div>
	);
}
