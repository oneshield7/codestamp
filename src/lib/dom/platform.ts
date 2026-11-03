export type Platform = 'windows' | 'macos' | 'linux';

function detect(agent: string): Platform {
  if (agent.includes('Windows')) {
    return 'windows';
  }
  return agent.includes('Mac OS X') ? 'macos' : 'linux';
}

/** Read from the webview's user agent: WebView2, WKWebView and WebKitGTK each name their OS. */
export const platform: Platform = detect(navigator.userAgent);

export function forPlatform<T>(choices: Record<Platform, T>): T {
  switch (platform) {
    case 'windows':
      return choices.windows;
    case 'macos':
      return choices.macos;
    case 'linux':
      return choices.linux;
  }
}

/** The OS as it reads mid-sentence, as in "the certificates Windows trusts". */
export const systemName = forPlatform({ windows: 'Windows', macos: 'macOS', linux: 'the system' });
