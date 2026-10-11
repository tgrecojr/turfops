import { useEffect, useState } from "react";
import { getToday } from "../api/client";

/** The lawn's local date from the server, or null until (or unless) it arrives. */
// @spec UI-UTIL-005
export function useServerToday(): string | null {
	const [today, setToday] = useState<string | null>(null);
	useEffect(() => {
		let cancelled = false;
		getToday()
			.then((r) => {
				if (!cancelled) setToday(r.today);
			})
			// No server date: no warning, and the save is never blocked.
			.catch(() => undefined);
		return () => {
			cancelled = true;
		};
	}, []);
	return today;
}
