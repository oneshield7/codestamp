import { forPlatform } from '../dom/platform';

/** Followed by ", then …" where it is shown. */
export const INSTALL_GIT = forPlatform({
  windows: 'Install Git for Windows from git-scm.com',
  macos: 'Install Git from git-scm.com',
  linux: 'Install Git with your package manager (the git package)',
});
