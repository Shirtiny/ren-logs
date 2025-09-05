import dom from '@shirtiny/utils/lib/dom';

const THEME_STORAGE_KEY = 'leact-theme';

const config = [
  { rel: 'icon', file: '/favicon.ico' },
  {
    rel: 'icon',
    file: '/favicon-16x16.png',
    sizes: '16x16',
    type: 'image/png',
  },
  {
    rel: 'icon',
    file: '/favicon-32x32.png',
    sizes: '32x32',
    type: 'image/png',
  },
  { rel: 'apple-touch-icon', file: '/apple-touch-icon.png', sizes: '180x180' },
  {
    rel: 'manifest',
    file: '/site.webmanifest',
  },
];

// media="(prefers-color-scheme: light)"

export enum ColorThemes {
  DARK = 'abyss',
  LIGHT = 'light',
}

const setIcons = (themeColor: ColorThemes) => {
  const isLight = themeColor === ColorThemes.LIGHT;
  const iconDir = `/favicon-${isLight ? 'light' : 'dark'}`;

  const iconRels = Array.from(new Set(config.map((item) => item.rel)));

  const olds = document.head.querySelectorAll(
    iconRels.map((rel) => `link[rel="${rel}"]`).join(','),
  );

  olds.forEach((el) => dom.removeSelf(el));

  const links = config.map((item) => {
    const { file, rel, sizes, type } = item;

    return dom.create('link', {
      rel,
      sizes,
      type,
      href: `${iconDir}${file}`,
    });
  });

  const frag = dom.createFragment();
  dom.append(frag, ...links);
  dom.append(document.head, frag);
};

const getTheme = (): ColorThemes | null => {
  const htmlEl = document.querySelector('html')!;
  const curTheme = htmlEl.getAttribute('data-theme') as ColorThemes;
  return curTheme;
};

const setTheme = async (themeColor: ColorThemes, save = true) => {
  const htmlEl = document.querySelector('html')!;
  const curTheme = htmlEl.getAttribute('data-theme');
  save && localStorage.setItem(THEME_STORAGE_KEY, themeColor);
  if (curTheme === themeColor) return;

  document.documentElement.classList.add('view-transition');

  const updateDom = () => {
    htmlEl.setAttribute('data-theme', themeColor);
    setTimeout(() => {
      document.documentElement.classList.remove('view-transition');
    }, 0);

    setIcons(themeColor);
  };

  updateDom();
  return;
};

const toggleTheme = () => {
  const curTheme = getTheme();
  const newTheme =
    curTheme === ColorThemes.LIGHT ? ColorThemes.DARK : ColorThemes.LIGHT;
  setTheme(newTheme);
  return newTheme;
};

const initTheme = () => {
  let initialTheme = localStorage.getItem('leact-theme');

  if (!initialTheme) {
    initialTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
      ? ColorThemes.DARK
      : ColorThemes.LIGHT;
  } else {
    initialTheme =
      initialTheme === ColorThemes.LIGHT ? ColorThemes.LIGHT : ColorThemes.DARK;
  }

  const htmlEl = document.querySelector('html')!;

  htmlEl.setAttribute('data-theme', initialTheme);

  setIcons(initialTheme as ColorThemes);
};

initTheme();

const theme = { getTheme, setTheme, toggleTheme };

export default theme;
