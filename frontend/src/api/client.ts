import type {
  Application,
  CalendarResponse,
  DashboardResponse,
  EnvironmentalSummary,
  GddSummary,
  HealthResponse,
  HistoricalData,
  LawnProfile,
  NitrogenBudget,
  Recommendation,
  SeasonalPlan,
  SoilTempForecast,
  SoilTest,
  SoilTestSummary,
} from '../types';

const BASE = '/api/v1';
const DEFAULT_TIMEOUT_MS = 15_000;

async function fetchJson<T>(
  url: string,
  init?: RequestInit,
  timeoutMs = DEFAULT_TIMEOUT_MS
): Promise<T> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const res = await fetch(url, {
      headers: { 'Content-Type': 'application/json' },
      signal: controller.signal,
      ...init,
    });
    if (!res.ok) {
      const body = await res.text();
      // Sanitize: strip potential stack traces/internal paths, limit length
      const sanitized = body.length > 200 ? body.slice(0, 200) + '...' : body;
      const safeMessage = sanitized.replace(/\/[^\s:]+\.(rs|js|ts):\d+/g, '[internal]');
      throw new Error(`${res.status}: ${safeMessage}`);
    }
    // 204 No Content
    if (res.status === 204) return undefined as unknown as T;
    return res.json();
  } catch (e) {
    if (e instanceof DOMException && e.name === 'AbortError') {
      throw new Error(`Request timed out after ${timeoutMs}ms`, { cause: e });
    }
    throw e;
  } finally {
    clearTimeout(timer);
  }
}

// Health
export const getHealth = () => fetchJson<HealthResponse>(`${BASE}/health`);

// Dashboard
export const getDashboard = () =>
  fetchJson<DashboardResponse>(`${BASE}/dashboard`);

// Profile
export const getProfile = () => fetchJson<LawnProfile>(`${BASE}/profile`);

export const updateProfile = (data: Partial<LawnProfile>) =>
  fetchJson<LawnProfile>(`${BASE}/profile`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });

// Applications
export const getApplications = (type?: string) => {
  const params = type ? `?type=${encodeURIComponent(type)}` : '';
  return fetchJson<Application[]>(`${BASE}/applications${params}`);
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
}) =>
  fetchJson<Application>(`${BASE}/applications`, {
    method: 'POST',
    body: JSON.stringify(data),
  });

export const deleteApplication = (id: number) =>
  fetchJson<void>(`${BASE}/applications/${id}`, { method: 'DELETE' });

// Calendar
export const getCalendar = (year?: number, month?: number) => {
  const params = new URLSearchParams();
  if (year) params.set('year', String(year));
  if (month) params.set('month', String(month));
  const qs = params.toString();
  return fetchJson<CalendarResponse>(
    `${BASE}/applications/calendar${qs ? `?${qs}` : ''}`
  );
};

// Environmental
export const getEnvironmental = () =>
  fetchJson<EnvironmentalSummary>(`${BASE}/environmental`);

export const refreshEnvironmental = () =>
  fetchJson<EnvironmentalSummary>(`${BASE}/environmental/refresh`, {
    method: 'POST',
  });

// Recommendations
export const getRecommendations = () =>
  fetchJson<Recommendation[]>(`${BASE}/recommendations`);

export const patchRecommendation = (
  id: string,
  data: { dismissed?: boolean; addressed?: boolean }
) =>
  fetchJson<{ id: string; dismissed: boolean; addressed: boolean }>(
    `${BASE}/recommendations/${encodeURIComponent(id)}`,
    { method: 'PATCH', body: JSON.stringify(data) }
  );

// GDD
export const getGdd = (year?: number) => {
  const params = year ? `?year=${year}` : '';
  return fetchJson<GddSummary>(`${BASE}/gdd${params}`);
};

// Historical trends
export const getHistorical = (range: '7d' | '30d' | '90d') =>
  fetchJson<HistoricalData>(`${BASE}/historical?range=${range}`);

// Nitrogen budget
export const getNitrogenBudget = (year?: number) => {
  const params = year ? `?year=${year}` : '';
  return fetchJson<NitrogenBudget>(`${BASE}/nitrogen-budget${params}`);
};

// Soil temperature forecast
export const getSoilTempForecast = () =>
  fetchJson<SoilTempForecast>(`${BASE}/soil-temp-forecast`);

// Soil Tests
export const getSoilTests = () =>
  fetchJson<SoilTest[]>(`${BASE}/soil-tests`);

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
    method: 'POST',
    body: JSON.stringify(data),
  });

export const updateSoilTest = (id: number, data: SoilTestData) =>
  fetchJson<SoilTest>(`${BASE}/soil-tests/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });

export const deleteSoilTest = (id: number) =>
  fetchJson<void>(`${BASE}/soil-tests/${id}`, { method: 'DELETE' });

export const getSoilTestRecommendations = () =>
  fetchJson<SoilTestSummary>(`${BASE}/soil-tests/recommendations`);

// Seasonal plan
export const getSeasonalPlan = (year?: number) => {
  const params = year ? `?year=${year}` : '';
  return fetchJson<SeasonalPlan>(`${BASE}/seasonal-plan${params}`, undefined, 30_000);
};
