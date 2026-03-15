export interface GaugeConfig {
  label: string;
  unit: string;
  min: number;
  max: number;
  thresholds: { warn: number; critical: number };
}

export const SOIL_TEMP_GAUGE: GaugeConfig = {
  label: 'Soil Temp (10cm)',
  unit: '\u00B0F',
  min: 30,
  max: 100,
  thresholds: { warn: 75, critical: 85 },
};

export const AMBIENT_TEMP_GAUGE: GaugeConfig = {
  label: 'Ambient Temp',
  unit: '\u00B0F',
  min: 0,
  max: 110,
  thresholds: { warn: 85, critical: 95 },
};

export const HUMIDITY_GAUGE: GaugeConfig = {
  label: 'Humidity',
  unit: '%',
  min: 0,
  max: 100,
  thresholds: { warn: 80, critical: 90 },
};

export const SOIL_MOISTURE_GAUGE: GaugeConfig = {
  label: 'Soil Moisture (10cm)',
  unit: '%',
  min: 0,
  max: 50,
  thresholds: { warn: 40, critical: 45 },
};
