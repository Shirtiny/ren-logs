import type { FC } from 'react';
import type { ICommonProps } from '@/types';
import clsx from 'clsx';
import component from '@/hoc/component';
import GlobalContextStore from '@/store/global';

import { ColorThemes } from '@/styles/theme';
import IconWrap from '@/components/Icon';
import { HiOutlineMoon, HiOutlineSun } from 'react-icons/hi2';

import css from './index.module.scss';

interface IProps extends ICommonProps {}

const Theme: FC<IProps> = ({ className, theme, toggleTheme, ...rest }) => {
  const light = theme === ColorThemes.LIGHT;

  return (
    <div className={clsx(css.theme, className)} {...rest}>
      <label
        className="ui-swap ui-swap-rotate ui-btn ui-btn-sm ui-btn-ghost ui-btn-circle"
        role="button"
        tabIndex={0}
      >
        <input
          className="ui-theme-controller"
          type="checkbox"
          checked={light}
          onChange={toggleTheme}
        />

        <IconWrap className="ui-swap-on h-5 w-5" Icon={HiOutlineSun} />
        <IconWrap className="ui-swap-off h-5 w-5" Icon={HiOutlineMoon} />
      </label>
    </div>
  );
};

export default component<IProps>(Theme, {
  globalDeps: ['theme', 'toggleTheme'],
});
