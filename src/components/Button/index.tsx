import { ButtonHTMLAttributes, FC } from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';
import css from './index.module.scss';
import clsx from 'clsx';

interface IProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    ICommonProps {}

const Button: FC<IProps> = ({ className, children, ...rest }) => {
  return (
    <button className={clsx('ui-btn', css.button, className)} {...rest}>
      {children}
    </button>
  );
};

export default component<IProps>(Button);
