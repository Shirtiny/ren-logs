const fs = require('fs');
const path = require('path');

// @rm -Rf dist .vercel/output
// @rm -Rf ./vite.config.js ./vite.config.d.ts ./*.tsbuildinfo

const filesToDelete = ['./vite.config.js', './vite.config.d.ts'];

const dirsToDelete = ['dist', '.vercel/output'];

function deletePath(targetPath) {
  if (fs.existsSync(targetPath)) {
    try {
      fs.rmSync(targetPath, { recursive: true, force: true });
      console.log(`delete: ${targetPath}`);
    } catch (error) {
      console.error(`delete ${targetPath} failed: ${error.message}`);
    }
  } else {
    console.log(`path skip: ${targetPath}`);
  }
}

// 删除指定文件和目录
filesToDelete.forEach(deletePath);
dirsToDelete.forEach(deletePath);

// 删除 .tsbuildinfo 文件
const currentDir = process.cwd();
fs.readdirSync(currentDir).forEach((file) => {
  if (file.endsWith('.tsbuildinfo')) {
    const filePath = path.join(currentDir, file);
    deletePath(filePath);
  }
});

console.log('clean complete.');
