/**
 * Barra de confianza con color semaforo:
 * verde >= 0.80, ambar >= 0.60, rojo por debajo.
 */
interface ConfidenceBarProps {
  value: number;
  showLabel?: boolean;
}

export function confidenceLevel(value: number): 'high' | 'medium' | 'low' {
  if (value >= 0.8) return 'high';
  if (value >= 0.6) return 'medium';
  return 'low';
}

export default function ConfidenceBar({ value, showLabel = true }: ConfidenceBarProps) {
  const clamped = Math.max(0, Math.min(1, value));
  const percent = Math.round(clamped * 100);
  const level = confidenceLevel(clamped);

  return (
    <div className="confidence">
      <div
        className="confidence-track"
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`Confianza ${percent}%`}
      >
        <div className={`confidence-fill confidence-${level}`} style={{ width: `${percent}%` }} />
      </div>
      {showLabel && <span className={`confidence-label confidence-text-${level}`}>{percent}%</span>}
    </div>
  );
}
