// Calendar-date helpers. `toISOString()` reports the UTC date, which is already tomorrow
// from 8 pm Eastern — an application logged that evening would be stored a day late and
// ignored as future-dated by the disease and timing models.

function toLocalISO(d: Date): string {
	const month = String(d.getMonth() + 1).padStart(2, "0");
	const day = String(d.getDate()).padStart(2, "0");
	return `${d.getFullYear()}-${month}-${day}`;
}

/** Today's date (YYYY-MM-DD) on the viewer's own calendar. */
export function todayLocalISO(): string {
	return toLocalISO(new Date());
}

/** `dateStr` (YYYY-MM-DD) plus `days`, staying on the local calendar throughout. */
export function addDaysISO(dateStr: string, days: number): string {
	const d = new Date(`${dateStr}T00:00:00`);
	d.setDate(d.getDate() + days);
	return toLocalISO(d);
}
