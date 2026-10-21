// Same-origin client for the Chronos local API (see docs/interface.md).
// Every call returns { data, text }: `text` is the exact server JSON, kept so
// exports reproduce the server serialization byte for byte.

export class ApiError extends Error {
  constructor(message, status) {
    super(message);
    this.status = status;
  }
}

async function request(method, path, body) {
  let res;
  try {
    res = await fetch(path, {
      method,
      headers: body === undefined ? { Accept: 'application/json' } : { Accept: 'application/json', 'Content-Type': 'application/json' },
      body,
      cache: 'no-store',
    });
  } catch {
    throw new ApiError(
      'Le serveur Chronos ne répond pas. Vérifiez qu’il tourne (commande dans le README), puis rechargez la page.',
      0,
    );
  }
  const text = await res.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = null;
  }
  if (!res.ok) {
    const msg = data && typeof data.error === 'string' && data.error
      ? data.error
      : `Le serveur a répondu ${res.status}${text ? ` : ${text.slice(0, 200)}` : '.'}`;
    throw new ApiError(msg, res.status);
  }
  if (data === null || typeof data !== 'object') {
    throw new ApiError(`Réponse illisible de ${path} : JSON attendu.`, res.status);
  }
  return { data, text };
}

export const api = {
  defaults: () => request('GET', '/api/defaults'),
  world: () => request('GET', '/api/world'),
  generate: (config) => request('POST', '/api/generate', JSON.stringify(config)),
  // The file text is forwarded untouched so the server validates exactly what was exported.
  importWorld: (text) => request('POST', '/api/import', text),
};
