import { useCallback, useRef, useState } from 'react';
import type { ChangeEvent } from 'react';
import type { ScanResponse } from '../lib/types';
import { scanImage } from '../lib/api';
import { useT } from '../lib/i18n';
import CameraCapture from '../components/CameraCapture';
import ResultPanel from '../components/ResultPanel';

type ScanMode = 'camera' | 'upload';

/**
 * Pagina de escaneo con dos modos:
 *  - Camara: video en vivo via getUserMedia.
 *  - Subir: input de fichero con capture=environment, que en moviles abre
 *    la camara nativa y funciona incluso sin getUserMedia (p. ej. sin HTTPS).
 */
export default function ScanPage() {
  const { t } = useT();
  const [mode, setMode] = useState<ScanMode>('camera');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ScanResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const analyze = useCallback(
    async (blob: Blob) => {
      setLoading(true);
      setError(null);
      setResult(null);
      try {
        const response = await scanImage(blob);
        setResult(response);
      } catch {
        setError(t('scan.error'));
      } finally {
        setLoading(false);
      }
    },
    [t],
  );

  function handleFileChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) {
      void analyze(file);
    }
    // Permite volver a seleccionar el mismo archivo.
    event.target.value = '';
  }

  return (
    <div className="page scan-page">
      <header className="page-header">
        <h1>{t('scan.title')}</h1>
        <p className="page-subtitle">{t('scan.subtitle')}</p>
      </header>

      <div className="tabs" role="tablist" aria-label={t('scan.captureMode')}>
        <button
          type="button"
          role="tab"
          aria-selected={mode === 'camera'}
          className={`tab${mode === 'camera' ? ' active' : ''}`}
          onClick={() => setMode('camera')}
        >
          {t('scan.tab.camera')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={mode === 'upload'}
          className={`tab${mode === 'upload' ? ' active' : ''}`}
          onClick={() => setMode('upload')}
        >
          {t('scan.tab.upload')}
        </button>
      </div>

      {mode === 'camera' ? (
        <CameraCapture onCapture={(blob) => void analyze(blob)} disabled={loading} />
      ) : (
        <div className="upload-panel">
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            capture="environment"
            onChange={handleFileChange}
            className="visually-hidden"
            id="scan-file-input"
          />
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => fileInputRef.current?.click()}
            disabled={loading}
          >
            {t('scan.uploadButton')}
          </button>
          <p className="hint">{t('scan.uploadHint')}</p>
        </div>
      )}

      {loading && (
        <div className="loading" role="status">
          <div className="spinner" aria-hidden="true" />
          <p>{t('scan.analyzing')}</p>
        </div>
      )}

      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}

      {!loading && result && <ResultPanel key={result.scan_id} result={result} />}
    </div>
  );
}
