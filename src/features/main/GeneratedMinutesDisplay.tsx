import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { ScrollArea } from "../../components/ui/scroll-area";
import {
	Tabs,
	TabsContent,
	TabsList,
	TabsTrigger,
} from "../../components/ui/tabs";
import { Textarea } from "../../components/ui/textarea";
import { formatTimestamp } from "../../lib/utils";
import { useTranscriptStore } from "../../stores/transcript";

export function GeneratedMinutesDisplay() {
	const minutes = useTranscriptStore((state) => state.minutes);
	const { t } = useTranslation();

	if (!minutes) return null;

	return (
		<Card>
			<CardHeader>
				<CardTitle>{t("minutes.generated.title")}</CardTitle>
				<CardDescription>
					{t("minutes.generated.description", {
						datetime: new Date(minutes.generatedAt).toLocaleString(),
						model: minutes.model,
					})}
				</CardDescription>
			</CardHeader>
			<CardContent>
				<Tabs defaultValue="summary">
					<TabsList>
						<TabsTrigger value="summary">
							{t("minutes.generated.tabs.summary")}
						</TabsTrigger>
						<TabsTrigger value="decisions">
							{t("minutes.generated.tabs.decisions")}
						</TabsTrigger>
						<TabsTrigger value="actions">
							{t("minutes.generated.tabs.actions")}
						</TabsTrigger>
						<TabsTrigger value="timeline">
							{t("minutes.generated.tabs.timeline")}
						</TabsTrigger>
					</TabsList>
					<TabsContent value="summary" className="mt-4">
						<Textarea
							value={minutes.summary.content}
							readOnly
							className="h-48"
						/>
					</TabsContent>
					<TabsContent value="decisions" className="mt-4">
						<Textarea
							value={
								minutes.decisions.content || t("minutes.generated.sectionEmpty")
							}
							readOnly
							className="h-48"
						/>
					</TabsContent>
					<TabsContent value="actions" className="mt-4">
						<Textarea
							value={
								minutes.actions.content || t("minutes.generated.sectionEmpty")
							}
							readOnly
							className="h-48"
						/>
					</TabsContent>
					<TabsContent value="timeline" className="mt-4">
						<ScrollArea className="h-64 rounded-md border border-border">
							<div className="space-y-4 p-4">
								{minutes.timeline.map((topic) => (
									<div key={topic.id} className="space-y-2">
										<div className="flex items-center justify-between">
											<p className="text-sm font-medium">{topic.title}</p>
											<Badge variant="secondary">
												{formatTimestamp(topic.start)} -{" "}
												{formatTimestamp(topic.end)}
											</Badge>
										</div>
										<p className="text-sm text-muted-foreground">
											{topic.description}
										</p>
										{topic.markers.length > 0 && (
											<ul className="list-disc space-y-1 pl-5 text-sm">
												{topic.markers.map((marker) => (
													<li key={marker.id}>
														{t(`common.markerTypes.${marker.type}` as const)}:{" "}
														{marker.label}
													</li>
												))}
											</ul>
										)}
									</div>
								))}
								{!minutes.timeline.length && (
									<p className="text-sm text-muted-foreground">
										{t("minutes.generated.timelineEmpty")}
									</p>
								)}
							</div>
						</ScrollArea>
					</TabsContent>
				</Tabs>
			</CardContent>
		</Card>
	);
}
