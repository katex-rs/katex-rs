import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function extractSymbols() {
    try {
        // Path to the symbols.js file
        const symbolsPath = path.join(__dirname, '..', 'KaTeX', 'src', 'symbols.js');

        // Read the file as text
        const content = fs.readFileSync(symbolsPath, 'utf8');

        // Split into lines
        const lines = content.split('\n');

        // Filter top-level defineSymbol calls (no leading whitespace)
        const defineSymbolLines = lines.filter(line =>
            line.trim().startsWith('defineSymbol(') && !line.startsWith(' ')
        );

        const regex = /defineSymbol\(\s*([a-zA-Z$_][\w$]*)\s*,\s*([a-zA-Z$_][\w$]*)\s*,\s*([a-zA-Z$_][\w$]*)\s*,\s*(?:"((?:[^"\\]|\\.)*)"|(null|true|false|[a-zA-Z$_][\w$]*))\s*,\s*"((?:[^"\\]|\\.)*)"(?:\s*,\s*([^)]+))?\s*\);/;

        const symbols = [];

        for (const line of defineSymbolLines) {
            const match = line.match(regex);
            if (!match) continue;

            const [
                , mode, font, group,
                replaceQuoted, // group 4
                replaceBare,   // group 5
                name,          // group 6
                acceptUnicodeChar // group 7
            ] = match;

            let processedReplace = null;
            if (typeof replaceQuoted === 'string') {
                processedReplace = replaceQuoted;
            } else if (typeof replaceBare === 'string' && replaceBare !== 'null') {
                processedReplace = replaceBare;
            }

            const processedName = typeof name === 'string' ? name : '';

            let processedAcceptUnicodeChar = false;
            if (typeof acceptUnicodeChar === 'string' && acceptUnicodeChar.trim() === 'true') {
                processedAcceptUnicodeChar = true;
            }

            symbols.push({
                mode,
                font,
                group,
                replace: processedReplace,
                name: processedName,
                acceptUnicodeChar: processedAcceptUnicodeChar
            });
        }

        // Output path
        const outputDir = path.join(__dirname, '../crates', 'katex', 'data');
        const outputPath = path.join(outputDir, 'symbols.json');

        // Write to file
        fs.writeFileSync(outputPath, JSON.stringify(symbols, null, 2), 'utf8');

        console.log(`Extracted ${symbols.length} symbols to ${outputPath}`);
    } catch (error) {
        console.error('Error extracting symbols:', error.message);
        process.exit(1);
    }
}

extractSymbols();
