import { FC } from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';
import { cls } from '@shirtiny/utils/lib/style';
import css from './index.module.scss';

interface IProps extends ICommonProps {
  src?: string;
  alt?: string;
}

const Image: FC<IProps> = ({
  src,
  alt,
  className,
  ...rest
}) => {
  return (
    <div
      className={cls(css.imageContainer, className)}
      {...rest}
    >
      <img src={src} alt={alt} />
    </div>
  );
};

export default component<IProps>(Image);
