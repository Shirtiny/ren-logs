/* eslint-disable react-compiler/react-compiler */
/* eslint-disable react-hooks/rules-of-hooks */

import { FC, memo, ComponentType } from 'react';
import GlobalContextStore, { IGlobalContextValue } from '@/store/global';
import logger from '@/utils/logger';

interface IOptions {
  manualMemo?: boolean;
  globalDeps?: (keyof IGlobalContextValue | never)[];
}

const NAME_PREFIX = '';

const pick = (target: any = {}, fields: string[]) => {
  const result: any = {};
  fields.forEach((key) => {
    if (target[key] !== undefined) {
      result[key] = target[key];
    }
  });
  return result;
};

export default function component<P>(
  Component: FC<P>,
  options?: IOptions,
): FC<P> {
  const { manualMemo = true, globalDeps = [] } = options || {};
  const componentName = Component.displayName || Component.name || '';
  const FinalComponent = manualMemo ? memo(Component) : Component;

  const Func = (props: any) => {
    'use no memo';

    let deps = {};

    // globalDeps是不变值
    if (globalDeps.length) {
      const global = GlobalContextStore.use();
      deps = pick(global, globalDeps);
    }

    const finalProps = {
      ...props,
      ...deps,
    };

    return (
      <FinalComponent
        {...finalProps}
        data-comp={`${NAME_PREFIX}${componentName}`}
      />
    );
  };
  // Object.assign(Func, Component);
  Func.displayName = `${componentName}Wrapper`;

  return memo(Func as ComponentType<P>);
}
