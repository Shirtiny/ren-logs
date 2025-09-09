import clsx from 'clsx';
import React, { useLayoutEffect, useRef, useState, type FC } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import Button from '@/components/Button';
import component from '@/hoc/component';
import type { ICommonProps } from '@/types';
import Language from './Language';
import Theme from './Theme';
import css from './index.module.scss';

interface IProps extends ICommonProps {}

const AppHeader: FC<IProps> = ({ className, ...rest }) => {
  const [sticky, setSticky] = useState(false);
  const sentinelTopRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const sentinelTop = sentinelTopRef.current;
    if (!sentinelTop) return;

    const observer = new IntersectionObserver(
      ([entry]) => setSticky(!entry.isIntersecting),
      { threshold: 0 },
    );

    observer.observe(sentinelTop);

    return () => {
      observer.disconnect();
    };
  }, []);

  const handleMinimize = async () => {
    const appWindow = getCurrentWebviewWindow();
    await appWindow.minimize();
  };

  const handleClose = async () => {
    const appWindow = getCurrentWebviewWindow();
    await appWindow.close();
  };

  const togglePin = async () => {
    invoke('toggle_always_on_top');
  };

  return (
    <>
      <div ref={sentinelTopRef} className="h-0 w-full" />
      <header
        className={clsx(
          'flex  sticky top-0 p-2 select-none bg-base-100/80 transition-all',
          sticky && 'shadow-md',
          css.appHeader,
          className,
        )}
        {...rest}
      >
        <div data-tauri-drag-region className="flex-1 cursor-move"></div>

        <div className="flex-none flex gap-x-1">
          <Button
            className="ui-btn-ghost ui-btn-xs"
            onClick={togglePin}
            title="置顶"
          >
            置顶
          </Button>
          <Button
            className="ui-btn-ghost ui-btn-xs"
            onClick={handleMinimize}
            title="最小化"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 13H5v-2h14v2z" />
            </svg>
          </Button>
          <Button
            className="ui-btn-ghost ui-btn-xs hover:ui-btn-error"
            onClick={handleClose}
            title="关闭"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
            </svg>
          </Button>
        </div>
      </header>
    </>
  );
};

export default component<IProps>(AppHeader);
