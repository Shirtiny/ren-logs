import clsx from 'clsx';
import React, { useLayoutEffect, useRef, useState, type FC } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import Image from '@/components/Image';
import component from '@/hoc/component';
import type { ICommonProps } from '@/types';
import Language from './Language';
import Theme from './Theme';
import css from './index.module.scss';

interface IProps extends ICommonProps {}

const AppHeader: FC<IProps> = ({ className, ...rest }) => {
  const [sticky, setSticky] = useState(false);
  const sentinelTopRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLElement>(null);
  const [isDragging, setIsDragging] = useState(false);

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

  const handleMouseDown = async (e: React.MouseEvent) => {
    // 防止在按钮上触发拖拽
    if (!(e.target as Element).closest('button')) {
      setIsDragging(true);
      const appWindow = getCurrentWebviewWindow();
      await appWindow.startDragging();
    }
  };

  const handleMinimize = async () => {
    const appWindow = getCurrentWebviewWindow();
    await appWindow.minimize();
  };

  const handleClose = async () => {
    const appWindow = getCurrentWebviewWindow();
    await appWindow.close();
  };

  return (
    <>
      <div ref={sentinelTopRef} className="h-0 w-full" />
      <header
        ref={headerRef}
        className={clsx(
          'ui-navbar bg-base-100 px-4 sticky top-0 cursor-move select-none',
          sticky && 'shadow-md',
          css.appHeader,
          isDragging && 'cursor-grabbing',
          className,
        )}
        onMouseDown={handleMouseDown}
        {...rest}
      >
        <div className="flex-1"></div>

        <div className="flex-none flex gap-x-1">
          <button
            className="btn btn-ghost btn-sm hover:bg-base-200"
            onClick={handleMinimize}
            title="最小化"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 13H5v-2h14v2z"/>
            </svg>
          </button>
          <button
            className="btn btn-ghost btn-sm hover:bg-red-500 hover:text-white"
            onClick={handleClose}
            title="关闭"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
            </svg>
          </button>
        </div>
      </header>
    </>
  );
};

export default component<IProps>(AppHeader);
