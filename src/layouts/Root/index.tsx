import { useLayoutEffect, type FC } from 'react';
import { Outlet } from 'react-router';
import {
  Tolgee,
  DevTools,
  TolgeeProvider,
  FormatSimple,
  LanguageStorage,
  LanguageDetector,
} from '@tolgee/react';
import { IconContext } from 'react-icons/lib';

import Loading from '@/components/Loading';
import AppLayout from '../App';

import GlobalContextStore from '@/store/global';
import { tolgee } from '@/utils/i18n';

import './index.scss';

interface IProps {}

const RootLayout: FC<IProps> = () => {
  return (
    <TolgeeProvider
      tolgee={tolgee}
      fallback={<Loading />} // loading fallback
    >
      <IconContext.Provider value={{ className: 'r-icon' }}>
        <GlobalContextStore.Provider>
          <AppLayout>
            <Outlet />
          </AppLayout>
        </GlobalContextStore.Provider>
      </IconContext.Provider>
    </TolgeeProvider>
  );
};

export default RootLayout;
