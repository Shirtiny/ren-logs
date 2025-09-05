import { FC, useImperativeHandle, useRef } from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';
import { cls } from '@shirtiny/utils/lib/style';
import logger from '@/utils/logger';

import SimpleBar from 'simplebar-react';

import 'simplebar-react/dist/simplebar.min.css';
import './index.scss';

interface IProps extends ICommonProps {
  maxHeight?: number | string;
  autoHide?: boolean;
}

export interface IScrollbarRef {
  el: HTMLDivElement;
  scrollableNode: HTMLDivElement;
  instance: any;
}

// SimpleBar is meant to be as easy to use as possible and lightweight. If you want something more advanced I recommend KingSora 's Overlay Scrollbars.
// https://kingsora.github.io/OverlayScrollbars/

const Scrollbar: FC<IProps> = ({
  ref,
  className,
  style = {},
  maxHeight,
  autoHide,
  children,
  ...rest
}) => {
  const scrollRef = useRef(null);
  const scrollableNodeRef = useRef(null);

  useImperativeHandle(ref, () => {
    const scrollCurrent = scrollRef.current as any;

    const root = scrollCurrent?.el;
    const scrollableNode = scrollableNodeRef.current;

    logger.debug('scrollbar root', root);
    logger.debug('scrollbar instance', scrollCurrent);
    logger.debug('scrollbar scrollableNode', scrollableNode);

    return { el: root, scrollableNode, instance: scrollCurrent };
  });

  return (
    <SimpleBar
      {...rest}
      autoHide={autoHide}
      className={cls('scrollbar', className)}
      style={{
        ...style,
        maxHeight,
      }}
      ref={scrollRef}
      scrollableNodeProps={{ ref: scrollableNodeRef }}
    >
      {children}
    </SimpleBar>
  );
};
Scrollbar.displayName = 'Scrollbar';

export default component<IProps>(Scrollbar, {});
