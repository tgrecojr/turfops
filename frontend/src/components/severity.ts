// A recommendation's severity as text and colors: symbol and label in ink on a tint of
// its status color, never the color alone.

import type { Severity } from "../types";
import { SEVERITY_COLORS, SEVERITY_SYMBOLS } from "../types";

/** Dark text for each severity, readable on its tint. */
const SEVERITY_INK: Record<Severity, string> = {
	Info: "#4a5568",
	Advisory: "#1e4e8c",
	Warning: "#744210",
	Critical: "#9b2c2c",
};

export interface SeverityChip {
	text: string;
	color: string;
	backgroundColor: string;
}

// @spec INV-UI-011
export function severityChip(severity: Severity): SeverityChip {
	return {
		text: `${SEVERITY_SYMBOLS[severity]} ${severity}`,
		color: SEVERITY_INK[severity],
		backgroundColor: `${SEVERITY_COLORS[severity]}22`,
	};
}
