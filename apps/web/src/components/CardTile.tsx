import type { Card } from '../lib/types';
import { imageSrc } from '../lib/api';
import { useT } from '../lib/i18n';

/**
 * Miniatura de carta reutilizable: imagen, nombre, set y numero.
 * Si recibe onClick se comporta como boton seleccionable.
 */
interface CardTileProps {
  card: Card;
  confidence?: number;
  selected?: boolean;
  onClick?: () => void;
}

export default function CardTile({ card, confidence, selected = false, onClick }: CardTileProps) {
  const { t } = useT();
  const src = imageSrc(card);
  const className = `card-tile${selected ? ' selected' : ''}${onClick ? ' clickable' : ''}`;

  const body = (
    <>
      {src ? (
        <img className="card-tile-image" src={src} alt={card.name} loading="lazy" />
      ) : (
        <div className="card-tile-placeholder" aria-hidden="true">
          ?
        </div>
      )}
      <div className="card-tile-info">
        <span className="card-tile-name">{card.name}</span>
        <span className="card-tile-meta">
          {t('common.setNumber', { set: card.set_name ?? card.set_id, number: card.number })}
        </span>
        {confidence !== undefined && (
          <span className="card-tile-confidence">{Math.round(confidence * 100)}%</span>
        )}
      </div>
    </>
  );

  if (onClick) {
    return (
      <button type="button" className={className} onClick={onClick} aria-pressed={selected}>
        {body}
      </button>
    );
  }

  return <div className={className}>{body}</div>;
}
