import { sharedStyles } from "../../styles/shared";
import type { NutrientLevel } from "../../types";
import { NUTRIENT_LEVEL_COLORS } from "../../types";
import { levelWord } from "./display";

/** A nutrient level: its word in its status color, never the color alone. */
// @spec SOIL-UI-009
export default function LevelBadge({ level }: { level: NutrientLevel }) {
	const color = NUTRIENT_LEVEL_COLORS[level];
	return (
		<span
			style={{
				...sharedStyles.badge,
				backgroundColor: `${color}22`,
				color,
				borderColor: color,
			}}
		>
			{levelWord(level)}
		</span>
	);
}
