import { default as erudaOrigin } from 'eruda';
import monitor from 'eruda-monitor';
import features from 'eruda-features';
import timing from 'eruda-timing';
import code from 'eruda-code';
import benchmark from 'eruda-benchmark';
import geolocation from 'eruda-geolocation';
import orientation from 'eruda-orientation';
import touches from 'eruda-touches';

const init = async () => {
  erudaOrigin.init();

  const plugins = [
    monitor,
    features,
    timing,
    code,
    benchmark,
    geolocation,
    orientation,
    touches,
  ];
  plugins.forEach((p) => erudaOrigin.add(p));
};

const eruda = {
  init,
};

export default eruda;
