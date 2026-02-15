import { useTranscriptStore } from "../../stores/transcript";
import { GeneratedMinutesDisplay } from "./GeneratedMinutesDisplay";
import { MainControlCard } from "./MainControlCard";

export function MainPanel() {
	const minutes = useTranscriptStore((state) => state.minutes);

	return (
		<section className="space-y-6">
			<MainControlCard />
			{minutes && <GeneratedMinutesDisplay />}
		</section>
	);
}
