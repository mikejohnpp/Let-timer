import { useState, useEffect, useCallback } from 'react';

export type AppView = 'today' | 'focus' | 'workspaces' | 'media' | 'settings' | 'dev/motion';

export interface RouteState {
  view: AppView;
  params: URLSearchParams;
}

function parseHash(): RouteState {
  const hash = window.location.hash.replace(/^#\/?/, '');
  const [path, queryString] = hash.split('?');
  const params = new URLSearchParams(queryString || '');

  let view: AppView = 'today';
  if (path === 'focus') view = 'focus';
  else if (path === 'workspaces') view = 'workspaces';
  else if (path === 'media') view = 'media';
  else if (path === 'settings') view = 'settings';
  else if (path === 'dev/motion') view = 'dev/motion';

  return { view, params };
}

export function useHashRoute(): [RouteState, (view: AppView, params?: Record<string, string>) => void] {
  const [route, setRoute] = useState<RouteState>(() => parseHash());

  useEffect(() => {
    const handleHashChange = () => {
      setRoute(parseHash());
    };

    window.addEventListener('hashchange', handleHashChange);
    if (!window.location.hash) {
      window.location.hash = '#/today';
    }
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  const navigate = useCallback((view: AppView, params?: Record<string, string>) => {
    let newHash = `#/${view}`;
    if (params && Object.keys(params).length > 0) {
      const sp = new URLSearchParams(params);
      newHash += `?${sp.toString()}`;
    }
    window.location.hash = newHash;
  }, []);

  return [route, navigate];
}

