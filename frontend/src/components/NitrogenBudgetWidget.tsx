import type { NitrogenBudget } from '../types';

interface NitrogenBudgetWidgetProps {
  data: NitrogenBudget;
}

export default function NitrogenBudgetWidget({ data }: NitrogenBudgetWidgetProps) {
  const {
    applied_lbs_per_1000sqft,
    target_lbs_per_1000sqft,
    remaining_lbs_per_1000sqft,
    percent_of_target,
    applications,
    grass_type_target,
  } = data;

  const pct = Math.min(percent_of_target, 100);
  const barColor =
    percent_of_target > 100
      ? '#ef4444' // Over budget
      : percent_of_target > 80
        ? '#eab308' // Approaching
        : '#48bb78'; // On track

  return (
    <div style={styles.card}>
      <div style={styles.header}>
        <div style={styles.label}>Nitrogen Budget ({data.year})</div>
        <span style={{ ...styles.rangeBadge, color: barColor }}>
          {grass_type_target.min_lbs_per_1000sqft}-{grass_type_target.max_lbs_per_1000sqft} lbs/1k
        </span>
      </div>

      <div style={styles.valueRow}>
        <span style={styles.bigValue}>{applied_lbs_per_1000sqft.toFixed(1)}</span>
        <span style={styles.target}>
          {' '}
          / {target_lbs_per_1000sqft.toFixed(1)} lbs N/1k sqft
        </span>
      </div>

      <div style={styles.progressBarBg}>
        <div
          style={{
            ...styles.progressBarFill,
            width: `${pct}%`,
            backgroundColor: barColor,
          }}
        />
      </div>

      <div style={styles.remainingRow}>
        {percent_of_target > 100 ? (
          <span style={{ color: '#ef4444', fontSize: '0.8rem', fontWeight: 600 }}>
            Over budget by {(applied_lbs_per_1000sqft - target_lbs_per_1000sqft).toFixed(1)} lbs
          </span>
        ) : (
          <span style={{ color: '#718096', fontSize: '0.8rem' }}>
            Remaining: {remaining_lbs_per_1000sqft.toFixed(1)} lbs N/1k sqft
          </span>
        )}
      </div>

      {applications.length > 0 && (
        <div style={styles.appList}>
          {applications.map((app, i) => (
            <div key={i} style={styles.appRow}>
              <span style={styles.appDate}>{app.date}</span>
              <span style={styles.appProduct}>{app.product_name || 'Unknown'}</span>
              <span style={styles.appN}>+{app.n_lbs_per_1000sqft.toFixed(2)}</span>
            </div>
          ))}
        </div>
      )}

      {applications.length === 0 && (
        <div style={styles.noApps}>
          No nitrogen applications recorded yet. Add N-P-K data when logging applications.
        </div>
      )}
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
  rangeBadge: {
    fontSize: '0.7rem',
    fontWeight: 600,
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
    fontSize: '0.85rem',
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
  remainingRow: {
    marginBottom: 8,
  },
  appList: {
    borderTop: '1px solid #edf2f7',
    paddingTop: 8,
  },
  appRow: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '3px 0',
    fontSize: '0.75rem',
  },
  appDate: {
    color: '#718096',
    minWidth: 80,
  },
  appProduct: {
    color: '#4a5568',
    flex: 1,
    paddingLeft: 8,
  },
  appN: {
    color: '#48bb78',
    fontWeight: 600,
    minWidth: 50,
    textAlign: 'right' as const,
  },
  noApps: {
    color: '#a0aec0',
    fontSize: '0.75rem',
    borderTop: '1px solid #edf2f7',
    paddingTop: 8,
  },
};
