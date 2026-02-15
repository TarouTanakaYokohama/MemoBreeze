import { GoogleDocsSyncCard } from "./GoogleDocsSyncCard";
import { VoskSettingsCard } from "./VoskSettingsCard";

export function SettingsPanel() {
	return (
		<section className="space-y-6">
			<VoskSettingsCard />
			<GoogleDocsSyncCard />
		</section>
	);
}
