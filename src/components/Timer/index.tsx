import { useEffect, useState, type FC } from 'react';
import type { ICommonProps } from '@/types';
import component from '@/hoc/component';
import clsx from 'clsx';

interface IProps extends ICommonProps {
  start?: number;
  end?: number;
}

const Timer: FC<IProps> = ({
  className,
  style = {},
  start = 0,
  end = 0,
  step = 1,
  onStop,
  ...rest
}) => {
  const [second, setSecond] = useState(start);

  useEffect(() => {
    if (end === start) return;

    const direction = Math.sign(end - start);

    const timer = setInterval(() => {
      setSecond((second) => {
        const next = second + direction * Math.abs(step);
        const stopFlag = (end - next) * direction <= 0;
        if (stopFlag) {
          clearInterval(timer);
          onStop?.();
        }
        return next;
      });
    }, 1000);

    return () => {
      clearInterval(timer);
    };
  }, [end, start, step, onStop]);

  return (
    <span
      className={clsx(className)}
      style={{
        ...style,
      }}
      {...rest}
    >
      {second}
    </span>
  );
};

export default component<IProps>(Timer);
