import { useCallback, useEffect, useRef, useState } from 'react';
import { useT } from '../lib/i18n';
import type { TranslationKey } from '../lib/locales/es';

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
  const { t } = useT();
  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  // Guardamos la CLAVE del error (no el texto) para que el mensaje cambie de
  // idioma reactivamente sin reiniciar la camara.
  const [errorKey, setErrorKey] = useState<TranslationKey | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function startCamera() {
      if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
        setErrorKey('camera.unavailable');
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
          setErrorKey('camera.denied');
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

  if (errorKey) {
    return (
      <div className="camera-error" role="alert">
        <p>{t(errorKey)}</p>
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
        {ready ? t('camera.capture') : t('camera.starting')}
      </button>
      <p className="hint">{t('camera.hint')}</p>
    </div>
  );
}
