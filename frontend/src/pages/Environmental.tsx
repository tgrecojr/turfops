import { useCallback, useEffect, useRef, useState } from 'react';
import {
  getEnvironmental,
  refreshEnvironmental,
} from '../api/client';
import Gauge from '../components/Gauge';
import {
  SOIL_TEMP_GAUGE,
  AMBIENT_TEMP_GAUGE,
  HUMIDITY_GAUGE,
  SOIL_MOISTURE_GAUGE,
} from '../components/gaugeConfigs';
import { sharedStyles } from '../styles/shared';
import type { EnvironmentalSummary } from '../types';

const POLL_INTERVAL = 30_000;

export default function Environmental() {
  const [data, setData] = useState<EnvironmentalSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const fetchData = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const d = await getEnvironmental();
      if (!controller.signal.aborted) {
        setData(d);
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
};
