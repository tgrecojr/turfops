interface GaugeProps {
  label: string;
  value: number | null;
  unit: string;
  min: number;
  max: number;
  thresholds?: { warn: number; critical: number };
}

export default function Gauge({
  label,
  value,
  unit,
  min,
  max,
  thresholds,
}: GaugeProps) {
  const pct =
    value !== null ? Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100)) : 0;

  let barColor = '#48bb78'; // green
  if (value !== null && thresholds) {
    if (value >= thresholds.critical) barColor = '#fc8181';
    else if (value >= thresholds.warn) barColor = '#ecc94b';
  }

  return (
    <div style={styles.container}>
      <div style={styles.header}>
        <span style={styles.label}>{label}</span>
        <span style={styles.value}>
          {value !== null ? value.toFixed(1) : '--'} {unit}
        </span>
      </div>
      <div style={styles.track}>
        <div
          style={{
            ...styles.bar,
            width: `${pct}%`,
            backgroundColor: barColor,
          }}
        />
      </div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: { marginBottom: '1rem' },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    marginBottom: 4,
    fontSize: '0.85rem',
  },
  label: { color: '#4a5568', fontWeight: 500 },
  value: { color: '#2d3748', fontWeight: 600 },
  track: {
    height: 8,
    backgroundColor: '#e2e8f0',
    borderRadius: 4,
    overflow: 'hidden',
  },
  bar: {
    height: '100%',
    borderRadius: 4,
    transition: 'width 0.4s ease',
  },
};
