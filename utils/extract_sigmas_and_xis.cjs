const fs = require('fs');
const path = require('path');

// Read the fontMetrics.js file
const fontMetricsPath = path.join(__dirname, '../KaTeX', 'src', 'fontMetrics.js');
const fontMetricsContent = fs.readFileSync(fontMetricsPath, 'utf8');

// Extract the sigmasAndXis object
const sigmasMatch = fontMetricsContent.match(/const sigmasAndXis = ({[\s\S]*?});/);
if (!sigmasMatch) {
    console.error('Could not find sigmasAndXis in the file');
    process.exit(1);
}

// Use eval to parse the JavaScript object
const sigmasAndXis = eval('(' + sigmasMatch[1] + ')');

// Extract documentation from JavaScript comments
const lines = fontMetricsContent.split('\n');
const fieldDocs = {};

let currentDoc = [];
let inSigmasBlock = false;

for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();

    if (line.includes('const sigmasAndXis = {')) {
        inSigmasBlock = true;
        continue;
    }

    if (inSigmasBlock && line === '};') {
        break;
    }

    if (inSigmasBlock) {
        // Check for multi-line comments before fields
        if (line.startsWith('//') && !line.includes(':')) {
            currentDoc.push(line.substring(2).trim());
        } else if (line.includes(':')) {
            // Extract field name and inline comment
            const fieldMatch = line.match(/^(\w+):/);
            if (fieldMatch) {
                const fieldName = fieldMatch[1];
                const commentMatch = line.match(/\/\/ (.+)$/);
                if (commentMatch) {
                    const comment = commentMatch[1];
                    if (currentDoc.length > 0) {
                        // Combine multi-line docs with inline comment
                        fieldDocs[fieldName] = [...currentDoc, comment].join(' ');
                        currentDoc = [];
                    } else {
                        fieldDocs[fieldName] = comment;
                    }
                } else if (currentDoc.length > 0) {
                    // Use multi-line docs if no inline comment
                    fieldDocs[fieldName] = currentDoc.join(' ');
                    currentDoc = [];
                }
            }
        }
    }
}

// Add default documentation for fields without comments
const defaultDocs = {
    sqrtRuleThickness: "The \\sqrt rule width is taken from the height of the surd character. Since we use the same font at all sizes, this thickness doesn't scale.",
    ptPerEm: "This value determines how large a pt is, for metrics which are defined in terms of pts. This value is also used in katex.scss; if you change it make sure the values match.",
    doubleRuleSep: "The space between adjacent `|` columns in an array definition. From `\\showthe\\doublerulesep` in LaTeX. Equals 2.0 / ptPerEm.",
    arrayRuleWidth: "The width of separator lines in {array} environments. From `\\showthe\\arrayrulewidth` in LaTeX. Equals 0.4 / ptPerEm.",
    fboxsep: "Two values from LaTeX source2e: 3 pt / ptPerEm",
    fboxrule: "Two values from LaTeX source2e: 0.4 pt / ptPerEm"
};

Object.keys(defaultDocs).forEach(key => {
    if (!fieldDocs[key]) {
        fieldDocs[key] = defaultDocs[key];
    }
});

// Create the JSON structure with data and documentation
const jsonData = {
    sigmasAndXis,
    fieldDocs
};

// Convert to JSON with proper formatting
const jsonString = JSON.stringify(jsonData, null, 2);

// Write to JSON file
const outputDir = path.join(__dirname, '../crates', 'katex', 'data');
if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
}
const jsonPath = path.join(outputDir, 'sigmas_and_xis.json');
fs.writeFileSync(jsonPath, jsonString, 'utf8');

console.log(`Successfully converted to JSON: ${jsonPath}`);
console.log(`File size: ${jsonString.length} characters`);