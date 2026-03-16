import { useCallback, useEffect, useRef, useState } from 'react';
import {
  getEnvironmental,
  getHistorical,
  getSoilTempForecast,
  refreshEnvironmental,
} from '../api/client';
import Gauge from '../components/Gauge';
import PredictionChart from '../components/PredictionChart';
import TrendChart from '../components/TrendChart';
import {
  SOIL_TEMP_GAUGE,
  AMBIENT_TEMP_GAUGE,
  HUMIDITY_GAUGE,
  SOIL_MOISTURE_GAUGE,
} from '../components/gaugeConfigs';
import { sharedStyles } from '../styles/shared';
import type { EnvironmentalSummary, HistoricalData, SoilTempForecast } from '../types';
import { PREDICTION_CONFIDENCE_COLORS } from '../types';

const POLL_INTERVAL = 30_000;

type HistRange = '7d' | '30d' | '90d';

export default function Environmental() {
  const [data, setData] = useState<EnvironmentalSummary | null>(null);
  const [histData, setHistData] = useState<HistoricalData | null>(null);
  const [soilForecast, setSoilForecast] = useState<SoilTempForecast | null>(null);
  const [histRange, setHistRange] = useState<HistRange>('7d');
  const [histLoading, setHistLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const fetchData = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const [d, sf] = await Promise.all([
        getEnvironmental(),
        getSoilTempForecast().catch(() => null),
      ]);
      if (!controller.signal.aborted) {
        setData(d);
        setSoilForecast(sf);
        setError(null);
      }
    } catch (e) {
      if (!controller.signal.aborted) {
        setError(e instanceof Error ? e.message : 'Failed to load');
      }
    } finally {
      if (!controller.signal.aborted) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    fetchData();

    let intervalId: ReturnType<typeof setInterval> | null = null;

    const startPolling = () => {
      stopPolling();
      if (!document.hidden) {
        intervalId = setInterval(fetchData, POLL_INTERVAL);
      }
    };

    const stopPolling = () => {
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };

    const handleVisibility = () => {
      if (document.hidden) {
        stopPolling();
      } else {
        fetchData();
        startPolling();
      }
    };

    document.addEventListener('visibilitychange', handleVisibility);
    startPolling();

    return () => {
      stopPolling();
      abortRef.current?.abort();
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [fetchData]);

  // Fetch historical data when range changes
  useEffect(() => {
    let cancelled = false;
    setHistLoading(true);
    getHistorical(histRange)
      .then((d) => {
        if (!cancelled) {
          setHistData(d);
          setHistLoading(false);
        }
      })
      .catch(() => {
        if (!cancelled) setHistLoading(false);
      });
    return () => { cancelled = true; };
  }, [histRange]);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      const d = await refreshEnvironmental();
      setData(d);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Refresh failed');
    } finally {
      setRefreshing(false);
    }
  };

  if (loading) return <div style={sharedStyles.loading}>Loading...</div>;

  const current = data?.current;

  // Soil depth table
  const depthRows = [
    { depth: '5 cm', temp: current?.soil_temp_5_f, moisture: current?.soil_moisture_5 },
    { depth: '10 cm', temp: current?.soil_temp_10_f, moisture: current?.soil_moisture_10 },
    { depth: '20 cm', temp: current?.soil_temp_20_f, moisture: current?.soil_moisture_20 },
    { depth: '50 cm', temp: current?.soil_temp_50_f, moisture: current?.soil_moisture_50 },
    { depth: '100 cm', temp: current?.soil_temp_100_f, moisture: current?.soil_moisture_100 },
  ];

  return (
    <div>
      <div style={sharedStyles.headerRow}>
        <h1 style={sharedStyles.pageTitle}>Environmental Data</h1>
        <button
          style={styles.refreshBtn}
          onClick={handleRefresh}
          disabled={refreshing}
        >
          {refreshing ? 'Refreshing...' : 'Refresh Now'}
        </button>
      </div>

      {error && <div style={sharedStyles.error}>{error}</div>}

      {data?.last_updated && (
        <p style={styles.updated}>
          Last updated: {new Date(data.last_updated).toLocaleString()}
          {data.soil_temp_trend && ` | Soil temp trend: ${data.soil_temp_trend}`}
        </p>
      )}

      {/* Gauges */}
      <div style={sharedStyles.gaugeGrid}>
        <div style={sharedStyles.card}>
          <Gauge
            {...SOIL_TEMP_GAUGE}
            value={current?.soil_temp_10_f ?? null}
          />
        </div>
        <div style={sharedStyles.card}>
          <Gauge
            {...AMBIENT_TEMP_GAUGE}
            value={current?.ambient_temp_f ?? null}
          />
        </div>
        <div style={sharedStyles.card}>
          <Gauge
            {...HUMIDITY_GAUGE}
            value={current?.humidity_percent ?? null}
          />
        </div>
        <div style={sharedStyles.card}>
          <Gauge
            {...SOIL_MOISTURE_GAUGE}
            value={
              current?.soil_moisture_10 != null ? current.soil_moisture_10 * 100 : null
            }
          />
        </div>
      </div>

      {/* 7-day summary */}
      <div style={styles.summaryGrid}>
        <SummaryCard
          label="7-Day Soil Temp Avg"
          value={data?.soil_temp_7day_avg_f}
          unit={'\u00B0F'}
        />
        <SummaryCard
          label="7-Day Ambient Avg"
          value={data?.ambient_temp_7day_avg_f}
          unit={'\u00B0F'}
        />
        <SummaryCard
          label="7-Day Humidity Avg"
          value={data?.humidity_7day_avg}
          unit="%"
        />
        <SummaryCard
          label="7-Day Precipitation"
          value={data?.precipitation_7day_total_mm}
          unit="mm"
        />
      </div>

      {/* Soil depth table */}
      <h2 style={sharedStyles.sectionTitle}>Soil Profile by Depth</h2>
      <table style={{ ...sharedStyles.table, marginBottom: '1.5rem' }}>
        <thead>
          <tr>
            <th style={sharedStyles.th}>Depth</th>
            <th style={sharedStyles.th}>Temperature</th>
            <th style={sharedStyles.th}>Moisture</th>
          </tr>
        </thead>
        <tbody>
          {depthRows.map((row) => (
            <tr key={row.depth}>
              <td style={sharedStyles.td}>{row.depth}</td>
              <td style={sharedStyles.td}>
                {row.temp != null ? `${row.temp.toFixed(1)} \u00B0F` : '--'}
              </td>
              <td style={sharedStyles.td}>
                {row.moisture != null
                  ? `${(row.moisture * 100).toFixed(1)}%`
                  : '--'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {/* Forecast */}
      {data?.forecast && data.forecast.daily_summary.length > 0 && (
        <>
          <h2 style={sharedStyles.sectionTitle}>5-Day Forecast</h2>
          <div style={styles.forecastGrid}>
            {data.forecast.daily_summary.map((day) => (
              <div key={day.date} style={styles.forecastCard}>
                <div style={styles.forecastDate}>{day.date}</div>
                <div style={styles.forecastCondition}>
                  {day.dominant_condition}
                </div>
                <div style={styles.forecastTemp}>
                  {day.high_temp_f.toFixed(0)}{'\u00B0'} / {day.low_temp_f.toFixed(0)}{'\u00B0'}
                </div>
                <div style={styles.forecastDetail}>
                  Humidity: {day.avg_humidity.toFixed(0)}%
                </div>
                <div style={styles.forecastDetail}>
                  Precip: {day.total_precipitation_mm.toFixed(1)}mm (
                  {(day.max_precipitation_prob * 100).toFixed(0)}%)
                </div>
              </div>
            ))}
          </div>
        </>
      )}

      {/* Soil Temperature Forecast */}
      {soilForecast && (
        <>
          <h2 style={sharedStyles.sectionTitle}>Soil Temperature Forecast</h2>
          <div style={{ marginBottom: '1rem' }}>
            <PredictionChart
              recentSoilTemps={histData?.soil_temp_10_f ?? []}
              predictions={soilForecast.predictions}
            />
          </div>
          {soilForecast.threshold_crossings.length > 0 && (
            <div style={styles.crossingGrid}>
              {soilForecast.threshold_crossings.map((c, i) => (
                <div key={i} style={styles.crossingCard}>
                  <div style={styles.crossingHeader}>
                    <span style={styles.crossingName}>{c.threshold_name}</span>
                    <span
                      style={{
                        ...styles.crossingBadge,
                        backgroundColor: PREDICTION_CONFIDENCE_COLORS[c.confidence] + '22',
                        color: PREDICTION_CONFIDENCE_COLORS[c.confidence],
                        borderColor: PREDICTION_CONFIDENCE_COLORS[c.confidence],
                      }}
                    >
                      {c.confidence}
                    </span>
                  </div>
                  <div style={styles.crossingTemp}>
                    {c.direction === 'Rising' ? '\u2191' : '\u2193'} {c.threshold_temp_f}{'\u00B0F'}
                  </div>
                  <div style={styles.crossingDate}>
                    ~{c.days_until_crossing} days ({new Date(c.estimated_crossing_date + 'T00:00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric' })})
                  </div>
                </div>
              ))}
            </div>
          )}
          <div style={styles.modelInfo}>
            Model: R\u00B2={soilForecast.model_info.r_squared.toFixed(2)}, lag={soilForecast.model_info.lag_days}d, quality={soilForecast.model_info.quality}
          </div>
        </>
      )}

      {/* Historical Trends */}
      <div style={styles.trendHeader}>
        <h2 style={sharedStyles.sectionTitle}>Historical Trends</h2>
        <div style={styles.rangeButtons}>
          {(['7d', '30d', '90d'] as HistRange[]).map((r) => (
            <button
              key={r}
              style={{
                ...styles.rangeBtn,
                ...(histRange === r ? styles.rangeBtnActive : {}),
              }}
              onClick={() => setHistRange(r)}
            >
              {r}
            </button>
          ))}
        </div>
      </div>
      {histLoading ? (
        <p style={sharedStyles.loading}>Loading trends...</p>
      ) : histData ? (
        <div style={styles.chartGrid}>
          <TrendChart
            data={histData.soil_temp_10_f}
            label="Soil Temperature (10cm)"
            unit={'\u00B0F'}
            color="#e67e22"
            thresholdValue={55}
            thresholdLabel="Pre-emergent"
          />
          <TrendChart
            data={histData.ambient_temp_f}
            label="Ambient Temperature"
            unit={'\u00B0F'}
            color="#3498db"
            thresholdValue={85}
            thresholdLabel="Heat stress"
          />
          <TrendChart
            data={histData.humidity_percent}
            label="Humidity"
            unit="%"
            color="#9b59b6"
            thresholdValue={80}
            thresholdLabel="Disease risk"
          />
          <TrendChart
            data={histData.soil_moisture_10}
            label="Soil Moisture (10cm)"
            unit=""
            color="#27ae60"
            thresholdValue={0.10}
            thresholdLabel="Drought"
          />
          <TrendChart
            data={histData.precipitation_mm}
            label="Precipitation"
            unit="mm"
            color="#2c3e50"
          />
          <TrendChart
            data={histData.gdd_accumulation}
            label="GDD Accumulation (Base 50°F)"
            unit="GDD"
            color="#48bb78"
            thresholdValue={200}
            thresholdLabel="Crabgrass"
          />
        </div>
      ) : null}
    </div>
  );
}

function SummaryCard({
  label,
  value,
  unit,
}: {
  label: string;
  value: number | null | undefined;
  unit: string;
}) {
  return (
    <div style={styles.summaryCard}>
      <div style={styles.summaryLabel}>{label}</div>
      <div style={styles.summaryValue}>
        {value != null ? `${value.toFixed(1)} ${unit}` : '--'}
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  refreshBtn: {
    padding: '0.5rem 1rem',
    backgroundColor: '#3182ce',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    cursor: 'pointer',
    fontWeight: 600,
    fontSize: '0.85rem',
  },
  updated: { color: '#718096', fontSize: '0.8rem', marginBottom: '1rem' },
  summaryGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
    gap: '1rem',
    marginBottom: '1.5rem',
  },
  summaryCard: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '0.8rem 1rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    textAlign: 'center' as const,
  },
  summaryLabel: { fontSize: '0.75rem', color: '#718096', marginBottom: 4 },
  summaryValue: { fontSize: '1.2rem', fontWeight: 600, color: '#2d3748' },
  forecastGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(150px, 1fr))',
    gap: '0.75rem',
    marginBottom: '1.5rem',
  },
  forecastCard: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '0.8rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    textAlign: 'center' as const,
  },
  forecastDate: { fontSize: '0.8rem', fontWeight: 600, color: '#2d3748' },
  forecastCondition: { fontSize: '0.85rem', color: '#4a5568', margin: '4px 0' },
  forecastTemp: { fontSize: '1.1rem', fontWeight: 600, color: '#1a202c' },
  forecastDetail: { fontSize: '0.7rem', color: '#718096', marginTop: 2 },
  trendHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '0.75rem',
  },
  rangeButtons: {
    display: 'flex',
    gap: 4,
  },
  rangeBtn: {
    padding: '4px 12px',
    borderRadius: 6,
    border: '1px solid #e2e8f0',
    backgroundColor: '#fff',
    color: '#4a5568',
    fontSize: '0.75rem',
    fontWeight: 600,
    cursor: 'pointer',
  },
  rangeBtnActive: {
    backgroundColor: '#3182ce',
    color: '#fff',
    borderColor: '#3182ce',
  },
  chartGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(380px, 1fr))',
    gap: '1rem',
    marginBottom: '1.5rem',
  },
  crossingGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
    gap: '0.75rem',
    marginBottom: '0.75rem',
  },
  crossingCard: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '0.8rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  crossingHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 4,
  },
  crossingName: {
    fontSize: '0.75rem',
    fontWeight: 600,
    color: '#4a5568',
  },
  crossingBadge: {
    display: 'inline-block',
    padding: '1px 6px',
    borderRadius: 8,
    fontSize: '0.6rem',
    fontWeight: 600,
    border: '1px solid',
  },
  crossingTemp: {
    fontSize: '1.1rem',
    fontWeight: 700,
    color: '#1a202c',
  },
  crossingDate: {
    fontSize: '0.7rem',
    color: '#718096',
    marginTop: 2,
  },
  modelInfo: {
    fontSize: '0.65rem',
    color: '#a0aec0',
    textAlign: 'center' as const,
    marginBottom: '1.5rem',
  },
};
