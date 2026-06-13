import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * Captura con la camara del dispositivo (getUserMedia, camara trasera).
 * Pinta el fotograma actual en un canvas y emite un blob JPEG (calidad 0.9).
 * Si la camara no esta disponible (permiso denegado, sin HTTPS, navegador
 * antiguo), muestra un aviso claro y sugiere el modo Subir.
 */
interface CameraCaptureProps {
  onCapture: (blob: Blob) => void;
  disabled?: boolean;
}

export default function CameraCapture({ onCapture, disabled = false }: CameraCaptureProps) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function startCamera() {
      if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
        setError(
          'La cámara no está disponible en este navegador o en esta conexión (se necesita HTTPS o localhost). Usa la pestaña «Subir» para hacer la foto con la cámara nativa.'
        );
        return;
      }
      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: 'environment' },
          audio: false,
        });
        if (cancelled) {
          stream.getTracks().forEach((track) => track.stop());
          return;
        }
        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
        }
        setReady(true);
      } catch {
        if (!cancelled) {
          setError(
            'No se pudo acceder a la cámara. Comprueba que has concedido el permiso, o usa la pestaña «Subir» para elegir una foto.'
          );
        }
      }
    }

    void startCamera();

    return () => {
      cancelled = true;
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
        streamRef.current = null;
      }
    };
  }, []);

  const handleCapture = useCallback(() => {
    const video = videoRef.current;
    if (!video || video.videoWidth === 0) {
      return;
    }
    const canvas = document.createElement('canvas');
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    const context = canvas.getContext('2d');
    if (!context) {
      return;
    }
    context.drawImage(video, 0, 0, canvas.width, canvas.height);
    canvas.toBlob(
      (blob) => {
        if (blob) {
          onCapture(blob);
        }
      },
      'image/jpeg',
      0.9
    );
  }, [onCapture]);

  if (error) {
    return (
      <div className="camera-error" role="alert">
        <p>{error}</p>
      </div>
    );
  }

  return (
    <div className="camera-capture">
      <div className="camera-frame">
        <video ref={videoRef} autoPlay playsInline muted className="camera-video" />
        <div className="camera-guide" aria-hidden="true" />
      </div>
      <button
        type="button"
        className="btn btn-primary btn-capture"
        onClick={handleCapture}
        disabled={disabled || !ready}
      >
        {ready ? 'Capturar carta' : 'Iniciando cámara…'}
      </button>
      <p className="hint">Encuadra la carta dentro del marco y pulsa el botón.</p>
    </div>
  );
}
