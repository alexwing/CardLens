import { useCallback, useRef, useState } from 'react';
import type { ChangeEvent } from 'react';
import type { ScanResponse } from '../lib/types';
import { scanImage } from '../lib/api';
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
  const [mode, setMode] = useState<ScanMode>('camera');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ScanResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const analyze = useCallback(async (blob: Blob) => {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const response = await scanImage(blob);
      setResult(response);
    } catch {
      setError('No se pudo analizar la imagen. Comprueba que la API está en marcha e inténtalo de nuevo.');
    } finally {
      setLoading(false);
    }
  }, []);

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
        <h1>Escanear carta</h1>
        <p className="page-subtitle">Identifica una carta Pokémon con una foto.</p>
      </header>

      <div className="tabs" role="tablist" aria-label="Modo de captura">
        <button
          type="button"
          role="tab"
          aria-selected={mode === 'camera'}
          className={`tab${mode === 'camera' ? ' active' : ''}`}
          onClick={() => setMode('camera')}
        >
          Cámara
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={mode === 'upload'}
          className={`tab${mode === 'upload' ? ' active' : ''}`}
          onClick={() => setMode('upload')}
        >
          Subir
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
            Hacer foto o elegir imagen
          </button>
          <p className="hint">
            En el móvil se abrirá la cámara nativa. También puedes elegir una foto de la galería.
          </p>
        </div>
      )}

      {loading && (
        <div className="loading" role="status">
          <div className="spinner" aria-hidden="true" />
          <p>Analizando la carta…</p>
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
