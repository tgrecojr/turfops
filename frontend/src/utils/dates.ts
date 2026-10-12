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

/** The warning for a form date after the server's today (YYYY-MM-DD), or null. The save
 * is never blocked: a future entry is a plan, and models ignore it until its day. */
// @spec UI-UTIL-005
export function futureDateWarning(
	date: string,
	serverToday: string | null,
): string | null {
	if (!serverToday || !date || date <= serverToday) return null;
	return `This date is after today (${formatMonthDay(serverToday)}) on the lawn's calendar. It will be saved, but it won't count until that day.`;
}

// Display helpers for API calendar dates (YYYY-MM-DD) and month-days (MM-DD). They read
// the digits, never a Date at local midnight, so no timezone can shift the day.
// @spec UI-UTIL-006

const MONTHS = [
	"Jan",
	"Feb",
	"Mar",
	"Apr",
	"May",
	"Jun",
	"Jul",
	"Aug",
	"Sep",
	"Oct",
	"Nov",
	"Dec",
];

/** `2026-09-18` → `9/18`. */
export function formatShortDate(iso: string): string {
	const [, month, day] = iso.split("-");
	return `${Number(month)}/${Number(day)}`;
}

/** `2026-09-18` → `Sep 18`. */
export function formatMonthDay(iso: string): string {
	const [, month, day] = iso.split("-");
	return `${MONTHS[Number(month) - 1]} ${Number(day)}`;
}

/** `2026-03-23` → `Mar 23, 2026`. */
export function formatFullDate(iso: string): string {
	return `${formatMonthDay(iso)}, ${iso.slice(0, 4)}`;
}

/** `2026-09-18`, `2026-10-02` → `Sep 18 – Oct 2`. */
export function formatDateRange(start: string, end: string): string {
	return `${formatMonthDay(start)} – ${formatMonthDay(end)}`;
}

/** A care-plan month-day `03-15` → `Mar 15` (unparseable input comes back as is). */
export function formatMmDd(mmdd: string): string {
	const [month, day] = mmdd.split("-").map(Number);
	if (!month || !day || month > 12) return mmdd;
	return `${MONTHS[month - 1]} ${day}`;
}

/** The viewer's local date of an instant (a timestamp such as `updated_at`). */
export function formatTimestampDate(instant: string): string {
	const d = new Date(instant);
	return Number.isNaN(d.getTime()) ? instant : d.toLocaleDateString();
}
