import type { CSSProperties, PropsWithChildren, RefObject } from 'react';
import { IGlobalContextValue } from './store/global';

export interface ICommonProps
  extends Partial<IGlobalContextValue>,
    PropsWithChildren {
  ref?: RefObject<any>;
  className?: string;
  style?: CSSProperties;
  [key: string]: any;
}
