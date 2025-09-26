const fs = require('fs');
const path = require('path');

// Read the fontMetricsData.js file
const fontMetricsPath = path.join(__dirname, '../KaTeX', 'src', 'fontMetricsData.js');
const fontMetricsContent = fs.readFileSync(fontMetricsPath, 'utf8');

// Extract the JavaScript object from the file
// The file exports a default object, so we need to extract it
const objectMatch = fontMetricsContent.match(/export default\s*({[\s\S]*});/);
if (!objectMatch) {
    console.error('Could not find the JavaScript object in the file');
    process.exit(1);
}

// Use eval to parse the JavaScript object (this is safe since we control the input)
const fontMetricsData = eval('(' + objectMatch[1] + ')');

// Convert to JSON with proper formatting
const jsonData = JSON.stringify(fontMetricsData, null, 2);

const outputDir = path.join(__dirname, '../crates', 'katex', 'data');

// Write to JSON file
if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
}
const jsonPath = path.join(outputDir, 'font_metrics_data.json');
fs.writeFileSync(jsonPath, jsonData, 'utf8');

console.log(`Successfully converted to JSON: ${jsonPath}`);
console.log(`File size: ${jsonData.length} characters`);