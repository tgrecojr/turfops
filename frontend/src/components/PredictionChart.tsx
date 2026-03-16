import {
  ResponsiveContainer,
  ComposedChart,
  Area,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
  ReferenceLine,
} from 'recharts';
import type { SoilTempPrediction, TimeSeriesPoint } from '../types';

interface PredictionChartProps {
  recentSoilTemps: TimeSeriesPoint[];
  predictions: SoilTempPrediction[];
  height?: number;
}

const THRESHOLDS = [
  { temp: 50, label: 'Pre-Emergent', color: '#eab308' },
  { temp: 55, label: 'Crabgrass', color: '#f97316' },
  { temp: 60, label: 'Grub Control', color: '#ef4444' },
  { temp: 75, label: 'Heat Stress', color: '#dc2626' },
];

export default function PredictionChart({
  recentSoilTemps,
  predictions,
  height = 260,
}: PredictionChartProps) {
  // Build combined dataset: actual (solid) + predicted (dashed)
  const actualData = recentSoilTemps.slice(-168).map((p) => ({
    time: new Date(p.timestamp).getTime(),
    actual: p.value,
    predicted: null as number | null,
  }));

  const predData = predictions.map((p) => ({
    time: new Date(p.date + 'T12:00:00').getTime(),
    actual: null as number | null,
    predicted: p.predicted_soil_temp_f,
  }));

  // Bridge: connect last actual to first prediction
  if (actualData.length > 0 && predData.length > 0) {
    const lastActual = actualData[actualData.length - 1];
    predData.unshift({
      time: lastActual.time,
      actual: null,
      predicted: lastActual.actual,
    });
  }

  const chartData = [...actualData, ...predData].sort((a, b) => a.time - b.time);

  if (chartData.length === 0) {
    return (
      <div style={styles.container}>
        <div style={styles.label}>Soil Temperature Forecast</div>
        <div style={styles.empty}>No data available</div>
      </div>
    );
  }

  const formatDate = (tick: number) => {
    const d = new Date(tick);
    return `${d.getMonth() + 1}/${d.getDate()}`;
  };

  const formatTooltip = (tick: number) =>
    new Date(tick).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
    });

  return (
    <div style={styles.container}>
      <div style={styles.headerRow}>
        <div style={styles.label}>Soil Temperature Forecast (10cm)</div>
        <div style={styles.legend}>
          <span style={styles.legendItem}>
            <span style={{ ...styles.legendLine, backgroundColor: '#e67e22' }} />
            Actual
          </span>
          <span style={styles.legendItem}>
            <span style={{ ...styles.legendLine, backgroundColor: '#3b82f6', borderStyle: 'dashed' }} />
            Predicted
          </span>
        </div>
      </div>
      <ResponsiveContainer width="100%" height={height}>
        <ComposedChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
          <defs>
            <linearGradient id="gradActual" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#e67e22" stopOpacity={0.2} />
              <stop offset="95%" stopColor="#e67e22" stopOpacity={0} />
            </linearGradient>
            <linearGradient id="gradPredicted" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.15} />
              <stop offset="95%" stopColor="#3b82f6" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#edf2f7" />
          <XAxis
            dataKey="time"
            type="number"
            domain={['dataMin', 'dataMax']}
            tickFormatter={formatDate}
            tick={{ fontSize: 11, fill: '#718096' }}
            stroke="#e2e8f0"
          />
          <YAxis
            tick={{ fontSize: 11, fill: '#718096' }}
            stroke="#e2e8f0"
            width={45}
            label={{ value: '\u00B0F', angle: -90, position: 'insideLeft', fontSize: 11, fill: '#718096' }}
          />
          <Tooltip
            labelFormatter={(tick) => formatTooltip(Number(tick))}
            formatter={(value, name) => [
              `${Number(value).toFixed(1)}\u00B0F`,
              name === 'actual' ? 'Actual' : 'Predicted',
            ]}
            contentStyle={{ fontSize: '0.8rem', borderRadius: 6, border: '1px solid #e2e8f0' }}
          />
          {THRESHOLDS.map((t) => (
            <ReferenceLine
              key={t.temp}
              y={t.temp}
              stroke={t.color}
              strokeDasharray="5 3"
              strokeOpacity={0.5}
              label={{
                value: `${t.temp}\u00B0 ${t.label}`,
                position: 'insideTopRight',
                fontSize: 9,
                fill: t.color,
              }}
            />
          ))}
          <Area
            type="monotone"
            dataKey="actual"
            stroke="#e67e22"
            fill="url(#gradActual)"
            strokeWidth={2}
            dot={false}
            connectNulls={false}
          />
          <Line
            type="monotone"
            dataKey="predicted"
            stroke="#3b82f6"
            strokeWidth={2}
            strokeDasharray="6 3"
            dot={{ r: 3, fill: '#3b82f6' }}
            connectNulls={false}
          />
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '0.75rem 1rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  headerRow: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  label: {
    fontSize: '0.8rem',
    fontWeight: 600,
    color: '#4a5568',
  },
  legend: {
    display: 'flex',
    gap: 12,
    fontSize: '0.7rem',
    color: '#718096',
  },
  legendItem: {
    display: 'flex',
    alignItems: 'center',
    gap: 4,
  },
  legendLine: {
    width: 16,
    height: 2,
    display: 'inline-block',
    borderRadius: 1,
  },
  empty: {
    color: '#a0aec0',
    fontSize: '0.8rem',
    padding: '2rem 0',
    textAlign: 'center' as const,
  },
};
