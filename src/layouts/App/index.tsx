/* eslint-disable react-compiler/react-compiler */
import clsx from 'clsx';
import { FC, useEffect, useLayoutEffect, useState } from 'react';
import { useNavigation } from 'react-router';
import { useTranslate } from '@tolgee/react';
import Button from '@/components/Button';
import RouterLoading from '@/components/Loading/Router';
import component from '@/hoc/component';
import themeUtil from '@/styles/theme';
import type { ICommonProps } from '@/types';
import layout from '@/utils/layout';
import logger from '@/utils/logger';
import { showWindow } from '@/utils/window';
import Header from './Header';
import css from './index.module.scss';

interface IProps extends ICommonProps {}

const AppLayout: FC<IProps> = ({
  className,
  theme,
  clientWidth,
  isMobile,
  children,
  ...rest
}) => {
  const { t } = useTranslate();
  const navigation = useNavigation();

  useLayoutEffect(() => {
    logger.debug('layout', isMobile);

    // Vite 加载动态导入失败
    window.addEventListener('vite:preloadError', (event) => {
      event.preventDefault();
      window.location.reload(); // 例如，刷新页面
    });

    return () => {};
  }, [isMobile]);

  return (
    <div
      className={clsx(css.appLayout, 'bg-base-100/30 hover:bg-base-100/80 transition-colors')}
      {...rest}
    >
      {navigation.state === 'loading' && <RouterLoading />}
      <Header />
      <main className={css.main}>{children}</main>
      <footer className={css.footer}></footer>
    </div>
  );
};

export default component<IProps>(AppLayout, {
  globalDeps: ['theme', 'clientWidth', 'isMobile'],
});
