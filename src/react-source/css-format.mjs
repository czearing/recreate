export function formatCss(source) {
  let output = '';
  let indent = 0;
  let quote = '';
  let comment = false;
  const writeIndent = () => '  '.repeat(indent);

  for (let index = 0; index < source.length; index += 1) {
    const char = source[index];
    const next = source[index + 1];
    if (comment) {
      output += char;
      if (char === '*' && next === '/') {
        output += next;
        index += 1;
        comment = false;
      }
      continue;
    }
    if (!quote && char === '/' && next === '*') {
      comment = true;
      output += '/*';
      index += 1;
      continue;
    }
    if (quote) {
      output += char;
      if (char === quote && source[index - 1] !== '\\') quote = '';
      continue;
    }
    if (char === '"' || char === "'") {
      quote = char;
      output += char;
      continue;
    }
    if (char === '{') {
      output = `${output.trimEnd()} {\n`;
      indent += 1;
      output += writeIndent();
      continue;
    }
    if (char === '}') {
      indent = Math.max(0, indent - 1);
      output = `${output.trimEnd()}\n${writeIndent()}}\n${writeIndent()}`;
      continue;
    }
    if (char === ';') {
      output = `${output.trimEnd()};\n${writeIndent()}`;
      continue;
    }
    if (/\s/.test(char)) {
      if (!output.endsWith(' ') && !output.endsWith('\n')) output += ' ';
      continue;
    }
    output += char;
  }
  return `${output.trim()}\n`;
}
