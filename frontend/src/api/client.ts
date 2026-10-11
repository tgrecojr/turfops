import type {
	Application,
	CalendarResponse,
	DashboardResponse,
	EnvironmentalSummary,
	FracClassLookup,
	GddSummary,
	HealthResponse,
	HistoricalData,
	LawnProfile,
	NitrogenBudget,
	Plant,
	PlantType,
	Recommendation,
	SeasonalPlan,
	Severity,
	SoilTempForecast,
	SoilTest,
	SoilTestSummary,
} from "../types";
import type { DiseaseRiskResponse } from "../types/disease";
import type {
	LoggedProductSuggestion,
	Product,
	ProductCategory,
	ProductCreated,
	ProductFacts,
	ProductForm,
	ProductRefreshed,
	ShoppingList,
	StockStatus,
} from "../types/inventory";
import { factsOf } from "../types/inventory";
import type { TimingResponse } from "../types/timing";
import { ApiError, sentenceForStatus } from "./errors";

const BASE = "/api/v1";
const DEFAULT_TIMEOUT_MS = 15_000;

/** The server's `{"error": …}` message, or a plain sentence when the body has none. */
function errorMessage(body: string, status: number): string {
	try {
		const parsed: unknown = JSON.parse(body);
		if (
			parsed &&
			typeof parsed === "object" &&
			"error" in parsed &&
			typeof parsed.error === "string" &&
			parsed.error
		) {
			return parsed.error;
		}
	} catch {
		// Not JSON (a proxy's HTML page, plain text): never shown to the owner.
	}
	return sentenceForStatus(status);
}

export async function fetchJson<T>(
	url: string,
	init?: RequestInit,
	timeoutMs = DEFAULT_TIMEOUT_MS,
): Promise<T> {
	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), timeoutMs);
	// @spec UI-API-005, UI-API-006
	// The caller's headers add to the JSON default; the caller's signal joins the timeout
	// rather than replacing it.
	const headers = new Headers(init?.headers);
	if (!headers.has("Content-Type"))
		headers.set("Content-Type", "application/json");
	const signal = init?.signal
		? AbortSignal.any([controller.signal, init.signal])
		: controller.signal;

	try {
		const res = await fetch(url, { ...init, headers, signal });
		// @spec UI-ERR-004, UI-API-003
		const text = await res.text();
		if (!res.ok) throw new ApiError(res.status, errorMessage(text, res.status));
		// 204, or any 2xx with nothing in it, resolves with no value.
		if (!text) return undefined as unknown as T;
		return JSON.parse(text) as T;
	} catch (e) {
		// @spec UI-ERR-008
		// Only our own timer is a timeout; a caller cancelling its request (a superseded
		// poll) is passed through as the abort it is.
		if (
			e instanceof DOMException &&
			e.name === "AbortError" &&
			controller.signal.aborted
		) {
			throw new ApiError(0, "The request timed out.", true);
		}
		throw e;
	} finally {
		clearTimeout(timer);
	}
}

// Health
export const getHealth = () => fetchJson<HealthResponse>(`${BASE}/health`);

/** The lawn's local date (the server's `TZ`), never the viewer's clock. */
// @spec UI-UTIL-005
export const getToday = () =>
	fetchJson<{ today: string; timezone: string | null }>(`${BASE}/today`);

// Dashboard
export const getDashboard = (signal?: AbortSignal) =>
	fetchJson<DashboardResponse>(`${BASE}/dashboard`, { signal });

// Profile
export const getProfile = () => fetchJson<LawnProfile>(`${BASE}/profile`);

/** An empty `soil_type` / `irrigation_type`, or a `lawn_size_sqft` of 0, clears the field. */
export const updateProfile = (
	data: Partial<Omit<LawnProfile, "soil_type" | "irrigation_type">> & {
		soil_type?: LawnProfile["soil_type"] | "";
		irrigation_type?: LawnProfile["irrigation_type"] | "";
	},
) =>
	fetchJson<LawnProfile>(`${BASE}/profile`, {
		method: "PUT",
		body: JSON.stringify(data),
	});

// Applications
/** `scope`: "turf" or "landscape"; omitted = both. Filtered by the server before paging. */
// @spec APP-LIST-003
export const getApplications = (
	type?: string,
	limit = 50,
	offset = 0,
	scope?: "turf" | "landscape",
) => {
	const params = new URLSearchParams({
		limit: String(limit),
		offset: String(offset),
	});
	if (type) params.set("type", type);
	if (scope) params.set("scope", scope);
	return fetchJson<Application[]>(`${BASE}/applications?${params}`);
};

export const createApplication = (data: {
	application_type: string;
	product_name?: string;
	application_date: string;
	rate_per_1000sqft?: number;
	coverage_sqft?: number;
	notes?: string;
	nitrogen_pct?: number;
	phosphorus_pct?: number;
	potassium_pct?: number;
	plant_id?: number | null;
	follow_up_date?: string | null;
	frac_classes?: string[];
	product_id?: number | null;
}) =>
	fetchJson<Application>(`${BASE}/applications`, {
		method: "POST",
		body: JSON.stringify(data),
	});

export const updateApplication = (
	id: number,
	data: {
		application_type: string;
		product_name?: string;
		application_date: string;
		rate_per_1000sqft?: number;
		coverage_sqft?: number;
		notes?: string;
		nitrogen_pct?: number;
		phosphorus_pct?: number;
		potassium_pct?: number;
		plant_id?: number | null;
		follow_up_date?: string | null;
		frac_classes?: string[];
		product_id?: number | null;
	},
) =>
	fetchJson<Application>(`${BASE}/applications/${id}`, {
		method: "PUT",
		body: JSON.stringify(data),
	});

export const deleteApplication = (id: number) =>
	fetchJson<void>(`${BASE}/applications/${id}`, { method: "DELETE" });

// Calendar
export const getCalendar = (year?: number, month?: number) => {
	const params = new URLSearchParams();
	if (year) params.set("year", String(year));
	if (month) params.set("month", String(month));
	const qs = params.toString();
	return fetchJson<CalendarResponse>(
		`${BASE}/applications/calendar${qs ? `?${qs}` : ""}`,
	);
};

// Environmental
export const getEnvironmental = () =>
	fetchJson<EnvironmentalSummary>(`${BASE}/environmental`);

export const refreshEnvironmental = () =>
	fetchJson<EnvironmentalSummary>(`${BASE}/environmental/refresh`, {
		method: "POST",
	});

export const getFracClasses = (product?: string) =>
	fetchJson<FracClassLookup>(
		`${BASE}/frac-classes${product ? `?product=${encodeURIComponent(product)}` : ""}`,
	);

// Recommendations
/** `includeInactive` also returns the ones the user has dismissed / addressed. */
export const getRecommendations = (includeInactive = false) =>
	fetchJson<Recommendation[]>(
		`${BASE}/recommendations${includeInactive ? "?include_inactive=true" : ""}`,
	);

/**
 * `severity` is what the recommendation showed when the user clicked, so an escalation
 * past it brings the alert back. Clearing both flags restores the recommendation.
 */
export const patchRecommendation = (
	id: string,
	data: { dismissed?: boolean; addressed?: boolean; severity?: Severity },
) =>
	fetchJson<{ id: string; dismissed: boolean; addressed: boolean }>(
		`${BASE}/recommendations/${encodeURIComponent(id)}`,
		{ method: "PATCH", body: JSON.stringify(data) },
	);

// GDD
export const getDiseaseRisk = (signal?: AbortSignal) =>
	fetchJson<DiseaseRiskResponse>(`${BASE}/disease-risk`, { signal });

export const getTimingWindows = (signal?: AbortSignal) =>
	fetchJson<TimingResponse>(`${BASE}/timing-windows`, { signal });

export const getGdd = (year?: number, signal?: AbortSignal) => {
	const params = year ? `?year=${year}` : "";
	return fetchJson<GddSummary>(`${BASE}/gdd${params}`, { signal });
};

// Historical trends
export const getHistorical = (range: "7d" | "30d" | "90d") =>
	fetchJson<HistoricalData>(`${BASE}/historical?range=${range}`);

// Nitrogen budget
export const getNitrogenBudget = (year?: number, signal?: AbortSignal) => {
	const params = year ? `?year=${year}` : "";
	return fetchJson<NitrogenBudget>(`${BASE}/nitrogen-budget${params}`, {
		signal,
	});
};

// Soil temperature forecast
export const getSoilTempForecast = (signal?: AbortSignal) =>
	fetchJson<SoilTempForecast>(`${BASE}/soil-temp-forecast`, { signal });

// Soil Tests
export const getSoilTests = () => fetchJson<SoilTest[]>(`${BASE}/soil-tests`);

type SoilTestData = {
	test_date: string;
	lab_name?: string;
	ph: number;
	buffer_ph?: number;
	phosphorus_ppm?: number;
	potassium_ppm?: number;
	calcium_ppm?: number;
	magnesium_ppm?: number;
	sulfur_ppm?: number;
	iron_ppm?: number;
	manganese_ppm?: number;
	zinc_ppm?: number;
	boron_ppm?: number;
	copper_ppm?: number;
	organic_matter_pct?: number;
	cec?: number;
	notes?: string;
};

export const createSoilTest = (data: SoilTestData) =>
	fetchJson<SoilTest>(`${BASE}/soil-tests`, {
		method: "POST",
		body: JSON.stringify(data),
	});

export const updateSoilTest = (id: number, data: SoilTestData) =>
	fetchJson<SoilTest>(`${BASE}/soil-tests/${id}`, {
		method: "PUT",
		body: JSON.stringify(data),
	});

export const deleteSoilTest = (id: number) =>
	fetchJson<void>(`${BASE}/soil-tests/${id}`, { method: "DELETE" });

export const getSoilTestRecommendations = () =>
	fetchJson<SoilTestSummary>(`${BASE}/soil-tests/recommendations`);

// Seasonal plan
export const getSeasonalPlan = (year?: number) => {
	const params = year ? `?year=${year}` : "";
	return fetchJson<SeasonalPlan>(
		`${BASE}/seasonal-plan${params}`,
		undefined,
		30_000,
	);
};

// Plants (landscape maintenance)
export const listPlants = () => fetchJson<Plant[]>(`${BASE}/plants`);

export const getPlant = (id: number) =>
	fetchJson<Plant>(`${BASE}/plants/${id}`);

type CreatePlantBody = {
	common_name?: string;
	scientific_name?: string;
	plant_type: PlantType;
	location?: string;
	planting_date?: string;
	notes?: string;
};

export const createPlant = (data: CreatePlantBody) =>
	// LLM round-trip can take a while — extend the client-side timeout.
	fetchJson<Plant>(
		`${BASE}/plants`,
		{
			method: "POST",
			body: JSON.stringify(data),
		},
		60_000,
	);

type UpdatePlantBody = {
	common_name?: string;
	scientific_name?: string | null;
	plant_type?: PlantType;
	location?: string | null;
	planting_date?: string | null;
	notes?: string | null;
};

export const updatePlant = (id: number, data: UpdatePlantBody) =>
	fetchJson<Plant>(`${BASE}/plants/${id}`, {
		method: "PUT",
		body: JSON.stringify(data),
	});

export const deletePlant = (id: number) =>
	fetchJson<void>(`${BASE}/plants/${id}`, { method: "DELETE" });

export const refreshPlantPlan = (id: number) =>
	fetchJson<Plant>(
		`${BASE}/plants/${id}/refresh-plan`,
		{ method: "POST" },
		60_000,
	);

// Products (inventory)
export const listProducts = (opts?: {
	category?: ProductCategory;
	includeArchived?: boolean;
}) => {
	const params = new URLSearchParams();
	if (opts?.category) params.set("category", opts.category);
	if (opts?.includeArchived) params.set("include_archived", "true");
	const qs = params.toString();
	return fetchJson<Product[]>(`${BASE}/products${qs ? `?${qs}` : ""}`);
};

export type CreateProductBody = {
	name: string;
	brand?: string;
	category?: ProductCategory;
	form?: ProductForm;
	stock_status?: StockStatus;
	notes?: string;
	assist?: boolean;
};

export const createProduct = (data: CreateProductBody) =>
	// The assistant round-trip can take a while — extend the client-side timeout.
	fetchJson<ProductCreated>(
		`${BASE}/products`,
		{ method: "POST", body: JSON.stringify(data) },
		60_000,
	);

export type UpdateProductBody = ProductFacts & {
	name: string;
	brand: string | null;
	stock_status: StockStatus;
	notes: string | null;
	archived: boolean;
};

export const updateProduct = (id: number, data: UpdateProductBody) =>
	fetchJson<Product>(`${BASE}/products/${id}`, {
		method: "PUT",
		body: JSON.stringify(data),
	});

export const deleteProduct = (id: number) =>
	fetchJson<void>(`${BASE}/products/${id}`, { method: "DELETE" });

export const refreshProductProfile = (id: number) =>
	fetchJson<ProductRefreshed>(
		`${BASE}/products/${id}/refresh-profile`,
		{ method: "POST" },
		60_000,
	);

export const listLoggedProductSuggestions = () =>
	fetchJson<LoggedProductSuggestion[]>(`${BASE}/inventory/suggestions`);

export const linkProductApplications = (id: number) =>
	fetchJson<{ linked: number }>(`${BASE}/products/${id}/link-applications`, {
		method: "POST",
	});

/** Flip a product back to In stock (after logging an application against an Out one). */
export const markProductInStock = (p: Product) =>
	p.id == null
		? Promise.resolve(p)
		: updateProduct(p.id, {
				...factsOf(p),
				name: p.name,
				brand: p.brand,
				stock_status: "InStock",
				notes: p.notes,
				archived: p.archived,
			});

export const getShoppingList = () =>
	fetchJson<ShoppingList>(`${BASE}/inventory/shopping-list`, undefined, 30_000);
