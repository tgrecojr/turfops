import type { SoilTempPrediction, ThresholdPrediction } from '../types';
import { PREDICTION_CONFIDENCE_COLORS } from '../types';

interface SoilTempForecastWidgetProps {
  crossings: ThresholdPrediction[];
  predictions: SoilTempPrediction[];
  currentSoilTemp: number | null;
}

// A Falling crossing is only "meaningful" if the forecast stays below the
// threshold at the end of the window. Otherwise it's a transient dip and
// shouldn't replace the next agronomic milestone as the headline.
function isMeaningfulCrossing(
  crossing: ThresholdPrediction,
  predictions: SoilTempPrediction[],
): boolean {
  if (crossing.direction === 'Rising') return true;
  if (predictions.length === 0) return true;
  const last = predictions[predictions.length - 1];
  return last.predicted_soil_temp_f < crossing.threshold_temp_f;
}

function describeCrossing(c: ThresholdPrediction): string {
  return c.direction === 'Rising'
    ? `until soil reaches ${c.threshold_temp_f}°F`
    : `until soil drops below ${c.threshold_temp_f}°F`;
}

export default function SoilTempForecastWidget({
  crossings,
  predictions,
  currentSoilTemp,
}: SoilTempForecastWidgetProps) {
  const upcoming = crossings.filter(
    (c) => c.days_until_crossing > 0 && isMeaningfulCrossing(c, predictions),
  );

  if (upcoming.length === 0) {
    return (
      <div style={styles.card}>
        <div style={styles.label}>Soil Temp Forecast</div>
        <div style={styles.noData}>
          {currentSoilTemp != null
            ? `Current: ${currentSoilTemp.toFixed(1)}°F — No threshold crossings predicted`
            : 'Prediction data unavailable'}
        </div>
      </div>
    );
  }

  const nearest = upcoming[0];
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
        <span style={styles.unit}>{describeCrossing(nearest)}</span>
      </div>

      <div style={styles.detail}>{nearest.threshold_name}</div>

      <div style={styles.detail}>
        Est. {new Date(nearest.estimated_crossing_date + 'T00:00:00').toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}
      </div>

      {currentSoilTemp != null && (
        <div style={styles.currentTemp}>
          Current soil: {currentSoilTemp.toFixed(1)}{'°F'}
        </div>
      )}

      {upcoming.length > 1 && (
        <div style={styles.moreList}>
          {upcoming.slice(1, 3).map((c, i) => (
            <div key={i} style={styles.moreItem}>
              ~{c.days_until_crossing}d {describeCrossing(c)} ({c.threshold_name})
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
    marginLeft: 6,
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
