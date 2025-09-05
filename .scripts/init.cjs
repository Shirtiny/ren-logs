const shell = require('shelljs');
const ci = require('ci-info');
const husky = require('husky');
const dotenv = require('dotenv');

const { GITHUB_TOKEN } = process.env;

//检查控制台是否以运行`git `开头的命令
if (!shell.which('git')) {
  //在控制台输出内容
  shell.echo('Sorry, this script requires git');
  shell.exit(1);
}

if (ci.isCI) {
  console.log('The name of the CI server is:', ci.name);
  // shell.exec("git config --global user.name shirtiny");
  // shell.exec("git config --global user.email shirtiny@gmail.com");
  // shell.exec(
  //   `git remote set-url origin https://shirtiny:${GITHUB_TOKEN}@github.com/Shirtiny/chat-tan.git `
  // );
  // shell.exec(`git add .`);
  // shell.exec(`git reset --hard`);
  // shell.exec(`git checkout main`);
  // shell.exec("git pull");
  // shell.exec("git pull --tags");
} else {
  console.log('This program is not running on a CI server');
  dotenv.config({ path: './.env.local' });
  husky.install();
}

console.log('node version');
shell.exec('pnpm node --version');
shell.exec('pnpm node .scripts/version.cjs');
