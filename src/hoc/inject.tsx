import { FC } from 'react';

// inject static values to a component so that they're always provided
export default function inject<P>(
  Component: FC<P>,
  injector = (_: any) => ({}),
) {
  const Inject: FC<P> = (props: any) => {
    return <Component {...props} {...() => injector(props)} />;
  };
  Object.assign(Inject, Component);
  Inject.displayName = `${Component.displayName || Component.name}Injector`;

  return Inject;
}
