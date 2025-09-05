import { FC } from 'react';

const IconWrap: FC<any> = ({ Icon, ...rest }: { Icon: any }) => {
  return <Icon {...rest} />;
};

export default IconWrap;
