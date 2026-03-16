import type { ThresholdPrediction } from '../types';
import { PREDICTION_CONFIDENCE_COLORS } from '../types';

interface SoilTempForecastWidgetProps {
  crossings: ThresholdPrediction[];
  currentSoilTemp: number | null;
}

export default function SoilTempForecastWidget({
  crossings,
  currentSoilTemp,
}: SoilTempForecastWidgetProps) {
  // Show the nearest actionable crossing
  const upcoming = crossings.filter((c) => c.days_until_crossing > 0);

  if (upcoming.length === 0) {
    return (
      <div style={styles.card}>
        <div style={styles.label}>Soil Temp Forecast</div>
        <div style={styles.noData}>
          {currentSoilTemp != null
            ? `Current: ${currentSoilTemp.toFixed(1)}\u00B0F \u2014 No threshold crossings predicted`
            : 'Prediction data unavailable'}
        </div>
      </div>
    );
  }

  const nearest = upcoming[0];
  const directionArrow = nearest.direction === 'Rising' ? '\u2191' : '\u2193';
  const confColor = PREDICTION_CONFIDENCE_COLORS[nearest.confidence];

  return (
    <div style={styles.card}>
      <div style={styles.header}>
        <div style={styles.label}>Soil Temp Forecast</div>
        <span
          style={{
            ...styles.confBadge,
            backgroundColor: confColor + '22',
            color: confColor,
            borderColor: confColor,
          }}
        >
          {nearest.confidence}
        </span>
      </div>

      <div style={styles.mainRow}>
        <span style={styles.bigValue}>~{nearest.days_until_crossing}d</span>
        <span style={styles.unit}>until {nearest.threshold_temp_f}\u00B0F</span>
      </div>

      <div style={styles.detail}>
        {directionArrow} {nearest.threshold_name}
      </div>

      <div style={styles.detail}>
        Est. {new Date(nearest.estimated_crossing_date + 'T00:00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}
      </div>

      {currentSoilTemp != null && (
        <div style={styles.currentTemp}>
          Current soil: {currentSoilTemp.toFixed(1)}{'\u00B0F'}
        </div>
      )}

      {upcoming.length > 1 && (
        <div style={styles.moreList}>
          {upcoming.slice(1, 3).map((c, i) => (
            <div key={i} style={styles.moreItem}>
              {c.direction === 'Rising' ? '\u2191' : '\u2193'}{' '}
              {c.threshold_temp_f}\u00B0F ({c.threshold_name}) ~{c.days_until_crossing}d
            </div>
          ))}
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
  confBadge: {
    display: 'inline-block',
    padding: '1px 8px',
    borderRadius: 10,
    fontSize: '0.7rem',
    fontWeight: 600,
    border: '1px solid',
  },
  mainRow: {
    marginBottom: 4,
  },
  bigValue: {
    fontSize: '1.8rem',
    fontWeight: 700,
    color: '#1a202c',
  },
  unit: {
    fontSize: '0.9rem',
    color: '#718096',
    marginLeft: 4,
  },
  detail: {
    fontSize: '0.8rem',
    color: '#4a5568',
    marginBottom: 2,
  },
  currentTemp: {
    fontSize: '0.7rem',
    color: '#a0aec0',
    marginTop: 6,
  },
  noData: {
    fontSize: '0.8rem',
    color: '#a0aec0',
    marginTop: 6,
  },
  moreList: {
    marginTop: 8,
    borderTop: '1px solid #edf2f7',
    paddingTop: 6,
  },
  moreItem: {
    fontSize: '0.7rem',
    color: '#718096',
    marginBottom: 2,
  },
};
