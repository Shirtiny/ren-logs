const isDev = () => {
  return import.meta.env.DEV;
};

const isDebug = () => {
  return location.search.includes('debug');
};

const isMobile = (win: Window, width?: number) => {
  const isMobileAgent =
    /(phone|pad|pod|iPhone|iPod|ios|iPad|Android|Mobile|BlackBerry|IEMobile|MQQBrowser|JUC|Fennec|wOSBrowser|BrowserNG|WebOS|Symbian|Windows Phone)/i.test(
      win.navigator.userAgent,
    );

  if (width) {
    const isNarrow = win.document.documentElement.clientWidth <= width;
    return isMobileAgent || isNarrow;
  }

  return isMobileAgent;
};

export const isFirefox = (): boolean =>
  /firefox/i.test(window.navigator.userAgent);

function isScreenWap() {
  if (typeof window === 'undefined') return false;
  return window.innerWidth <= 875;
}

const env = {
  isDev,
  isDebug,
  isMobile,
  isFirefox,
  isScreenWap,
};

export default env;
