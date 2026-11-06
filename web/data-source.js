// Seul point d'accès aux données de la page sociale (S5, première tranche).
// Aujourd'hui : un `fetch` d'un fichier local au format HistoryView (§14.3).
// Plus tard : remplacer `loadHistoryView` par un appel à /api/v2
// (POST /worlds/{canon}/query) qui renvoie le même objet ; rien d'autre ne change.

export const FIXTURE_URL = './fixtures/social-view.demo.json';

export async function loadHistoryView(url = FIXTURE_URL) {
  const res = await fetch(url, { headers: { Accept: 'application/json' } });
  if (!res.ok) throw new Error(`Lecture de ${url} impossible (HTTP ${res.status}).`);
  const view = await res.json();
  if (!view || typeof view !== 'object' || !Array.isArray(view.entities)) {
    throw new Error('Format HistoryView invalide : « entities » manquant.');
  }
  return view;
}
