/* eslint-disable react-compiler/react-compiler */
'use no memo';

import type { ComponentType, ReactNode } from 'react';
import { createContext, useContext } from 'react';

const EMPTY: unique symbol = Symbol();

export interface ContextStoreProviderProps<State = void> {
  initialState?: State;
  children: ReactNode;
}

export interface ContextStore<Value = any, State = void> {
  Provider: ComponentType<ContextStoreProviderProps<State>>;
  use: () => Value;
}

function createContextStore<Value, State = void>(
  useHook: (initialState?: State) => Value,
): ContextStore<Value, State> {
  const TempContext = createContext<Value | typeof EMPTY>(EMPTY);

  function Provider(props: ContextStoreProviderProps<State>) {
    "use no memo";
    const value = useHook(props.initialState);

    return (
      <TempContext.Provider value={value}>
        {props.children}
      </TempContext.Provider>
    );
  }

  function useInspect(): Value {
    "use no memo";
    const value = useContext(TempContext);
    if (value === EMPTY) {
      // throw new Error('Component must be wrapped with <Store.Provider>');
      return {} as Value;
    }
    return value;
  }

  return { Provider, use: useInspect };
}

function useContextStore<Value, State = void>(
  store: ContextStore<Value, State>,
): Value {
  return store.use();
}

export { useContextStore };

export default createContextStore;
