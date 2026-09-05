// Run after building the pinned submodule: node xtask/tests/generate-upstream-fixtures.mjs
import {createRequire} from 'node:module';
import {readFileSync, writeFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
const require = createRequire(import.meta.url);
const katex = require('../../KaTeX/dist/katex.js');
const revision = execFileSync('git', ['-C', 'KaTeX', 'rev-parse', 'HEAD'], {encoding: 'utf8'}).trim();
const stamp = readFileSync(new URL('../../KaTeX/dist/.katex-rs-revision', import.meta.url), 'utf8');
if (stamp !== revision) throw new Error('Stale KaTeX dist; run cargo xtask build-katex first');
const expressions = [
  String.raw`x^{\dfrac{1}{2}}`, String.raw`x^{\tfrac{1}{2}}`,
  String.raw`x^{\cfrac{1}{2}}`, String.raw`x^{\dbinom{1}{2}}`,
  String.raw`\genfrac{(}{)}{0.04em}{2}{a}{b}`,
  String.raw`\mathbf{\scriptstyle x}`, String.raw`\mathbb{\displaystyle R}`,
  String.raw`\mathbf{\hbox{$x$}}`, String.raw`\mathbf{\colorbox{red}{$x$}}`,
  String.raw`\textbf{$x$}`, String.raw`\mathbf{\begin{matrix}x\end{matrix}}`,
  String.raw`\hat{\mathbb{R}}`, String.raw`\hat{\mathbf{x}}`,
  String.raw`\mathnormal{ff}`, String.raw`\sqrt\imath`, String.raw`\mathop\int`,
  String.raw`\bigl{(}x\bigr{)}`, String.raw`\left[\rule{1em}{4em}\right]`,
  String.raw`\not=`, String.raw`\text{a·b}`, String.raw`a·b`,
  String.raw`\text{♥️}`, String.raw`\char"10FFFF`,
  String.raw`\smash{gj}`, String.raw`\smash[t]{gj}`, String.raw`\smash[b]{gj}`,
  String.raw`a\hphantom{\kern-1em}b`, String.raw`\text{\sout{abc}}`,
  String.raw`\sout{x}`, String.raw`\vcenter{x}`, String.raw`\mathrel{\vcenter{x}}`,
  String.raw`\overbracket{a+b}^{n}`, String.raw`\underbracket{a+b}_{n}`,
  String.raw`\overbrace{x+y}`, String.raw`\underbrace{x+y}`,
  String.raw`\begin{aligned}a&=b\\c&=d\end{aligned}`,
  String.raw`\begin{gathered}a\\b\end{gathered}`,
  String.raw`\htmlData{ a= x=y ,b=one{,}two}{x}`,
  String.raw`\fcolorbox{red}{blue}{x}`, String.raw`\color{#1234}x`,
  String.raw`\begin{alignedat}{1}x&=y\end{alignedat}`,
  String.raw`\sqrt[3]{x}+\overline{x}+\underline{y}+\mathllap{z}`,
  String.raw`x\\y\tag{1}`, String.raw`\vec{x}+\Huge y`,
];
// MathML regressions found by comparing the screenshot dataset to upstream.
const mathmlExpressions = [
  String.raw`a\,b\:c\;d\!e\quad f`,
  String.raw`\mkern1mu a\mkern-1mu b\mkern-4mu c\mkern-5mu d`,
  String.raw`\limsup_{x\to\infty}x+\operatorname{a\,b}x`,
  String.raw`a\neq b\notin c+\mathrel{x}+\mathord{+}`,
  String.raw`\boldsymbol{+}\mathbin{\mathbf{x}}\mathopen{\mathbf{y}}`,
  String.raw`\left(x\middle|y\middle\vert z\right)`,
  String.raw`\begin{array}{|rl:c||}1&2&3\\\hline a&b&c\end{array}`,
  String.raw`\begin{array}{c|c}a&b\end{array}`,
  String.raw`\begin{array}{cc|}a&b\end{array}`,
  String.raw`\begin{array}{c||c:c}a&b&c\end{array}`,
  String.raw`\begin{CD} A @<a<< B @>>b> C \\ @| @AcAA @VVdV \\ D @= E @>>> F \end{CD}`,
  String.raw`\xrightarrow[ab]{ABC}+\xleftarrow{x}+\xrightleftarrows{y}`,
  String.raw`\textbf{a b}\texttt{a b}\textit{a b}\textsf{a b}`,
  String.raw`\textsf{a \textbf{b} \textit{c}}`,
  String.raw`\text{\it a b}\text{\tt a b}`,
];
const cases = [...expressions, ...mathmlExpressions].flatMap(expression => [false, true].flatMap(displayMode => (mathmlExpressions.includes(expression) ? ['mathml'] : ['html', 'mathml']).map(output => {
  if (!displayMode && (expression.includes('\\tag') || expression.includes('\\begin{CD}'))) return null;
  const settings = {output, strict: 'ignore', trust: true, displayMode};
  return {expression, output, displayMode, expected: katex.renderToString(expression, settings)};
}))).filter(Boolean);
writeFileSync(new URL('../../crates/katex/tests/fixtures/upstream.json', import.meta.url),
  JSON.stringify({revision, version: katex.version, cases}, null, 2) + '\n');
console.log(`Generated ${cases.length} cases from ${revision} (${katex.version})`);
