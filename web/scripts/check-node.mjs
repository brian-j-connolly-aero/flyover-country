// Fail early, and kindly, on a Node.js too old for Vite 7.
const [major, minor] = process.versions.node.split('.').map(Number);
const ok = (major === 20 && minor >= 19) || (major === 22 && minor >= 12) || major >= 23;
if (!ok) {
  console.error(`
  Flyover Country needs Node.js 20.19 or newer (22 LTS recommended).
  This computer has Node.js ${process.versions.node}.

  Windows: install the LTS version from https://nodejs.org, then open a new terminal.
  WSL or Linux: nvm install --lts   (nvm: https://github.com/nvm-sh/nvm)
`);
  process.exit(1);
}
