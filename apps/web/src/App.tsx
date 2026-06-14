import { BrowserRouter, NavLink, Navigate, Route, Routes } from 'react-router-dom';
import ScanPage from './pages/ScanPage';
import SearchPage from './pages/SearchPage';
import CollectionPage from './pages/CollectionPage';
import CardPage from './pages/CardPage';
import LanguageToggle from './components/LanguageToggle';
import { useT } from './lib/i18n';

function ScanIcon() {
  return (
    <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M4 8V6a2 2 0 0 1 2-2h2" />
      <path d="M16 4h2a2 2 0 0 1 2 2v2" />
      <path d="M20 16v2a2 2 0 0 1-2 2h-2" />
      <path d="M8 20H6a2 2 0 0 1-2-2v-2" />
      <rect x="8.5" y="7" width="7" height="10" rx="1.5" />
    </svg>
  );
}

function SearchIcon() {
  return (
    <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <circle cx="11" cy="11" r="7" />
      <path d="m21 21-4.3-4.3" />
    </svg>
  );
}

function CollectionIcon() {
  return (
    <svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <rect x="3" y="6" width="9" height="13" rx="1.5" />
      <path d="M14 5l5.2 1.4a1.5 1.5 0 0 1 1.06 1.84L17.5 18.5" />
    </svg>
  );
}

export default function App() {
  const { t } = useT();
  return (
    <BrowserRouter>
      <div className="app-shell">
        <header className="app-header">
          <span className="app-brand">CardLens</span>
          <LanguageToggle />
        </header>
        <main className="app-content">
          <Routes>
            <Route path="/" element={<ScanPage />} />
            <Route path="/buscar" element={<SearchPage />} />
            <Route path="/coleccion" element={<CollectionPage />} />
            <Route path="/carta/:id" element={<CardPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
          <footer className="app-disclaimer">{t('disclaimer')}</footer>
        </main>
        <nav className="bottom-nav" aria-label={t('nav.ariaLabel')}>
          <NavLink to="/" end className={({ isActive }) => (isActive ? 'nav-item active' : 'nav-item')}>
            <ScanIcon />
            <span>{t('nav.scan')}</span>
          </NavLink>
          <NavLink to="/buscar" className={({ isActive }) => (isActive ? 'nav-item active' : 'nav-item')}>
            <SearchIcon />
            <span>{t('nav.search')}</span>
          </NavLink>
          <NavLink to="/coleccion" className={({ isActive }) => (isActive ? 'nav-item active' : 'nav-item')}>
            <CollectionIcon />
            <span>{t('nav.collection')}</span>
          </NavLink>
        </nav>
      </div>
    </BrowserRouter>
  );
}
