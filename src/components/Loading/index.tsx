import type { FC } from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';
import clsx from 'clsx';

import './index.scss';

interface IProps extends ICommonProps {}

const Loading: FC<IProps> = ({ className, style = {}, ...rest }) => {
  return (
    <div
      className={clsx('loading-overlay', className)}
      style={{
        ...style,
      }}
      {...rest}
    >
      {/* https://nzbin.github.io/three-dots/ */}
      <div className="spinner"></div>
    </div>
  );
};

export default component<IProps>(Loading);
