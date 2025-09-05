import logger from './logger';
import { FC } from 'react';

export interface Component {
  componentType: string;
}

interface RenderOptions {
  key: string;
}

const render = (
  map: Record<string, FC<any>>,
  component?: Component,
  options?: RenderOptions,
) => {
  if (!map || !component) return null;

  const { key } = options || {};

  const { componentType, ...props } = component;

  const Com = map[componentType];

  if (!Com) {
    logger.warn(`render: No matching implementation for ${componentType}.`);
    return null;
  }

  return <Com key={key} {...props} />;
};

export default render;
