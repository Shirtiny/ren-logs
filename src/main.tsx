import React from 'react';
import ReactDOM from 'react-dom/client';

import AppRouter from '@/router/index';
import dev from '@shirtiny/utils/lib/dev';
import env from './utils/env';
import i18n from './utils/i18n';
import 'pepjs';
import './styles/lib.scss';
import './styles/tailwind.css';
import './styles/global.scss';

async function beforeRender() {
  (window as any).dev = dev;
  i18n.init();

  if (env.isDebug()) {
    import('./utils/eruda').then(({ default: eruda }) => eruda.init());
  }

  if (!env.isDev()) {
    return;
  }

  const { worker } = await import('../mocks/browser');

  // `worker.start()` returns a Promise that resolves
  // once the Service Worker is up and ready to intercept requests.
  return worker.start();
}

beforeRender().then(() => {
  ReactDOM.createRoot(document.getElementById('root') as HTMLElement, {
    onUncaughtError: (error, _errorInfo) => {
      console.error(error);
    },
    onCaughtError: (error, _errorInfo) => {
      console.error(error);
    },
  }).render(
    <React.StrictMode>
      <AppRouter />
    </React.StrictMode>,
  );
});
