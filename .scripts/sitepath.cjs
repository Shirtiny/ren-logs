const fs = require('fs');
const path = require('path');

const distDir = path.resolve(__dirname, '../dist');
const outFilePath = path.resolve(distDir, 'sitepath.txt');

function getFiles(dir, fileList = []) {
  try {
    const files = fs.readdirSync(dir);

    files.forEach(file => {
      const filePath = path.join(dir, file);
      try {
        const stat = fs.statSync(filePath);

        if (stat.isDirectory()) {
          getFiles(filePath, fileList);
        } else {
          fileList.push(filePath);
        }
      } catch (statError) {
        console.warn(`Warning: Could not stat file ${filePath}: ${statError.message}`);
      }
    });
  } catch (readDirError) {
    console.error(`Error reading directory ${dir}: ${readDirError.message}`);
  }

  return fileList;
}

function generateSitemap() {
  const args = process.argv.slice(2);
  let base = '';

  // 解析命令行参数
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith('--base=')) {
      base = args[i].substring('--base='.length);
      break;
    }
  }

  if (!base) {
    console.error('Usage: node .scripts/sitemap.cjs --base=<your_base_url>');
    process.exit(1);
  }

  // 确保base URL没有末尾斜杠
  base = base.endsWith('/') ? base.slice(0, -1) : base;

  // 检查dist目录是否存在
  if (!fs.existsSync(distDir)) {
    console.error(`Error: The 'dist' directory does not exist at ${distDir}. Please build your project first.`);
    process.exit(1);
  }

  let files = [];
  try {
    files = getFiles(distDir);
  } catch (error) {
    console.error(`Error getting files from 'dist' directory: ${error.message}`);
    process.exit(1);
  }

  if (files.length === 0) {
    console.warn(`No files found in 'dist' directory at ${distDir}. Sitemap will be empty.`);
  }

  const sitemapEntries = files.map(file => {
    const relativePath = path.relative(distDir, file).replace(/\\/g, '/'); // 统一路径分隔符
    // 确保拼接时不会出现双斜杠，除非relativePath为空（即distDir本身）
    return `${base}/${relativePath}`;
  });

  try {
    fs.writeFileSync(outFilePath, sitemapEntries.join('\n'));
    console.log(`Sitemap generated at: ${outFilePath}`);
  } catch (writeError) {
    console.error(`Error writing sitemap to ${outFilePath}: ${writeError.message}`);
    process.exit(1);
  }
}

generateSitemap();
