import type { Draft, Immutable } from 'immer';
import { useCallback, useMemo, useReducer } from 'react';
import { produce } from 'immer';
import { updatedDiff } from 'deep-object-diff';
import { useTranslate } from '@tolgee/react';

import createContextStore from '@/store/contextStore';
import useOnlineStatus from '@/hooks/useOnlineStatus';
import useClientWidth from '@/hooks/useClientWidth';
import theme, { ColorThemes } from '@/styles/theme';
import env from '@/utils/env';
import logger from '@/utils/logger';

type State = Immutable<{
  theme: ColorThemes;
}>;

type StateIndexed = State & { [index: string]: any };

type Action = { type: string; name?: string; payload?: string };

export const globalInitialState: State = {
  theme: theme.getTheme()!,
};

export interface IGlobalContextValue {
  online: boolean;
  isMobile: boolean;
  clientWidth: number;
  theme: ColorThemes;

  toggleTheme: () => void;
}

const reducerWithImmer = produce<StateIndexed, [Action]>(
  (draft, { type, name, payload }) => {
    switch (type) {
      case 'set': {
        name && (draft[name] = payload);
        break;
      }
      default:
        break;
    }
  },
);

const reducer = env.isDev()
  ? (state: StateIndexed, action: Action) => {
      const nextState = reducerWithImmer(state, action);

      const now = new Date();
      logger.group(
        `global state action @${now.toLocaleTimeString()}.${now.getMilliseconds()}`,
        () => {
          logger.globalState.pre(state);
          logger.globalState.action(action);
          logger.globalState.next(nextState);
          logger.globalState.changes(updatedDiff(state, nextState));
        },
      );

      return nextState;
    }
  : reducerWithImmer;

const useGlobalState = (initialState = globalInitialState) => {
  const [state, dispatch] = useReducer(reducer, initialState);

  const online = useOnlineStatus();
  const clientWidth = useClientWidth();

  const toggleTheme = useCallback(() => {
    const cur = theme.getTheme();
    const next =
      cur === ColorThemes.LIGHT ? ColorThemes.DARK : ColorThemes.LIGHT;
    theme.setTheme(next, true);
    dispatch({ type: 'set', name: 'theme', payload: next });
  }, []);

  const isMobile = clientWidth <= 750;

  const contextValue: IGlobalContextValue = {
    ...state,
    online,
    isMobile,
    clientWidth,

    toggleTheme,
  };

  return contextValue;
};

const GlobalContextStore = createContextStore(useGlobalState);

export default GlobalContextStore;
