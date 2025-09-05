import { useLayoutEffect, useState, type FC } from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';

import css from './index.module.scss';
import clsx from 'clsx';
import { useTolgee } from '@tolgee/react';
import IconWrap from '@/components/Icon';
import { HiOutlineChevronDown, HiOutlineGlobeAlt } from 'react-icons/hi2';

interface IProps extends ICommonProps {}

const labelMap: Record<string, string> = {
  en: 'English',
  zh: '中 文',
};

const Language: FC<IProps> = ({ className, ...rest }) => {
  const tolgee = useTolgee(['language']);

  const [cur, setCur] = useState(() => tolgee.getLanguage());

  useLayoutEffect(() => {
    document.documentElement.setAttribute('lang', cur || '');
  }, [cur]);

  const handleChange = (key: string) => {
    tolgee.changeLanguage(key);
    setCur(key);
  };

  return (
    <div
      className={clsx(
        'ui-dropdown',
        'ui-dropdown-center',
        css.language,
        className,
      )}
      {...rest}
    >
      <div
        className="ui-btn ui-btn-sm ui-btn-ghost px-1.5  menu-dropdown-toggle"
        tabIndex={0}
        role="button"
      >
        <IconWrap Icon={HiOutlineGlobeAlt} />
        <IconWrap Icon={HiOutlineChevronDown} />
      </div>

      <ul
        className="ui-dropdown-content ui-menu  bg-base-100 rounded-box z-1 shadow-sm"
        tabIndex={0}
      >
        <ul className="ui-menu p-0 w-full">
          {Object.keys(labelMap).map((key) => {
            const label = [labelMap[key]];

            const active = key === cur;

            return (
              <li key={key} onClick={() => handleChange(key)}>
                <a className={clsx(active && 'ui-menu-active')}>{label}</a>
              </li>
            );
          })}
        </ul>
      </ul>
    </div>
  );
};

export default component<IProps>(Language);
