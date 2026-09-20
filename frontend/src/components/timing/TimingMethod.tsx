interface TimingMethodProps {
	historyYears: number[];
	depthCm: number;
}

/** "How this is calculated" for the timing page. Full write-up: docs/timing-windows.md. */
export default function TimingMethod({
	historyYears,
	depthCm,
}: TimingMethodProps) {
	const span =
		historyYears.length > 0
			? `${historyYears[0]}–${historyYears[historyYears.length - 1]} (${historyYears.length} complete years)`
			: "no complete years yet";

	return (
		<details style={styles.details}>
			<summary style={styles.summary}>How this is calculated</summary>
			<div style={styles.body}>
				<p>
					<strong>Soil temperature</strong> is measured at {depthCm} cm by the
					NOAA station — the depth weed and grass seed germinate at. Hourly
					readings become a daily mean, then a trailing 5-day mean. A threshold
					only counts once the 5-day mean has stayed past it for{" "}
					<strong>5 days in a row</strong>, which filters out February thaws and
					September cold snaps.
				</p>
				<p>
					<strong>Typical dates</strong> come from finding each trigger
					separately in every year on record ({span}) and taking the median; the
					range shown is the earliest and latest year.{" "}
					<strong>This season</strong> shows where the trigger actually stands:{" "}
					<em>observed</em> in station data, <em>tentative</em> (reached but not
					yet held 5 days), <em>forecast</em> (from the 5-day forecast run
					through an air-to-soil model), or still on its <em>typical</em> date.
					A trigger that is past its usual date but has not happened is marked
					overdue rather than given a made-up date.
				</p>
				<p>
					<strong>Freeze dates</strong> are the last spring and first fall days
					the station's minimum reached 32°F. Fall seeding closes a fixed number
					of days before the <em>typical</em> first freeze — enough for
					seedlings to establish — never on a forecast freeze, by which time it
					is already too late.
				</p>
				<p>
					<strong>Spring pre-emergent</strong> opens at 45°F, is ideal from
					50°F, and closes when soil holds 55°F (crabgrass germination) or 200
					growing degree days, whichever comes first.{" "}
					<strong>Fall pre-emergent</strong> opens as soil cools through 70°F
					(winter annuals such as <em>Poa annua</em> begin germinating), is late
					below 65°F and closed at 55°F.
				</p>
				<p>
					<strong>Seed and pre-emergent never share a season.</strong> Logging
					an overseeding marks that season's pre-emergent as skipped, and
					logging a pre-emergent blocks seeding. No rates are given — the
					product label governs.
				</p>
				<p style={styles.caveat}>
					The station is not your yard: a thin, sunny, south-facing lawn warms
					earlier in spring than the station's sod. Treat the dates as a guide
					and watch the soil temperature itself.
				</p>
			</div>
		</details>
	);
}

const styles: Record<string, React.CSSProperties> = {
	details: {
		marginTop: "1.25rem",
		backgroundColor: "#fff",
		borderRadius: 8,
		padding: "0.75rem 1rem",
		boxShadow: "0 1px 3px rgba(0,0,0,0.08)",
	},
	summary: {
		fontSize: "0.9rem",
		fontWeight: 600,
		color: "#2d3748",
		cursor: "pointer",
	},
	body: {
		fontSize: "0.85rem",
		color: "#4a5568",
		lineHeight: 1.6,
		maxWidth: 820,
	},
	caveat: { color: "#718096" },
};
