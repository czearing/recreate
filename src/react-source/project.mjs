import fs from 'node:fs';
import path from 'node:path';
import { buildColorResolver } from './color-evidence.mjs';
import { splitCssByComponent } from './css-split.mjs';
import { generateReactComponents } from './html-to-jsx.mjs';

const write = (file, content) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, content);
};

function componentSource(definition) {
  const props = (definition.props || []).map((prop) =>
    typeof prop === 'string' ? { name: prop, type: 'string' } : prop);
  const imports = [...new Set(definition.imports)]
    .map((name) => `import { ${name} } from './${name}';`)
    .join('\n');
  const type = props.length
    ? `export interface ${definition.name}Props {\n${props
      .map(({ name, type: propType }) => `  readonly ${name}: ${propType};`).join('\n')}\n}\n\n`
    : '';
  const propNames = props.map(({ name }) => name);
  const parameters = props.length
    ? `{ ${propNames.join(', ')} }: ${definition.name}Props`
    : '';
  const cssImport = definition.cssFile ? `import './${definition.name}.css';\n` : '';
  return `${cssImport}${imports}${cssImport || imports ? '\n' : ''}${type}export function ${definition.name}(${parameters}) {
  return (
${definition.jsx}
  );
}
`;
}

function copyDirectory(source, target) {
  if (!fs.existsSync(source)) return;
  fs.mkdirSync(target, { recursive: true });
  for (const entry of fs.readdirSync(source, { withFileTypes: true })) {
    const from = path.join(source, entry.name);
    const to = path.join(target, entry.name);
    if (entry.isDirectory()) copyDirectory(from, to);
    else fs.copyFileSync(from, to);
  }
}

export function buildReactProject({ specDir, outDir, maxNodes = 20 }) {
  const spec = JSON.parse(fs.readFileSync(path.join(specDir, 'spec.json'), 'utf8'));
  const homeFile = path.join(specDir, spec.home?.html || '');
  if (!fs.existsSync(homeFile)) throw new Error('The specification has no captured home HTML.');
  fs.rmSync(outDir, { recursive: true, force: true });
  const cssFiles = [
    ...fs.readdirSync(path.join(specDir, 'stylesheets')).filter((file) => file.endsWith('.css'))
      .map((file) => path.join(specDir, 'stylesheets', file)),
    spec.home?.stylesheet ? path.join(specDir, spec.home.stylesheet) : '',
  ].filter((file) => file && fs.existsSync(file));
  const cssSources = cssFiles.map((file) => fs.readFileSync(file, 'utf8'));
  const generated = generateReactComponents(
    fs.readFileSync(homeFile, 'utf8'),
    { maxNodes, resolveColor: buildColorResolver(cssSources) },
  );
  write(path.join(outDir, 'src', 'App.tsx'), `${generated.appImports
    .map((name) => `import { ${name} } from './components/${name}';`).join('\n')}

export function App() {
  return (
    <>
${generated.appChildren.join('\n')}
    </>
  );
}
`);
  const css = splitCssByComponent(
    cssSources,
    generated.definitions,
  );
  write(path.join(outDir, 'src', 'styles', 'shared.css'), css.sharedCss);
  for (const definition of generated.definitions) {
    const componentCss = css.componentCss.get(definition.name);
    definition.cssFile = Boolean(componentCss);
    write(
      path.join(outDir, 'src', 'components', `${definition.name}.tsx`),
      componentSource(definition),
    );
    if (componentCss) {
      write(path.join(outDir, 'src', 'components', `${definition.name}.css`), componentCss);
    }
  }
  for (const asset of generated.assets) {
    write(path.join(outDir, 'public', 'assets', asset.file), asset.source);
  }
  write(path.join(outDir, 'src', 'main.tsx'), `import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles/shared.css';
import { App } from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
`);
  write(path.join(outDir, 'index.html'), `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>${spec.home?.title || 'Generated site'}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
`);
  write(path.join(outDir, 'package.json'), `${JSON.stringify({
    name: 'site-spec-react-output',
    private: true,
    version: '0.0.0',
    type: 'module',
    scripts: { dev: 'vite', build: 'tsc -b && vite build' },
    dependencies: { react: '^19.1.0', 'react-dom': '^19.1.0' },
    devDependencies: {
      '@types/react': '^19.1.0',
      '@types/react-dom': '^19.1.0',
      typescript: '^5.8.3',
      vite: '^7.0.0',
    },
  }, null, 2)}\n`);
  write(path.join(outDir, 'tsconfig.json'), `${JSON.stringify({
    compilerOptions: {
      target: 'ES2022',
      useDefineForClassFields: true,
      lib: ['ES2022', 'DOM', 'DOM.Iterable'],
      allowJs: false,
      skipLibCheck: true,
      esModuleInterop: true,
      allowSyntheticDefaultImports: true,
      strict: true,
      forceConsistentCasingInFileNames: true,
      module: 'ESNext',
      moduleResolution: 'Bundler',
      resolveJsonModule: true,
      isolatedModules: true,
      noEmit: true,
      jsx: 'react-jsx',
    },
    include: ['src'],
  }, null, 2)}\n`);
  copyDirectory(path.join(specDir, 'snapshot-assets'), path.join(outDir, 'public', 'snapshot-assets'));
  return {
    outDir,
    componentCount: generated.definitions.length,
    assetCount: generated.assets.length,
    maxComponentLines: Math.max(...generated.definitions.map((definition) =>
      componentSource(definition).split('\n').length)),
    totalCssRuleCount: css.totalRuleCount,
    keptCssRuleCount: css.keptRuleCount,
  };
}
