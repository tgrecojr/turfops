import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
  ReferenceLine,
} from 'recharts';
import type { TimeSeriesPoint } from '../types';

interface TrendChartProps {
  data: TimeSeriesPoint[];
  label: string;
  unit: string;
  color: string;
  thresholdValue?: number;
  thresholdLabel?: string;
  height?: number;
}

export default function TrendChart({
  data,
  label,
  unit,
  color,
  thresholdValue,
  thresholdLabel,
  height = 200,
}: TrendChartProps) {
  if (data.length === 0) {
    return (
      <div style={styles.container}>
        <div style={styles.label}>{label}</div>
        <div style={styles.empty}>No data available</div>
      </div>
    );
  }

  const chartData = data.map((p) => ({
    time: new Date(p.timestamp).getTime(),
    value: p.value,
  }));

  const formatTime = (tick: number) => {
    const d = new Date(tick);
    return `${d.getMonth() + 1}/${d.getDate()}`;
  };

  const formatTooltipTime = (tick: number) => {
    return new Date(tick).toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div style={styles.container}>
      <div style={styles.label}>{label}</div>
      <ResponsiveContainer width="100%" height={height}>
        <AreaChart data={chartData} margin={{ top: 5, right: 10, left: 0, bottom: 5 }}>
          <defs>
            <linearGradient id={`grad-${color.replace('#', '')}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={color} stopOpacity={0.3} />
              <stop offset="95%" stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#edf2f7" />
          <XAxis
            dataKey="time"
            type="number"
            domain={['dataMin', 'dataMax']}
            tickFormatter={formatTime}
            tick={{ fontSize: 11, fill: '#718096' }}
            stroke="#e2e8f0"
          />
          <YAxis
            tick={{ fontSize: 11, fill: '#718096' }}
            stroke="#e2e8f0"
            width={45}
          />
          <Tooltip
            labelFormatter={(tick) => formatTooltipTime(Number(tick))}
            formatter={(value) => [`${Number(value).toFixed(1)} ${unit}`, label]}
            contentStyle={{ fontSize: '0.8rem', borderRadius: 6, border: '1px solid #e2e8f0' }}
          />
          {thresholdValue != null && (
            <ReferenceLine
              y={thresholdValue}
              stroke="#ef4444"
              strokeDasharray="5 3"
              label={{
                value: thresholdLabel || '',
                position: 'insideTopRight',
                fontSize: 10,
                fill: '#ef4444',
              }}
            />
          )}
          <Area
            type="monotone"
            dataKey="value"
            stroke={color}
            fill={`url(#grad-${color.replace('#', '')})`}
            strokeWidth={2}
            dot={false}
          />
        </AreaChart>
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
  label: {
    fontSize: '0.8rem',
    fontWeight: 600,
    color: '#4a5568',
    marginBottom: 8,
  },
  empty: {
    color: '#a0aec0',
    fontSize: '0.8rem',
    padding: '2rem 0',
    textAlign: 'center' as const,
  },
};
