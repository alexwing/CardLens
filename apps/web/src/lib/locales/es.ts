// Diccionario de textos en espanol. Es la fuente canonica de claves:
// `en.ts` debe declarar exactamente estas mismas claves (lo fuerza el tipo).
const es = {
  // Navegacion y cabecera
  'nav.scan': 'Escanear',
  'nav.search': 'Buscar',
  'nav.collection': 'Colección',
  'nav.ariaLabel': 'Navegación principal',
  'lang.ariaLabel': 'Idioma',

  // Aviso legal (pie de la app)
  'disclaimer':
    'Proyecto no oficial, sin ánimo de lucro. No afiliado ni respaldado por Nintendo, The Pokémon Company, Creatures ni GAME FREAK. «Pokémon», los nombres de las cartas y sus imágenes son marcas y © de sus respectivos propietarios; se usan únicamente con fines identificativos.',

  // Comun
  'common.language': 'Idioma',
  'common.confidence': 'Confianza',
  'common.setNumber': '{set} · Nº {number}',
  'common.dash': '—',

  // Escaneo
  'scan.title': 'Escanear carta',
  'scan.subtitle': 'Identifica una carta Pokémon con una foto.',
  'scan.captureMode': 'Modo de captura',
  'scan.tab.camera': 'Cámara',
  'scan.tab.upload': 'Subir',
  'scan.uploadButton': 'Hacer foto o elegir imagen',
  'scan.uploadHint': 'En el móvil se abrirá la cámara nativa. También puedes elegir una foto de la galería.',
  'scan.analyzing': 'Analizando la carta…',
  'scan.error': 'No se pudo analizar la imagen. Comprueba que la API está en marcha e inténtalo de nuevo.',

  // Camara
  'camera.unavailable': 'La cámara no está disponible en este navegador o en esta conexión (se necesita HTTPS o localhost). Usa la pestaña «Subir» para hacer la foto con la cámara nativa.',
  'camera.denied': 'No se pudo acceder a la cámara. Comprueba que has concedido el permiso, o usa la pestaña «Subir» para elegir una foto.',
  'camera.capture': 'Capturar carta',
  'camera.starting': 'Iniciando cámara…',
  'camera.hint': 'Encuadra la carta dentro del marco y pulsa el botón.',

  // Resultado del escaneo
  'result.title': 'Resultado',
  'result.noMatch.title': 'Sin coincidencias',
  'result.noMatch.text': 'No se ha podido identificar ninguna carta en la foto. Prueba con mejor luz, fondo liso y la carta ocupando la mayor parte del encuadre.',
  'result.lowConfidence': 'Confianza baja: revisa las alternativas y elige la carta correcta antes de guardar.',
  'result.altPrompt': '¿No es esta? Elige la correcta:',
  'result.save': 'Guardar en colección',
  'result.saving': 'Guardando…',
  'result.saved': 'Guardada en la colección ✓',
  'result.alreadySaved': 'Ya en la colección',
  'result.alreadyInCollection': 'Esta carta ya estaba en tu colección.',
  'result.savedText': 'Carta añadida.',
  'result.viewCollection': 'Ver colección',
  'result.viewCard': 'Ver ficha',
  'result.saveError': 'No se pudo guardar la carta. Inténtalo de nuevo.',
  'result.prices': 'Precios orientativos',
  'result.prices.loading': 'Cargando precios…',
  'result.prices.empty': 'Sin precio de mercado para esta carta.',

  // Detalle de carta
  'card.back': '← Volver',
  'card.loading': 'Cargando carta…',
  'card.error': 'No se pudo cargar la carta. Puede que no exista en el catálogo.',
  'card.meta.set': 'Set',
  'card.meta.number': 'Número',
  'card.meta.rarity': 'Rareza',
  'card.meta.type': 'Tipo',
  'card.prices': 'Precios',
  'card.prices.loading': 'Cargando precios…',
  'card.prices.empty': 'Sin precio de mercado para esta carta.',
  'card.prices.emptyHint': 'Algunas cartas (p. ej. de Pokémon TCG Pocket o promocionales) no cotizan en Cardmarket ni TCGplayer.',
  'card.prices.source': 'Fuente',
  'card.prices.market': 'Mercado',
  'card.prices.low': 'Mínimo',
  'card.prices.high': 'Máximo',
  'card.prices.trend': 'Tendencia',

  // Nota de la carta en la coleccion
  'card.inCollection': 'En tu colección',
  'card.note.title': 'Mi nota',
  'card.note.placeholder': 'Añade una nota para esta carta (estado, dónde la guardas, valor sentimental…)',
  'card.note.save': 'Guardar nota',
  'card.note.saving': 'Guardando…',
  'card.note.saved': 'Nota guardada ✓',
  'card.note.error': 'No se pudo guardar la nota. Inténtalo de nuevo.',
  'card.note.addToCollection': 'Añade esta carta a tu colección para poder anotarla.',
  'card.addToCollectionBtn': 'Añadir a la colección',

  // Buscador de cartas
  'search.title': 'Buscar cartas',
  'search.subtitle': 'Encuentra una carta por nombre, número o set.',
  'search.placeholder': 'Nombre, número o set…',
  'search.minChars': 'Escribe al menos 2 caracteres para buscar.',
  'search.searching': 'Buscando…',
  'search.empty': 'Sin resultados para «{q}».',
  'search.error': 'No se pudo buscar. Comprueba que la API está en marcha.',
  'search.show': 'Resultados:',
  'search.resultsHint': 'Se muestran hasta {n} resultados.',

  // Coleccion
  'collection.title': 'Mi colección',
  'collection.count.one': '1 carta guardada',
  'collection.count.other': '{count} cartas guardadas',
  'collection.export': 'Exportar JSON',
  'collection.import': 'Importar JSON',
  'collection.loading': 'Cargando colección…',
  'collection.loadError': 'No se pudo cargar la colección. Comprueba que la API está en marcha.',
  'collection.empty': 'Todavía no tienes cartas en la colección.',
  'collection.emptyCta': 'Escanear mi primera carta',
  'collection.remove': 'Eliminar',
  'collection.removeAria': 'Eliminar {name}',
  'collection.removeConfirm': '¿Eliminar «{name}» de tu colección?',
  'collection.removeError': 'No se pudo eliminar la carta. Inténtalo de nuevo.',
  'collection.quantityAdded': 'Cantidad: {qty} · Añadida el {date}',
  'collection.exportError': 'No se pudo exportar la colección. Comprueba que la API está en marcha.',
  'collection.exported.one': 'Colección exportada (1 carta).',
  'collection.exported.other': 'Colección exportada ({count} cartas).',
  'collection.importConfirm':
    'Importar colección:\n\nAceptar = REEMPLAZAR toda tu colección por el archivo.\nCancelar = COMBINAR (añadir cartas nuevas y actualizar las existentes).',
  'collection.importInvalid': 'El archivo no tiene un formato de colección válido (falta «items»).',
  'collection.importNotJson': 'El archivo no es un JSON válido.',
  'collection.importError': 'No se pudo importar la colección. Inténtalo de nuevo.',
  'collection.import.added': '{count} añadidas',
  'collection.import.updated': '{count} actualizadas',
  'collection.import.skipped': '{count} omitidas (no están en el catálogo local)',
  'collection.import.summary': 'Importación completada ({mode}): {parts}.',
  'collection.mode.merge': 'combinar',
  'collection.mode.replace': 'reemplazar',

  // Filtros de la coleccion
  'filters.search': 'Buscar por nombre',
  'filters.searchPlaceholder': 'Nombre de la carta…',
  'filters.tags': 'Etiquetas',
  'filters.all': 'Todo',
  'filters.set': 'Set',
  'filters.rarity': 'Rareza',
  'filters.type': 'Tipo',
  'filters.lang': 'Idioma',
  'filters.allSets': 'Todos los sets',
  'filters.allRarities': 'Todas las rarezas',
  'filters.allTypes': 'Todos los tipos',
  'filters.allLangs': 'Todos los idiomas',
  'filters.clear': 'Limpiar filtros',
  'filters.noResults': 'Ninguna carta coincide con los filtros.',
  'filters.resultCount.one': '1 carta coincide',
  'filters.resultCount.other': '{count} cartas coinciden',

  // Etiquetas en las cartas
  'tags.add': '+ etiqueta',
  'tags.addPlaceholder': 'Nueva o existente…',
  'tags.addConfirm': 'Añadir',
  'tags.cancel': 'Cancelar',
  'tags.removeAria': 'Quitar etiqueta {name}',
  'tags.addError': 'No se pudo añadir la etiqueta. Inténtalo de nuevo.',
  'tags.removeError': 'No se pudo quitar la etiqueta. Inténtalo de nuevo.',

  // Barra de confianza
  'confidence.aria': 'Confianza {percent}%',
};

export type TranslationKey = keyof typeof es;
export default es;
