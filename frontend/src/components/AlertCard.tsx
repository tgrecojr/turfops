import type { Recommendation } from '../types';
import { SEVERITY_COLORS, SEVERITY_SYMBOLS } from '../types';

interface AlertCardProps {
  rec: Recommendation;
}

export default function AlertCard({ rec }: AlertCardProps) {
  const color = SEVERITY_COLORS[rec.severity];
  const symbol = SEVERITY_SYMBOLS[rec.severity];

  return (
    <div style={{ ...styles.card, borderLeftColor: color }}>
      <div style={styles.header}>
        <span style={{ ...styles.badge, backgroundColor: color }}>
          {symbol} {rec.severity}
        </span>
        <span style={styles.category}>{rec.category}</span>
      </div>
      <div style={styles.title}>{rec.title}</div>
      <div style={styles.description}>{rec.description}</div>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  card: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '0.8rem 1rem',
    marginBottom: '0.75rem',
    borderLeft: '4px solid',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    marginBottom: 4,
  },
  badge: {
    color: '#fff',
    fontSize: '0.7rem',
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: 10,
  },
  category: {
    fontSize: '0.75rem',
    color: '#718096',
  },
  title: {
    fontWeight: 600,
    fontSize: '0.9rem',
    color: '#2d3748',
    marginBottom: 2,
  },
  description: {
    fontSize: '0.82rem',
    color: '#4a5568',
    lineHeight: 1.4,
  },
};
