// One dashboard poll: the composite dashboard plus the feature widgets, each loaded
// with the poll's abort signal. A widget whose load fails keeps its last good value,
// marked stale.

export interface Widget<T> {
	value: T | null;
	/** The value is from an earlier poll: this poll's load failed. */
	stale: boolean;
}

export type Fetcher<T> = (signal?: AbortSignal) => Promise<T>;

/** The dashboard and each widget's loader. */
export interface DashboardFetchers<D, W extends Record<string, unknown>> {
	dashboard: Fetcher<D>;
	widgets: { [K in keyof W]: Fetcher<W[K]> };
}

export type Widgets<W> = { [K in keyof W]: Widget<W[K]> };

export async function loadDashboard<D, W extends Record<string, unknown>>(
	fetchers: DashboardFetchers<D, W>,
	signal: AbortSignal,
	previous: Widgets<W> | null,
): Promise<{ dashboard: D; widgets: Widgets<W> }> {
	// @spec UI-DASH-003, FEED-UI-013
	const keys = Object.keys(fetchers.widgets) as (keyof W)[];
	const failed = Symbol("failed");
	const [dashboard, ...values] = await Promise.all([
		fetchers.dashboard(signal),
		...keys.map((k) =>
			fetchers.widgets[k](signal).catch(() => failed as typeof failed),
		),
	]);
	const widgets = {} as Widgets<W>;
	keys.forEach((k, i) => {
		const value = values[i];
		widgets[k] =
			value === failed
				? {
						value: previous?.[k]?.value ?? null,
						stale: previous?.[k]?.value != null,
					}
				: { value: value as W[typeof k], stale: false };
	});
	return { dashboard: dashboard as D, widgets };
}
