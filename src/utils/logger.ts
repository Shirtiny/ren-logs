import { ShLogger, LEVELS, css } from '@shirtiny/logger';
import {
  addListener,
  launch,
  stop,
  setDetectDelay,
  removeListener,
} from 'devtools-detector';
import env from './env';

const miku = 'https://i.giphy.com/media/11lxCeKo6cHkJy/giphy.webp';

class CustomLogger extends ShLogger {
  globalState = {
    pre: (...data: any[]) => {
      this.customFormat(
        LEVELS.group,
        [
          {
            str: ' pre ',
            style: css`
              color: #c8c2bc;
            `,
          },
        ],
        ...data,
      );
    },
    action: (...data: any[]) => {
      this.customFormat(
        LEVELS.group,
        [
          {
            str: ' action ',
            style: css`
              color: #a084cf;
            `,
          },
        ],
        ...data,
      );
    },
    next: (...data: any[]) => {
      this.customFormat(
        LEVELS.group,
        [
          {
            str: ' next ',
            style: css`
              color: #a0d995;
            `,
          },
        ],
        ...data,
      );
    },
    changes: (...data: any[]) => {
      this.customFormat(
        LEVELS.group,
        [
          {
            str: ' changes ',
            style: css`
              color: #ecb390;
            `,
          },
        ],
        ...data,
      );
    },
  };
}

const logger = new CustomLogger({
  level: env.isDev() ? LEVELS.debug : LEVELS.log,
});

export const logVersion = async () => {
  logger.group('info: ', async () => {
    env.isDev() && logger.log('env: ', import.meta.env);
    logger.log('log options:', logger.getLoggerOption());
  });

  try {
    const res = await fetch('/version.json');
    const versionInfo: any = await res.json();
    versionInfo &&
      (await logger.unionVersion(
        versionInfo.package.name,
        'main',
        versionInfo.git.abbreviatedSha,
        { src: miku },
      ));
  } catch (e) {
    logger.error(e);
  }
};

let flag = false;

const run = (_isOpen: boolean, detail: any) => {
  logger.log('devtools opened: ', detail);
  if (flag) return;
  logVersion();
  flag = true;
  stop();
  removeListener(run);
};

setDetectDelay(10000);

addListener(run);

launch();

export default logger;
