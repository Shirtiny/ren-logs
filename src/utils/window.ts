import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

export async function showWindow() {
  const appWindow = getCurrentWebviewWindow();
  await appWindow.show();
  await appWindow.unminimize();
  await appWindow.setFocus();
}
