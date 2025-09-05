function vhProperty(win: Window) {
  const setVhProperty = () => {
    win.document.documentElement.style.setProperty(
      '--vh',
      `${window.innerHeight}px`,
    );
  };

  setVhProperty();

  function onResize() {
    setVhProperty();
  }

  // We listen to the resize event
  win.addEventListener('resize', onResize);

  function clean() {
    win.document.documentElement.style.removeProperty('--vh');
    win.removeEventListener('resize', onResize);
  }

  return clean;
}

const layout = {
  vhProperty,
};

export default layout;
