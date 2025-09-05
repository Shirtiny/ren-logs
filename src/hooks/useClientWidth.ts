import { useSyncExternalStore } from "react";

export default function useClientWidth() {
  const clientWidth = useSyncExternalStore(subscribe, getSnapshot);
  return clientWidth;
}

function getSnapshot() {
  return window.document.documentElement.clientWidth;
}

function subscribe(onStoreChange: () => void) {
  window.addEventListener("resize", onStoreChange);

  return () => {
    window.removeEventListener("resize", onStoreChange);
  };
}
