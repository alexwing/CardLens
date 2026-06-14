import type { TranslationKey } from './es';

// English dictionary. Typed as Record<TranslationKey, string> so the compiler
// flags any missing or unknown key relative to the canonical Spanish dictionary.
const en: Record<TranslationKey, string> = {
  // Navigation and header
  'nav.scan': 'Scan',
  'nav.collection': 'Collection',
  'nav.ariaLabel': 'Main navigation',
  'lang.ariaLabel': 'Language',

  // Legal notice (app footer)
  'disclaimer':
    'Unofficial, non-commercial project. Not affiliated with or endorsed by Nintendo, The Pokémon Company, Creatures or GAME FREAK. "Pokémon", card names and card images are trademarks and © of their respective owners; used for identification purposes only.',

  // Common
  'common.language': 'Language',
  'common.confidence': 'Confidence',
  'common.setNumber': '{set} · No. {number}',
  'common.dash': '—',

  // Scan
  'scan.title': 'Scan a card',
  'scan.subtitle': 'Identify a Pokémon card from a photo.',
  'scan.captureMode': 'Capture mode',
  'scan.tab.camera': 'Camera',
  'scan.tab.upload': 'Upload',
  'scan.uploadButton': 'Take a photo or choose an image',
  'scan.uploadHint': 'On mobile the native camera opens. You can also pick a photo from your gallery.',
  'scan.analyzing': 'Analyzing the card…',
  'scan.error': 'Could not analyze the image. Make sure the API is running and try again.',

  // Camera
  'camera.unavailable': 'The camera is not available in this browser or connection (HTTPS or localhost required). Use the "Upload" tab to take the photo with the native camera.',
  'camera.denied': 'Could not access the camera. Make sure you have granted permission, or use the "Upload" tab to choose a photo.',
  'camera.capture': 'Capture card',
  'camera.starting': 'Starting camera…',
  'camera.hint': 'Frame the card inside the guide and press the button.',

  // Scan result
  'result.title': 'Result',
  'result.noMatch.title': 'No matches',
  'result.noMatch.text': 'Could not identify any card in the photo. Try better lighting, a plain background, and the card filling most of the frame.',
  'result.lowConfidence': 'Low confidence: check the alternatives and pick the correct card before saving.',
  'result.altPrompt': 'Not this one? Pick the correct card:',
  'result.save': 'Save to collection',
  'result.saving': 'Saving…',
  'result.saved': 'Saved to collection ✓',
  'result.alreadySaved': 'Already in collection',
  'result.alreadyInCollection': 'This card is already in your collection.',
  'result.savedText': 'Card added.',
  'result.viewCollection': 'View collection',
  'result.saveError': 'Could not save the card. Try again.',

  // Card detail
  'card.back': '← Back',
  'card.loading': 'Loading card…',
  'card.error': 'Could not load the card. It may not exist in the catalog.',
  'card.meta.set': 'Set',
  'card.meta.number': 'Number',
  'card.meta.rarity': 'Rarity',
  'card.meta.type': 'Type',
  'card.prices': 'Prices',
  'card.prices.loading': 'Loading prices…',
  'card.prices.empty': 'No price source configured.',
  'card.prices.emptyHint': 'Configure a price provider in the API (PRICE_PROVIDER) to see quotes here.',
  'card.prices.source': 'Source',
  'card.prices.market': 'Market',
  'card.prices.low': 'Low',
  'card.prices.high': 'High',
  'card.prices.trend': 'Trend',

  // Collection
  'collection.title': 'My collection',
  'collection.count.one': '1 card saved',
  'collection.count.other': '{count} cards saved',
  'collection.export': 'Export JSON',
  'collection.import': 'Import JSON',
  'collection.loading': 'Loading collection…',
  'collection.loadError': 'Could not load the collection. Make sure the API is running.',
  'collection.empty': 'You do not have any cards in your collection yet.',
  'collection.emptyCta': 'Scan my first card',
  'collection.remove': 'Remove',
  'collection.removeAria': 'Remove {name}',
  'collection.removeConfirm': 'Remove "{name}" from your collection?',
  'collection.removeError': 'Could not remove the card. Try again.',
  'collection.quantityAdded': 'Quantity: {qty} · Added on {date}',
  'collection.exportError': 'Could not export the collection. Make sure the API is running.',
  'collection.exported.one': 'Collection exported (1 card).',
  'collection.exported.other': 'Collection exported ({count} cards).',
  'collection.importConfirm':
    'Import collection:\n\nOK = REPLACE your entire collection with the file.\nCancel = MERGE (add new cards and update existing ones).',
  'collection.importInvalid': 'The file is not a valid collection format (missing "items").',
  'collection.importNotJson': 'The file is not valid JSON.',
  'collection.importError': 'Could not import the collection. Try again.',
  'collection.import.added': '{count} added',
  'collection.import.updated': '{count} updated',
  'collection.import.skipped': '{count} skipped (not in the local catalog)',
  'collection.import.summary': 'Import complete ({mode}): {parts}.',
  'collection.mode.merge': 'merge',
  'collection.mode.replace': 'replace',

  // Collection filters
  'filters.search': 'Search by name',
  'filters.searchPlaceholder': 'Card name…',
  'filters.tags': 'Tags',
  'filters.all': 'All',
  'filters.set': 'Set',
  'filters.rarity': 'Rarity',
  'filters.type': 'Type',
  'filters.lang': 'Language',
  'filters.allSets': 'All sets',
  'filters.allRarities': 'All rarities',
  'filters.allTypes': 'All types',
  'filters.allLangs': 'All languages',
  'filters.clear': 'Clear filters',
  'filters.noResults': 'No card matches the filters.',
  'filters.resultCount.one': '1 card matches',
  'filters.resultCount.other': '{count} cards match',

  // Tags on cards
  'tags.add': '+ tag',
  'tags.addPlaceholder': 'New or existing…',
  'tags.addConfirm': 'Add',
  'tags.cancel': 'Cancel',
  'tags.removeAria': 'Remove tag {name}',
  'tags.addError': 'Could not add the tag. Try again.',
  'tags.removeError': 'Could not remove the tag. Try again.',

  // Confidence bar
  'confidence.aria': 'Confidence {percent}%',
};

export default en;
