const MM_PER_INCH = 25.4;

// @spec UI-UTIL-003
export function mmToInches(mm: number): number {
	return mm / MM_PER_INCH;
}

export function formatInches(mm: number, decimals = 2): string {
	return `${mmToInches(mm).toFixed(decimals)} in`;
}
