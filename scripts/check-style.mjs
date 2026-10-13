import ts from 'typescript';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

// Parse production TSX so comments and test assertions are not mistaken for styles.
// Scan string/template tokens (including conditional class expressions).
const errors = [];
function checkTokens(text, file) {
  for (const token of text.split(/\s+/)) {
    const utility = token.split(/:(?![^\[]*\])/).at(-1);
    if (/^(rounded|shadow)(?:-|$)/.test(utility) && !/^(rounded|shadow)-none$/.test(utility)) {
      errors.push(`${file}: disallowed surface utility ${token}`);
    }
  }
}
function visitDirectory(dir) {
  for (const item of readdirSync(dir, { withFileTypes: true })) {
    if (item.name === 'test' || item.name === '__tests__') continue;
    const file = join(dir, item.name);
    if (item.isDirectory()) { visitDirectory(file); continue; }
    if (/\.(test|spec)\./.test(file)) continue;
    const text = readFileSync(file, 'utf8');
    if (file.endsWith('.tsx')) {
      const ast = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
      function visit(node) {
        if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)
            || node.kind === ts.SyntaxKind.TemplateHead || node.kind === ts.SyntaxKind.TemplateMiddle
            || node.kind === ts.SyntaxKind.TemplateTail) checkTokens(node.text, file);
        ts.forEachChild(node, visit);
      }
      visit(ast);
    } else if (file.endsWith('.css')) {
      const css = text.replace(/\/\*[\s\S]*?\*\//g, '');
      for (const [, property, value] of css.matchAll(/(border(?:-[\w]+)?-radius|box-shadow)\s*:\s*([^;}]+)/g)) {
        if (!/^(?:0(?:px|rem|em|%)?|none)(?:\s+!important)?\s*$/.test(value.trim())) {
          errors.push(`${file}: ${property}: ${value}`);
        }
      }
    }
  }
}
visitDirectory('src');
if (errors.length) {
  console.error(errors.join('\n'));
  process.exitCode = 1;
} else console.log('Production style guard passed.');
