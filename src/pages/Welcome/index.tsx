import { useLayoutEffect } from 'react';
import Button from '@/components/Button';
import { showWindow } from '@/utils/window';

const Component = () => {
  useLayoutEffect(() => {
    // showWindow();
  }, []);

  return (
    <div className="page page-welcome">
      <div></div>
    </div>
  );
};

export { Component };
