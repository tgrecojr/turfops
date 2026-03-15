import {
  ResponsiveContainer,
  AreaChart,
  Area,
  ReferenceLine,
  YAxis,
} from 'recharts';
import type { GddSummary } from '../types';
import { CRABGRASS_STATUS_LABELS, CRABGRASS_STATUS_COLORS } from '../types';

interface GddWidgetProps {
  data: GddSummary;
}

export default function GddWidget({ data }: GddWidgetProps) {
  const { current_gdd_total, crabgrass_model, daily_history } = data;
  const status = crabgrass_model.status;

  const sparkData = daily_history.slice(-60).map((d) => ({
    gdd: d.cumulative_gdd_base50,
  }));

  return (
    <div style={styles.card}>
      <div style={styles.header}>
        <div style={styles.label}>GDD (Base 50°F)</div>
        <span
          style={{
            ...styles.statusBadge,
            backgroundColor: CRABGRASS_STATUS_COLORS[status] + '22',
            color: CRABGRASS_STATUS_COLORS[status],
            borderColor: CRABGRASS_STATUS_COLORS[status],
          }}
        >
          {CRABGRASS_STATUS_LABELS[status]}
        </span>
      </div>

      <div style={styles.valueRow}>
        <span style={styles.bigValue}>{current_gdd_total.toFixed(0)}</span>
        <span style={styles.target}> / 200</span>
      </div>

      <div style={styles.progressBarBg}>
        <div
          style={{
            ...styles.progressBarFill,
            width: `${Math.min((current_gdd_total / 200) * 100, 100)}%`,
            backgroundColor: CRABGRASS_STATUS_COLORS[status],
          }}
        />
      </div>

      {sparkData.length > 5 && (
        <div style={styles.sparkContainer}>
          <ResponsiveContainer width="100%" height={50}>
            <AreaChart data={sparkData} margin={{ top: 2, right: 0, left: 0, bottom: 2 }}>
              <defs>
                <linearGradient id="gddGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#48bb78" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#48bb78" stopOpacity={0} />
                </linearGradient>
              </defs>
              <YAxis hide domain={[0, 'auto']} />
              <ReferenceLine y={200} stroke="#ef4444" strokeDasharray="3 2" />
              <Area
                type="monotone"
                dataKey="gdd"
                stroke="#48bb78"
                fill="url(#gddGrad)"
                strokeWidth={1.5}
                dot={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>
      )}

      <div style={styles.footnote}>
        Crabgrass germinates at ~200 GDD
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  card: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 6,
  },
  label: {
    fontSize: '0.75rem',
    fontWeight: 600,
    color: '#718096',
    textTransform: 'uppercase' as const,
  },
  statusBadge: {
    display: 'inline-block',
    padding: '1px 8px',
    borderRadius: 10,
    fontSize: '0.7rem',
    fontWeight: 600,
    border: '1px solid',
  },
  valueRow: {
    marginBottom: 6,
  },
  bigValue: {
    fontSize: '1.8rem',
    fontWeight: 700,
    color: '#1a202c',
  },
  target: {
    fontSize: '1rem',
    color: '#a0aec0',
    fontWeight: 500,
  },
  progressBarBg: {
    height: 6,
    borderRadius: 3,
    backgroundColor: '#edf2f7',
    overflow: 'hidden',
    marginBottom: 8,
  },
  progressBarFill: {
    height: '100%',
    borderRadius: 3,
    transition: 'width 0.3s ease',
  },
  sparkContainer: {
    marginTop: 4,
    marginBottom: 4,
  },
  footnote: {
    fontSize: '0.65rem',
    color: '#a0aec0',
    textAlign: 'center' as const,
  },
};
