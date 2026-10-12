import { useCallback, useEffect, useState } from "react";
import {
	createSoilTest,
	deleteSoilTest,
	getSoilTestRecommendations,
	getSoilTests,
	updateSoilTest,
} from "../api/client";
import { describeError } from "../api/errors";
import AdvicePanel from "../components/soilTests/AdvicePanel";
import { deleteConfirmText } from "../components/soilTests/display";
import { useServerToday } from "../components/useServerToday";
import { sharedStyles } from "../styles/shared";
import type { SoilTest, SoilTestSummary } from "../types";
import { futureDateWarning, todayLocalISO } from "../utils/dates";

export default function SoilTests() {
	const [tests, setTests] = useState<SoilTest[]>([]);
	const [summary, setSummary] = useState<SoilTestSummary | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState("");
	const [formOpen, setFormOpen] = useState(false);
	const [editingId, setEditingId] = useState<number | null>(null);
	const [submitting, setSubmitting] = useState(false);

	// Form fields
	const [testDate, setTestDate] = useState(todayLocalISO());
	// @spec UI-UTIL-005
	const serverToday = useServerToday();
	const [labName, setLabName] = useState("");
	const [ph, setPh] = useState("");
	const [bufferPh, setBufferPh] = useState("");
	const [phosphorus, setPhosphorus] = useState("");
	const [potassium, setPotassium] = useState("");
	const [calcium, setCalcium] = useState("");
	const [magnesium, setMagnesium] = useState("");
	const [sulfur, setSulfur] = useState("");
	const [iron, setIron] = useState("");
	const [manganese, setManganese] = useState("");
	const [zinc, setZinc] = useState("");
	const [boron, setBoron] = useState("");
	const [copper, setCopper] = useState("");
	const [organicMatter, setOrganicMatter] = useState("");
	const [cec, setCec] = useState("");
	const [notes, setNotes] = useState("");

	const loadData = useCallback(async () => {
		try {
			setError("");
			const t = await getSoilTests();
			setTests(t);
			if (t.length > 0) {
				const s = await getSoilTestRecommendations();
				setSummary(s);
			} else {
				setSummary(null);
			}
		} catch (e) {
			// @spec SOIL-UI-013
			setError(describeError(e));
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		loadData();
	}, [loadData]);

	const resetForm = () => {
		setEditingId(null);
		setTestDate(todayLocalISO());
		setLabName("");
		setPh("");
		setBufferPh("");
		setPhosphorus("");
		setPotassium("");
		setCalcium("");
		setMagnesium("");
		setSulfur("");
		setIron("");
		setManganese("");
		setZinc("");
		setBoron("");
		setCopper("");
		setOrganicMatter("");
		setCec("");
		setNotes("");
	};

	const handleEdit = (t: SoilTest) => {
		setEditingId(t.id ?? null);
		setTestDate(t.test_date);
		setLabName(t.lab_name ?? "");
		setPh(String(t.ph));
		setBufferPh(t.buffer_ph != null ? String(t.buffer_ph) : "");
		setPhosphorus(t.phosphorus_ppm != null ? String(t.phosphorus_ppm) : "");
		setPotassium(t.potassium_ppm != null ? String(t.potassium_ppm) : "");
		setCalcium(t.calcium_ppm != null ? String(t.calcium_ppm) : "");
		setMagnesium(t.magnesium_ppm != null ? String(t.magnesium_ppm) : "");
		setSulfur(t.sulfur_ppm != null ? String(t.sulfur_ppm) : "");
		setIron(t.iron_ppm != null ? String(t.iron_ppm) : "");
		setManganese(t.manganese_ppm != null ? String(t.manganese_ppm) : "");
		setZinc(t.zinc_ppm != null ? String(t.zinc_ppm) : "");
		setBoron(t.boron_ppm != null ? String(t.boron_ppm) : "");
		setCopper(t.copper_ppm != null ? String(t.copper_ppm) : "");
		setOrganicMatter(
			t.organic_matter_pct != null ? String(t.organic_matter_pct) : "",
		);
		setCec(t.cec != null ? String(t.cec) : "");
		setNotes(t.notes ?? "");
		setFormOpen(true);
	};

	const handleSubmit = async (e: React.FormEvent) => {
		e.preventDefault();
		if (!ph) {
			setError("pH is required");
			return;
		}
		setSubmitting(true);
		setError("");
		try {
			const optNum = (v: string) => (v ? Number(v) : undefined);
			const data = {
				test_date: testDate,
				lab_name: labName || undefined,
				ph: Number(ph),
				buffer_ph: optNum(bufferPh),
				phosphorus_ppm: optNum(phosphorus),
				potassium_ppm: optNum(potassium),
				calcium_ppm: optNum(calcium),
				magnesium_ppm: optNum(magnesium),
				sulfur_ppm: optNum(sulfur),
				iron_ppm: optNum(iron),
				manganese_ppm: optNum(manganese),
				zinc_ppm: optNum(zinc),
				boron_ppm: optNum(boron),
				copper_ppm: optNum(copper),
				organic_matter_pct: optNum(organicMatter),
				cec: optNum(cec),
				notes: notes || undefined,
			};
			if (editingId) {
				await updateSoilTest(editingId, data);
			} else {
				await createSoilTest(data);
			}
			resetForm();
			setFormOpen(false);
			await loadData();
		} catch (e) {
			setError(describeError(e));
		} finally {
			setSubmitting(false);
		}
	};

	// @spec SOIL-UI-017: a deleted test can't be recovered and may be the one advised on
	const handleDelete = async (test: SoilTest) => {
		if (test.id == null || !confirm(deleteConfirmText(test))) return;
		try {
			await deleteSoilTest(test.id);
			await loadData();
		} catch (e) {
			setError(describeError(e));
		}
	};

	if (loading)
		return <div style={sharedStyles.loading}>Loading soil tests...</div>;

	return (
		<div>
			<div style={sharedStyles.headerRow}>
				<h1 style={sharedStyles.pageTitle}>Soil Tests</h1>
				<button
					type="button"
					onClick={() => {
						if (formOpen) {
							resetForm();
							setFormOpen(false);
						} else {
							setFormOpen(true);
						}
					}}
					style={styles.addBtn}
				>
					{formOpen ? "Cancel" : "+ Add Soil Test"}
				</button>
			</div>

			{error && <div style={sharedStyles.error}>{error}</div>}

			{formOpen && (
				<form
					onSubmit={handleSubmit}
					style={{ ...sharedStyles.card, marginBottom: "1.5rem" }}
				>
					<h3 style={sharedStyles.sectionTitle}>
						{editingId ? "Edit Soil Test" : "New Soil Test"}
					</h3>

					<div style={styles.formSection}>
						<h4 style={styles.formSectionTitle}>Core</h4>
						<div style={styles.formGrid}>
							<label style={styles.label}>
								Date *
								<input
									type="date"
									value={testDate}
									onChange={(e) => setTestDate(e.target.value)}
									required
									style={styles.input}
								/>
								{futureDateWarning(testDate, serverToday) && (
									<div
										role="note"
										style={{
											fontSize: "0.75rem",
											color: "#975a16",
											marginTop: 4,
										}}
									>
										⚠ {futureDateWarning(testDate, serverToday)}
									</div>
								)}
							</label>
							<label style={styles.label}>
								Lab Name
								<input
									type="text"
									value={labName}
									onChange={(e) => setLabName(e.target.value)}
									placeholder="e.g. Waypoint"
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								pH *
								<input
									type="number"
									step="0.1"
									min="0"
									max="14"
									value={ph}
									onChange={(e) => setPh(e.target.value)}
									required
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Buffer pH
								<input
									type="number"
									step="0.1"
									value={bufferPh}
									onChange={(e) => setBufferPh(e.target.value)}
									style={styles.input}
								/>
							</label>
						</div>
					</div>

					<div style={styles.formSection}>
						<h4 style={styles.formSectionTitle}>Macronutrients (ppm)</h4>
						<div style={styles.formGrid}>
							<label style={styles.label}>
								Phosphorus (P)
								<input
									type="number"
									step="0.1"
									value={phosphorus}
									onChange={(e) => setPhosphorus(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Potassium (K)
								<input
									type="number"
									step="0.1"
									value={potassium}
									onChange={(e) => setPotassium(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Calcium (Ca)
								<input
									type="number"
									step="0.1"
									value={calcium}
									onChange={(e) => setCalcium(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Magnesium (Mg)
								<input
									type="number"
									step="0.1"
									value={magnesium}
									onChange={(e) => setMagnesium(e.target.value)}
									style={styles.input}
								/>
							</label>
						</div>
					</div>

					<div style={styles.formSection}>
						<h4 style={styles.formSectionTitle}>Micronutrients (ppm)</h4>
						<div style={styles.formGrid}>
							<label style={styles.label}>
								Sulfur (S)
								<input
									type="number"
									step="0.1"
									value={sulfur}
									onChange={(e) => setSulfur(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Iron (Fe)
								<input
									type="number"
									step="0.1"
									value={iron}
									onChange={(e) => setIron(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Manganese (Mn)
								<input
									type="number"
									step="0.1"
									value={manganese}
									onChange={(e) => setManganese(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Zinc (Zn)
								<input
									type="number"
									step="0.01"
									value={zinc}
									onChange={(e) => setZinc(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Boron (B)
								<input
									type="number"
									step="0.01"
									value={boron}
									onChange={(e) => setBoron(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								Copper (Cu)
								<input
									type="number"
									step="0.01"
									value={copper}
									onChange={(e) => setCopper(e.target.value)}
									style={styles.input}
								/>
							</label>
						</div>
					</div>

					<div style={styles.formSection}>
						<h4 style={styles.formSectionTitle}>Other</h4>
						<div style={styles.formGrid}>
							<label style={styles.label}>
								Organic Matter %
								<input
									type="number"
									step="0.1"
									value={organicMatter}
									onChange={(e) => setOrganicMatter(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={styles.label}>
								CEC (meq/100g)
								<input
									type="number"
									step="0.1"
									value={cec}
									onChange={(e) => setCec(e.target.value)}
									style={styles.input}
								/>
							</label>
							<label style={{ ...styles.label, gridColumn: "1 / -1" }}>
								Notes
								<input
									type="text"
									value={notes}
									onChange={(e) => setNotes(e.target.value)}
									style={styles.input}
								/>
							</label>
						</div>
					</div>

					<button type="submit" disabled={submitting} style={styles.submitBtn}>
						{submitting
							? "Saving..."
							: editingId
								? "Update Soil Test"
								: "Save Soil Test"}
					</button>
				</form>
			)}

			{/* Recommendations Panel */}
			{summary && <AdvicePanel summary={summary} today={serverToday} />}

			{/* Historical Tests Table */}
			<h2 style={sharedStyles.sectionTitle}>Test History</h2>
			{tests.length === 0 ? (
				<div style={sharedStyles.empty}>
					No soil tests recorded yet. Add your first test above.
				</div>
			) : (
				<div style={{ overflowX: "auto" }}>
					<table style={sharedStyles.table}>
						<thead>
							<tr>
								<th style={sharedStyles.th}>Date</th>
								<th style={sharedStyles.th}>Lab</th>
								<th style={sharedStyles.th}>pH</th>
								<th style={sharedStyles.th}>P (ppm)</th>
								<th style={sharedStyles.th}>K (ppm)</th>
								<th style={sharedStyles.th}>Ca (ppm)</th>
								<th style={sharedStyles.th}>Mg (ppm)</th>
								<th style={sharedStyles.th}>OM %</th>
								<th style={sharedStyles.th}></th>
							</tr>
						</thead>
						<tbody>
							{tests.map((t) => (
								<tr key={t.id}>
									<td style={sharedStyles.td}>{t.test_date}</td>
									<td style={sharedStyles.td}>{t.lab_name ?? "-"}</td>
									<td style={sharedStyles.td}>{t.ph.toFixed(1)}</td>
									<td style={sharedStyles.td}>
										{t.phosphorus_ppm?.toFixed(0) ?? "-"}
									</td>
									<td style={sharedStyles.td}>
										{t.potassium_ppm?.toFixed(0) ?? "-"}
									</td>
									<td style={sharedStyles.td}>
										{t.calcium_ppm?.toFixed(0) ?? "-"}
									</td>
									<td style={sharedStyles.td}>
										{t.magnesium_ppm?.toFixed(0) ?? "-"}
									</td>
									<td style={sharedStyles.td}>
										{t.organic_matter_pct?.toFixed(1) ?? "-"}
									</td>
									<td
										style={{
											...sharedStyles.td,
											display: "flex",
											gap: "0.5rem",
										}}
									>
										<button
											type="button"
											onClick={() => handleEdit(t)}
											style={styles.editBtn}
										>
											Edit
										</button>
										<button
											type="button"
											onClick={() => handleDelete(t)}
											style={styles.deleteBtn}
										>
											Delete
										</button>
									</td>
								</tr>
							))}
						</tbody>
					</table>
				</div>
			)}
		</div>
	);
}

const styles: Record<string, React.CSSProperties> = {
	addBtn: {
		padding: "0.5rem 1rem",
		backgroundColor: "#38a169",
		color: "#fff",
		border: "none",
		borderRadius: 6,
		cursor: "pointer",
		fontWeight: 600,
		fontSize: "0.85rem",
	},
	submitBtn: {
		padding: "0.5rem 1.5rem",
		backgroundColor: "#3182ce",
		color: "#fff",
		border: "none",
		borderRadius: 6,
		cursor: "pointer",
		fontWeight: 600,
		fontSize: "0.85rem",
		marginTop: "0.5rem",
	},
	editBtn: {
		padding: "2px 8px",
		backgroundColor: "transparent",
		color: "#3182ce",
		border: "1px solid #3182ce",
		borderRadius: 4,
		cursor: "pointer",
		fontSize: "0.75rem",
	},
	deleteBtn: {
		padding: "2px 8px",
		backgroundColor: "transparent",
		color: "#e53e3e",
		border: "1px solid #e53e3e",
		borderRadius: 4,
		cursor: "pointer",
		fontSize: "0.75rem",
	},
	formSection: {
		marginBottom: "1rem",
	},
	formSectionTitle: {
		fontSize: "0.85rem",
		fontWeight: 600,
		color: "#4a5568",
		margin: "0 0 0.5rem",
		borderBottom: "1px solid #e2e8f0",
		paddingBottom: "0.25rem",
	},
	formGrid: {
		display: "grid",
		gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
		gap: "0.75rem",
	},
	label: {
		display: "flex",
		flexDirection: "column" as const,
		fontSize: "0.8rem",
		color: "#4a5568",
		fontWeight: 500,
		gap: "0.25rem",
	},
	input: {
		padding: "0.4rem 0.5rem",
		border: "1px solid #e2e8f0",
		borderRadius: 4,
		fontSize: "0.85rem",
	},
};
