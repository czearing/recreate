import postcss from 'postcss';

const classNames = (selector) => [
  ...String(selector || '').matchAll(/\.(-?[_a-zA-Z]+[_a-zA-Z0-9-]*)/g),
].map((match) => match[1]);

function inlineColor(node) {
  const style = node.attrs?.find(({ name }) => name === 'style')?.value || '';
  return style.match(/(?:^|;)\s*color\s*:\s*([^;]+)/i)?.[1]?.trim();
}

export function buildColorResolver(cssSources) {
  const variables = new Map();
  const classColors = new Map();
  for (const source of cssSources) {
    const root = postcss.parse(source);
    root.walkDecls((declaration) => {
      if (declaration.prop.startsWith('--')) variables.set(declaration.prop, declaration.value);
      if (declaration.prop !== 'color' || declaration.parent?.type !== 'rule') return;
      for (const className of classNames(declaration.parent.selector)) {
        classColors.set(className, declaration.value);
      }
    });
  }
  const resolve = (value, depth = 0) => {
    if (!value || depth > 8) return undefined;
    const variable = String(value).match(/^var\((--[^,\s)]+)(?:,\s*([^)]+))?\)$/);
    if (!variable) return value;
    return resolve(variables.get(variable[1]) || variable[2], depth + 1);
  };
  return (node) => {
    let current = node;
    while (current) {
      const inline = resolve(inlineColor(current));
      if (inline) return inline;
      const classes = current.attrs?.find(({ name }) => name === 'class')?.value || '';
      for (const className of classes.split(/\s+/).filter(Boolean)) {
        const color = resolve(classColors.get(className));
        if (color) return color;
      }
      current = current.parentNode;
    }
    return '#5d5d5d';
  };
}
