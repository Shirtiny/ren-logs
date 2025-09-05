import {
  memo,
  RefObject,
  useCallback,
  type ChangeEvent,
  type InputHTMLAttributes,
} from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';
import css from './index.module.scss';
import clsx from 'clsx';

interface IProps extends InputHTMLAttributes<HTMLInputElement>, ICommonProps {
  ref?: RefObject<HTMLInputElement>;
  name?: string;
  /**
   * @deprecated input的最小高度 建议使用className控制
   */
  height?: number;
  maxLength?: number;

  onChange?(e: ChangeEvent<HTMLInputElement>): void;
}

const Input = ({
  ref,
  className,
  style,
  name,
  autoComplete = 'off',
  maxLength = 99,
  disabled,
  readOnly,
  height,
  onChange,
  ...rest
}: IProps) => {
  const handleChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => {
      onChange && onChange(e);
    },
    [onChange],
  );

  return (
    <input
      ref={ref}
      className={clsx(
        css.input,
        className,

        (readOnly || disabled) && css.disabled,
      )}
      style={style}
      name={name}
      disabled={disabled}
      readOnly={readOnly}
      autoComplete={autoComplete}
      maxLength={maxLength}
      onChange={handleChange}
      {...rest}
    />
  );
};

export default component<IProps>(memo(Input), {});
